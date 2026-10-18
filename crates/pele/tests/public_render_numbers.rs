//! Native numeric render layer against Pele458322a5 / npm1.6.0.
//! Raw option transport remains bit-exact. Computed NaNs compare as NaNs because
//! floating arithmetic does not promise payload identity; other IEEE bits do.
use pele::compat::{public::*, *};
use serde::Serialize;
use serde_json::{json, Value as J};
use std::sync::{Arc, Mutex};
fn fixture() -> J {
    serde_json::from_str(include_str!("../../../fixtures/pele/render-numbers.json")).unwrap()
}
fn number(j: &J) -> f64 {
    f64::from_bits(u64::from_str_radix(j["bits"].as_str().unwrap(), 16).unwrap())
}
fn bits(n: f64) -> String {
    format!("{:016x}", n.to_bits())
}
fn options(c: &J) -> RenderOptions {
    let mut options: RenderOptions = serde_json::from_value(c["legacyOptions"].clone()).unwrap();
    for (key, n) in c["numeric"].as_object().unwrap() {
        let value = Some(number(n));
        match key.as_str() {
            "fontSize" => options.render.font_size = value,
            "maxWidth" => options.render.max_width = value,
            "directionBreakpoint" => options.render.direction_breakpoint = value,
            "padding" => options.render.padding = value,
            "now" => options.render.now = value,
            _ => panic!("numeric field"),
        }
    }
    options
}
#[derive(Default)]
struct Host {
    calls: Vec<J>,
    now: Option<f64>,
    fail: bool,
}
impl Host {
    fn new(c: &J) -> Self {
        Self {
            now: c.get("hostNow").map(number),
            fail: c["hostThrows"] == true,
            ..Default::default()
        }
    }
}
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        let width = r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 };
        self.calls.push(json!([
            "width",
            r.text.encode_utf16().collect::<Vec<_>>(),
            bits(r.size),
            r.style,
            bits(width)
        ]));
        Ok(width)
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        let now = self.now.unwrap_or(1704067200000.);
        self.calls.push(json!([
            "clock",
            if self.fail { "throw".into() } else { bits(now) }
        ]));
        if self.fail {
            Err(PeleError::new(
                "render-numbers host clock stopped",
                ErrorCode::Semantic,
            ))
        } else {
            Ok(now)
        }
    }
}
fn assert_number(actual: f64, expected: &J, context: &str) {
    let raw = u64::from_str_radix(expected.as_str().unwrap(), 16).unwrap();
    let expected = f64::from_bits(raw);
    if expected.is_nan() {
        assert!(
            actual.is_nan(),
            "{context}: expected computed NaN, actual{}",
            bits(actual)
        )
    } else {
        assert_eq!(actual.to_bits(), raw, "{context}")
    }
}
fn assert_trace(actual: &[J], expected: &J, id: &str) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len(), "callback count {id}");
    for (index, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(a[0], e[0], "callback kind {id}/{index}");
        if a[0] == "clock" {
            assert_eq!(a, e, "clock {id}/{index}")
        } else {
            assert_eq!(a[1], e[1], "UTF16 request {id}/{index}");
            assert_eq!(a[3], e[3], "style {id}/{index}");
            for position in [2, 4] {
                let raw = u64::from_str_radix(a[position].as_str().unwrap(), 16).unwrap();
                assert_number(
                    f64::from_bits(raw),
                    &e[position],
                    &format!("callback number {id}/{index}/{position}"),
                )
            }
        }
    }
}
fn assert_render(r: Result<RenderResult, PeleError>, expected: &J, id: &str) {
    match r {
        Ok(r) => {
            assert_eq!(
                serde_json::to_value(&r).unwrap(),
                expected["result"],
                "complete returned SVG {id}"
            );
            assert_number(
                r.rendered.width,
                &expected["widthBits"],
                &format!("width {id}"),
            );
            assert_number(
                r.rendered.height,
                &expected["heightBits"],
                &format!("height {id}"),
            );
        }
        Err(error) => {
            let e = &expected["error"];
            if e["name"] == "PeleError" {
                assert_eq!(
                    serde_json::to_value(error).unwrap(),
                    *e,
                    "render error {id}"
                )
            } else {
                assert_eq!(e["name"], "Error");
                assert_eq!(
                    error.message,
                    e["message"].as_str().unwrap(),
                    "host error {id}"
                );
                assert_eq!(error.code, ErrorCode::Semantic)
            }
        }
    }
}
fn canonical(g: &ModelGraph) -> J {
    use std::collections::{HashMap, VecDeque};
    fn atom(
        g: &ModelGraph,
        a: &Atom,
        ids: &mut HashMap<u32, usize>,
        queue: &mut VecDeque<u32>,
    ) -> J {
        match a {
            Atom::Undefined => serde_json::json!(["undefined"]),
            Atom::Null => serde_json::json!(["null"]),
            Atom::Bool(v) => serde_json::json!(["bool", v]),
            Atom::Number(v) => serde_json::json!(["number", format!("{:016x}", v.to_bits())]),
            Atom::String(i) => serde_json::json!(["string", g.strings[*i as usize].0]),
            Atom::Ref(id) => {
                let next = ids.len();
                let index = *ids.entry(*id).or_insert_with(|| {
                    queue.push_back(*id);
                    next
                });
                serde_json::json!(["ref", index])
            }
            Atom::BuiltinFunction(id) => serde_json::json!(["builtin", id]),
            Atom::HostReference(id) => serde_json::json!(["host", id]),
        }
    }
    let mut ids = HashMap::new();
    let mut queue = VecDeque::new();
    let root = atom(g, &g.root, &mut ids, &mut queue);
    let mut nodes = Vec::new();
    while let Some(id) = queue.pop_front() {
        let n = &g.nodes[id as usize];
        let mut out = serde_json::json!({"kind":match n{Node::Error{..}=>"error",Node::Instance{..}=>"instance",Node::Object{..}=>"object",Node::Array{..}=>"array",Node::Map{..}=>"map",Node::Set{..}=>"set",Node::Date{..}=>"date"}});
        match n {
            Node::Error { error_type, .. } => {
                out["errorType"] = serde_json::to_value(error_type).unwrap();
            }
            Node::Instance { prototype, .. } => out["prototype"] = prototype.0.into(),
            Node::Array { length, .. } => out["length"] = (*length).into(),
            Node::Map { entries, .. } => {
                out["entries"] = entries
                    .iter()
                    .map(|(k, v)| {
                        serde_json::json!([
                            atom(g, k, &mut ids, &mut queue),
                            atom(g, v, &mut ids, &mut queue)
                        ])
                    })
                    .collect()
            }
            Node::Set { values, .. } => {
                out["values"] = values
                    .iter()
                    .map(|v| atom(g, v, &mut ids, &mut queue))
                    .collect()
            }
            Node::Date { milliseconds, .. } => {
                out["milliseconds"] = bits(*milliseconds).into();
            }
            _ => {}
        }
        let mut properties = n.properties().iter().collect::<Vec<_>>();
        let index = |p: &&Property| {
            g.strings[p.key as usize].well_formed().and_then(|s| {
                s.parse::<u32>()
                    .ok()
                    .filter(|n| *n != u32::MAX && n.to_string() == s)
            })
        };
        properties.sort_by(|a, b| match (index(a), index(b)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });
        out["properties"] = properties
            .into_iter()
            .map(|p| {
                serde_json::json!([
                    g.strings[p.key as usize].0,
                    atom(g, &p.value, &mut ids, &mut queue),
                    p.flags
                ])
            })
            .collect();
        nodes.push(out);
    }
    serde_json::json!({"root":root,"nodes":nodes})
}
#[test]
fn bounded_source_document_and_public_rendering_match_pinned_numeric_cases() {
    let f = fixture();
    assert_eq!(f["revision"], UPSTREAM_REVISION);
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    let mut boundaries = vec![];
    assert_eq!(f["cases"].as_array().unwrap().len(), 290);
    for c in f["cases"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        if c.get("boundary").is_some() {
            assert_eq!(c["boundary"], "runtime-null-now-outside-declared-packet");
            assert_eq!(c["values"]["now"]["kind"], "null");
            if c["family"] == "gantt" {
                assert_eq!(c["expected"]["schedule"]["nowBits"], bits(0.));
                assert_eq!(c["expected"]["counts"]["clock"], 0)
            }
            boundaries.push(id.to_owned());
            continue;
        }
        assert!(
            c.get("unsupported").is_none(),
            "owned worker unsupported input {id}"
        );
        assert!(
            c["expected"].get("unsupported").is_none(),
            "harness unsupported input {id}"
        );
        let source = c["source"].as_str().unwrap();
        let options = options(c);
        let expected = &c["expected"];
        let typed = engine.parse(source, &options.parse).unwrap();
        for output in [None, Some(OutputLimit::Undefined)] {
            let mut host = Host::new(c);
            assert_render(
                engine.render_source_bounded(source, &options, output, &mut host),
                expected,
                id,
            );
            assert_trace(&host.calls, &expected["calls"], id);
            let mut host = Host::new(c);
            assert_render(
                engine.render_document_bounded(&typed, &options, output, &mut host),
                expected,
                id,
            );
            assert_trace(&host.calls, &expected["calls"], id);
            let mut doc = public.parse(source, &options.parse).unwrap();
            assert_eq!(
                canonical(&doc.snapshot().unwrap()),
                expected["graphBefore"],
                "public before {id}"
            );
            let mut host = Host::new(c);
            let outcome = doc
                .render_operation_bounded(0, &options, output, &mut host)
                .unwrap();
            assert_eq!(outcome.revision, 1);
            assert_render(
                outcome.result.map_err(|error| match error {
                    PublicError::Pele(e) => e,
                    other => panic!("unexpected native error {id}: {other:?}"),
                }),
                expected,
                id,
            );
            assert_trace(&host.calls, &expected["calls"], id);
            assert_eq!(
                canonical(&outcome.graph),
                expected["graphAfter"],
                "complete scheduling/cache/Date identity {id}"
            );
            let mut stale = Host::new(c);
            assert_eq!(
                doc.render_operation_bounded(0, &options, output, &mut stale)
                    .unwrap_err()
                    .message,
                "Stale public model revision"
            );
            assert!(stale.calls.is_empty());
            if c["hostThrows"] == true && expected.get("error").is_some() {
                let recovered = doc
                    .render_operation_bounded(1, &options, output, &mut Host::default())
                    .unwrap();
                assert!(recovered.result.is_ok(), "clock error recovery {id}");
                assert_eq!(recovered.revision, 2)
            }
        }
    }
    assert_eq!(
        boundaries.len(),
        3,
        "runtime-null-now is a recorded API boundary, not a passing native number|Date vector"
    );
    assert_eq!(f["pairs"].as_array().unwrap().len(), 27);
    assert!(f["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|pair| pair["compared"] == true));
}
#[derive(Serialize)]
struct Model;
#[derive(Clone, Copy)]
enum Mode {
    Small,
    Large,
    ExpandTheme,
}
#[derive(Clone, Debug)]
struct Observation {
    generation: u32,
    lane: &'static str,
    numbers: [Option<u64>; 5],
}
struct Provider {
    generation: u32,
    mode: Mode,
    observations: Arc<Mutex<Vec<Observation>>>,
}
impl Provider {
    fn draw(
        &self,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
        lane: &'static str,
    ) -> Result<Rendered, PeleError> {
        self.observations.lock().unwrap().push(Observation {
            generation: self.generation,
            lane,
            numbers: [
                o.font_size.map(f64::to_bits),
                o.max_width.map(f64::to_bits),
                o.direction_breakpoint.map(f64::to_bits),
                o.padding.map(f64::to_bits),
                o.now.map(f64::to_bits),
            ],
        });
        let first = h.now_millis();
        let second = h.now_millis();
        assert_eq!(
            first.as_ref().map(|n| n.to_bits()),
            second.as_ref().map(|n| n.to_bits())
        );
        first?;
        let svg = match self.mode {
            Mode::Small => format!("<svg><text>{}:{lane}</text></svg>", self.generation),
            Mode::Large => "x".repeat(MAX_PUBLIC_OUTPUT_BYTES + 1),
            Mode::ExpandTheme => {
                let mut svg = format!("<svg style=\"--_wide:{}\">", "a".repeat(128 * 1024));
                for _ in 0..513 {
                    svg.push_str("<rect fill=\"var(--_wide)\"/>")
                }
                svg.push_str("</svg>");
                assert!(svg.len() < MAX_PUBLIC_OUTPUT_BYTES);
                svg
            }
        };
        Ok(Rendered {
            svg,
            width: 1.,
            height: 1.,
            links: vec![],
        })
    }
}
impl DiagramModule for Provider {
    type Model = Model;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("render-numbers-probe"))
    }
    fn parse(&self, _: ParseContext<'_>) -> Result<Model, PeleError> {
        Ok(Model)
    }
    fn render(
        &self,
        _: &Model,
        _: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.draw(o, h, "legacy")
    }
    fn render_with_output_limit(
        &self,
        _: &Model,
        _: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
        _: OutputLimit,
    ) -> Result<Rendered, PeleError> {
        self.draw(o, h, "exact")
    }
}
impl PublicDiagramModule for Provider {
    type PublicState = ModelGraph;
    fn create_public(&self, _: &Model) -> Result<ModelGraph, PeleError> {
        let mut w = GraphWriter::default();
        let property = w.property("renders", Atom::Number(0.));
        let root = w.node(Node::Object {
            properties: vec![property],
        });
        w.finish(Atom::Ref(root))
    }
    fn export_public(&self, s: &ModelGraph) -> Result<ModelGraph, PeleError> {
        Ok(s.clone())
    }
    fn apply_public(&self, s: &mut ModelGraph, g: ModelGraph) -> Result<(), PeleError> {
        *s = g;
        Ok(())
    }
    fn render_public(
        &self,
        s: &mut ModelGraph,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        advance(s);
        self.render(&Model, c, o, h)
    }
    fn render_public_operation_with_output_limit(
        &self,
        s: &mut ModelGraph,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
        limit: OutputLimit,
    ) -> Result<Rendered, PublicError> {
        advance(s);
        self.render_with_output_limit(&Model, c, o, h, limit)
            .map_err(Into::into)
    }
}
fn advance(g: &mut ModelGraph) {
    let Atom::Ref(root) = g.root else { panic!() };
    let Atom::Number(value) = &mut g.nodes[root as usize].properties_mut()[0].value else {
        panic!()
    };
    *value += 1.;
}
fn counter(g: &ModelGraph) -> f64 {
    let Atom::Number(n) = g.field("renders").unwrap().unwrap() else {
        panic!()
    };
    *n
}
fn provider(mode: Mode, log: Arc<Mutex<Vec<Observation>>>, generation: u32) -> Provider {
    Provider {
        generation,
        mode,
        observations: log,
    }
}
#[test]
fn raw_numbers_hooks_and_lazy_clock_keep_transport_bits_retained_ownership_and_partial_outcomes() {
    let log = Arc::new(Mutex::new(vec![]));
    let mut registry = PublicEngine::empty();
    registry.register(provider(Mode::Small, log.clone(), 1));
    let id = DiagramId::extension("render-numbers-probe");
    let mut doc = registry
        .parse_as(&id, "source", &ParseOptions::default())
        .unwrap();
    registry.register(provider(Mode::Small, log.clone(), 9));
    drop(registry);
    let mut options = RenderOptions::default();
    options.render.font_size = Some(f64::from_bits(0xfff8000000000042));
    options.render.max_width = Some(-0.);
    options.render.direction_breakpoint = Some(f64::INFINITY);
    options.render.padding = Some(f64::NEG_INFINITY);
    options.render.now = Some(f64::from_bits(0x7ff8000000000081));
    let wanted = [
        options.render.font_size.map(f64::to_bits),
        options.render.max_width.map(f64::to_bits),
        options.render.direction_breakpoint.map(f64::to_bits),
        options.render.padding.map(f64::to_bits),
        options.render.now.map(f64::to_bits),
    ];
    for (revision, output) in [None, Some(OutputLimit::Undefined)].into_iter().enumerate() {
        let mut host = Host::default();
        let result = doc
            .render_operation_bounded(revision as u64, &options, output, &mut host)
            .unwrap();
        let lane = if output.is_some() { "exact" } else { "legacy" };
        assert_eq!(
            result.result.unwrap().rendered.svg,
            format!("<svg><text>1:{lane}</text></svg>")
        );
        assert_eq!(counter(&result.graph), (revision + 1) as f64);
        assert_eq!(host.calls.len(), 1, "fallback clock cache");
        let observation = log.lock().unwrap().last().unwrap().clone();
        assert_eq!(
            observation.numbers, wanted,
            "raw IEEE settings preserved independently of computed arithmetic"
        );
        assert_eq!(observation.generation, 1);
        assert_eq!(observation.lane, lane)
    }
    let mut failed = Host {
        fail: true,
        ..Default::default()
    };
    let outcome = doc
        .render_operation_bounded(2, &options, None, &mut failed)
        .unwrap();
    assert_eq!(failed.calls.len(), 1);
    assert_eq!(counter(&outcome.graph), 3.);
    assert!(
        matches!(outcome.result,Err(PublicError::Pele(ref e))if e.message=="render-numbers host clock stopped")
    );
    let mut explicit = options.clone();
    explicit.render.now = Some(-0.);
    let mut host = Host {
        fail: true,
        ..Default::default()
    };
    let recovered = doc
        .render_operation_bounded(3, &explicit, None, &mut host)
        .unwrap();
    assert!(recovered.result.is_ok());
    assert!(
        host.calls.is_empty(),
        "finite explicit time suppresses fallback even for -0"
    );
    assert_eq!(counter(&recovered.graph), 4.);
    let mut immutable = Engine::empty();
    immutable.register(provider(Mode::Small, log.clone(), 4));
    let retained = immutable
        .parse_as(&id, "source", &ParseOptions::default())
        .unwrap();
    immutable.register(provider(Mode::Small, log.clone(), 9));
    drop(immutable);
    for output in [None, Some(OutputLimit::Undefined)] {
        let lane = if output.is_some() { "exact" } else { "legacy" };
        let mut host = Host::default();
        let result = Engine::empty()
            .render_document_bounded(&retained, &options, output, &mut host)
            .unwrap();
        assert_eq!(
            result.rendered.svg,
            format!("<svg><text>4:{lane}</text></svg>")
        );
        assert_eq!(host.calls.len(), 1);
        let observation = log.lock().unwrap().last().unwrap().clone();
        assert_eq!(observation.numbers, wanted);
        assert_eq!(observation.generation, 4);
        assert_eq!(observation.lane, lane);
    }
}
#[test]
fn new_bounded_lanes_keep_native_svg_guards_before_theme_and_after_theme_expansion() {
    let log = Arc::new(Mutex::new(vec![]));
    let mut registry = PublicEngine::empty();
    registry.register(provider(Mode::Large, log.clone(), 1));
    let id = DiagramId::extension("render-numbers-probe");
    let mut doc = registry
        .parse_as(&id, "source", &ParseOptions::default())
        .unwrap();
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(0);
    options.render.native_theme = Some(NativeTheme {
        background: "invalid".into(),
        ..Default::default()
    });
    for (revision, output) in [None, Some(OutputLimit::Number(f64::NAN))]
        .into_iter()
        .enumerate()
    {
        let outcome = doc
            .render_operation_bounded(revision as u64, &options, output, &mut Host::default())
            .unwrap();
        assert_eq!(counter(&outcome.graph), (revision + 1) as f64);
        assert!(matches!(outcome.result,Err(PublicError::Pele(ref e))if e.message=="Native render output exceeds byte budget"&&e.code==ErrorCode::Limit&&e.diagram_type.as_deref()==Some("render-numbers-probe")),"native pre-theme output guard wins before invalid theme and semantic output comparison")
    }
    // A short trusted SVG may expand through a repeated provider-defined CSS token.
    // The new lane bounds its resolved returned bytes as well as its input bytes.
    registry.register(provider(Mode::ExpandTheme, log, 1));
    let mut doc = registry
        .parse_as(&id, "source", &ParseOptions::default())
        .unwrap();
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(usize::MAX);
    options.render.native_theme = Some(NativeTheme::default());
    for (revision, output) in [None, Some(OutputLimit::Number(f64::INFINITY))]
        .into_iter()
        .enumerate()
    {
        let outcome = doc
            .render_operation_bounded(revision as u64, &options, output, &mut Host::default())
            .unwrap();
        assert!(
            matches!(outcome.result,Err(PublicError::Pele(ref e))if e.message=="Native render output exceeds byte budget"&&e.code==ErrorCode::Limit),
            "post-theme output guard {revision}"
        );
        assert_eq!(counter(&outcome.graph), (revision + 1) as f64)
    }
}
#[test]
fn legacy_class_dispatch_and_new_source_byte_guard_preserve_error_order() {
    let engine = Engine::with_builtins();
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(0);
    let mut host = Host::default();
    let source = "classDiagram\nclass LongIdentifier";
    let error = engine
        .render_source_bounded(source, &options, None, &mut host)
        .err()
        .unwrap();
    assert_eq!(
        error.message,
        "Diagram output is longer than the limit of 0 characters."
    );
    assert!(
        host.calls.is_empty(),
        "legacy Class early semantic check remains"
    );
    let mut host = Host::default();
    let error = engine
        .render_source_bounded(source, &options, Some(OutputLimit::Number(0.)), &mut host)
        .err()
        .unwrap();
    assert_eq!(
        error.message,
        "Diagram output is longer than the limit of 0 characters."
    );
    assert!(
        !host.calls.is_empty(),
        "exact Class semantic rejection follows callbacks"
    );
    let source = " ".repeat(MAX_PUBLIC_SOURCE_BYTES + 1);
    options.parse.preparation.limit = usize::MAX;
    for output in [None, Some(OutputLimit::Number(f64::INFINITY))] {
        let mut host = Host::default();
        let error = engine
            .render_source_bounded(&source, &options, output, &mut host)
            .err()
            .unwrap();
        assert_eq!(
            error.message,
            "Native transport: declared input exceeds budget"
        );
        assert_eq!(error.code, ErrorCode::Limit);
        assert!(host.calls.is_empty())
    }
}
