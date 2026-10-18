//! Public well-formed source-limit lane, pinned to Pele 458322a5 / npm 1.6.0.
//! fixtures/pele/source-limit.json includes complete models, SVGs and errors;
//! IEEE arguments travel as bits. Native byte budgets are an independent contract.
use pele::compat::{public::*, *};
use serde::Serialize;
use serde_json::{json, Value as Json};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct FixedHost;
impl RenderResources for FixedHost {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        Ok(r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 })
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        Ok(1_704_067_200_000.)
    }
}
fn limit(value: &Json) -> SourceLimit {
    match value["kind"].as_str().unwrap() {
        "undefined" => SourceLimit::Undefined,
        "null" => SourceLimit::Null,
        "number" => SourceLimit::Number(f64::from_bits(
            u64::from_str_radix(value["bits"].as_str().unwrap(), 16).unwrap(),
        )),
        _ => panic!("fixture limit"),
    }
}
fn error(result: Result<impl Sized, PeleError>) -> Json {
    serde_json::to_value(result.err().expect("expected rejection")).unwrap()
}
#[test]
fn pinned_limits_prepare_parse_and_render_through_both_native_engines() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/source-limit.json")).unwrap();
    assert_eq!(fixture["revision"], UPSTREAM_REVISION);
    let options: ParseOptions = serde_json::from_value(fixture["options"].clone()).unwrap();
    let mut rendering: RenderOptions = serde_json::from_value(fixture["options"].clone()).unwrap();
    rendering.render.now = Some(fixture["host"]["now"].as_f64().unwrap());
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 207);
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let source = case["source"].as_str().unwrap();
        let argument = limit(&case["limit"]);
        let expected = &case["expected"];
        let typed = engine.parse_with_source_limit(source, &options, argument);
        let mutable = public.parse_with_source_limit(source, &options, argument);
        let inspection = engine.inspect_with_source_limit(source, &options.preparation, argument);
        if let Some(expected_error) = expected.get("error") {
            assert_eq!(error(typed), *expected_error, "typed {id}");
            assert_eq!(error(mutable), *expected_error, "mutable {id}");
            if expected_error["code"] == "limit" {
                assert_eq!(error(inspection), *expected_error, "inspection {id}");
                continue;
            }
        } else {
            let typed = typed.unwrap_or_else(|e| panic!("typed {id}: {e}"));
            assert_eq!(
                serde_json::to_value(typed.model_view().unwrap()).unwrap(),
                expected["model"],
                "model {id}"
            );
            let result = engine
                .render_document(&typed, &rendering, &mut FixedHost)
                .unwrap();
            assert_eq!(
                serde_json::to_value(&result).unwrap(),
                expected["result"],
                "SVG {id}"
            );
            assert_eq!(
                format!("{:016x}", result.rendered.width.to_bits()),
                expected["widthBits"],
                "width {id}"
            );
            assert_eq!(
                format!("{:016x}", result.rendered.height.to_bits()),
                expected["heightBits"],
                "height {id}"
            );
            let mut mutable = mutable.unwrap_or_else(|e| panic!("mutable {id}: {e}"));
            let outcome = mutable
                .render_operation(0, &rendering, &mut FixedHost)
                .unwrap();
            assert_eq!(
                serde_json::to_value(outcome.result.unwrap()).unwrap(),
                expected["result"],
                "mutable SVG {id}"
            );
            outcome.graph.validate().unwrap();
        }
        // Preparation itself is identical to the existing public lane after its
        // source gate succeeds, including BOM/CRLF/comments/entity encoding.
        let old_options = PrepareOptions {
            limit: source.encode_utf16().count(),
            config: options.preparation.config.clone(),
        };
        assert_eq!(
            serde_json::to_value(inspection.unwrap()).unwrap(),
            serde_json::to_value(engine.inspect(source, &old_options).unwrap()).unwrap(),
            "preparation {id}"
        );
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Seen {
    source: String,
    config: Object,
    title: Option<String>,
    line_offset: usize,
    max_edges: usize,
    generation: u32,
}
#[derive(Clone)]
struct Probe {
    calls: Arc<AtomicUsize>,
    generation: u32,
}
struct ProbeState {
    model: Seen,
    graph: ModelGraph,
}
impl DiagramModule for Probe {
    type Model = Seen;
    fn descriptor(&self) -> ModuleDescriptor {
        let mut d = ModuleDescriptor::new(DiagramId::extension("source-probe"));
        d.keep_comments = true;
        d
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Seen, PeleError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Seen {
            source: c.source.into(),
            config: c.config.clone(),
            title: c.title.map(str::to_owned),
            line_offset: c.line_offset,
            max_edges: c.max_edges,
            generation: self.generation,
        })
    }
    fn render(
        &self,
        m: &Seen,
        config: &Object,
        _: &RenderSettings,
        _: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        assert_eq!(&m.config, config);
        Ok(Rendered {
            svg: format!("<svg><text>{}</text></svg>", self.generation),
            width: m.max_edges as f64,
            height: m.line_offset as f64,
            links: vec![],
        })
    }
}
impl PublicDiagramModule for Probe {
    type PublicState = ProbeState;
    fn create_public(&self, m: &Seen) -> Result<ProbeState, PeleError> {
        let mut w = GraphWriter::default();
        let source = w.string(m.source.as_str());
        let properties = vec![
            w.property("source", Atom::String(source)),
            w.property("generation", Atom::Number(m.generation as f64)),
        ];
        let root = w.node(Node::Object { properties });
        Ok(ProbeState {
            model: m.clone(),
            graph: w.finish(Atom::Ref(root))?,
        })
    }
    fn export_public(&self, s: &ProbeState) -> Result<ModelGraph, PeleError> {
        Ok(s.graph.clone())
    }
    fn apply_public(&self, s: &mut ProbeState, g: ModelGraph) -> Result<(), PeleError> {
        s.graph = g;
        Ok(())
    }
    fn render_public(
        &self,
        s: &mut ProbeState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.render(&s.model, c, o, h)
    }
}
#[test]
fn explicit_module_selection_retains_context_registration_and_parser_call_order() {
    let calls = Arc::new(AtomicUsize::new(0));
    let probe = Probe {
        calls: calls.clone(),
        generation: 1,
    };
    let id = DiagramId::extension("source-probe");
    let mut engine = Engine::empty();
    engine.register(probe.clone());
    let mut public = PublicEngine::empty();
    public.register(probe);
    let source="---\r\ntitle: Front\r\nconfig:\r\n  selected: front\r\n---\r\n%% author 😀\r\ncustom #35;\r\n";
    let options: ParseOptions = serde_json::from_value(
        json!({"limit":0,"maxEdges":2,"config":{"selected":"caller","retained":"yes"}}),
    )
    .unwrap();
    assert_eq!(
        error(engine.parse_as(&id, source, &options))["code"],
        "limit"
    );
    assert_eq!(
        error(public.parse_as(&id, source, &options))["code"],
        "limit"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for argument in [
        SourceLimit::Null,
        SourceLimit::Number(-0.),
        SourceLimit::Number(source.encode_utf16().count() as f64 - 0.5),
    ] {
        assert_eq!(
            error(public.parse_as_with_source_limit(&id, source, &options, argument))["code"],
            "limit"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut legacy = options.clone();
    legacy.preparation.limit = source.encode_utf16().count();
    let old = engine.parse_as(&id, source, &legacy).unwrap();
    let new = engine
        .parse_as_with_source_limit(&id, source, &options, SourceLimit::Undefined)
        .unwrap();
    assert_eq!(new.model::<Seen>(), old.model::<Seen>());
    let captured = new.model::<Seen>().unwrap();
    assert_eq!(captured.title.as_deref(), Some("Front"));
    assert_eq!(
        captured.config.get("selected"),
        Some(&Value::String("front".into()))
    );
    assert_eq!(
        captured.config.get("retained"),
        Some(&Value::String("yes".into()))
    );
    assert_eq!(captured.max_edges, 2);
    assert!(
        captured.line_offset > 0
            && captured.source.contains("%% author 😀")
            && captured.source.ends_with('\n')
    );
    let mut document = public
        .parse_as_with_source_limit(&id, source, &options, SourceLimit::Number(f64::INFINITY))
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    public.register(Probe {
        calls: calls.clone(),
        generation: 9,
    });
    drop(public);
    drop(engine);
    let before = document.snapshot().unwrap();
    let expected = RenderOptions::default();
    let result = document
        .render_operation(0, &expected, &mut FixedHost)
        .unwrap();
    assert_eq!(
        result.result.unwrap().rendered.svg,
        "<svg><text>1</text></svg>"
    );
    assert_eq!(
        serde_json::to_value(result.graph).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let empty = Engine::empty();
    assert_eq!(
        error(empty.parse_as_with_source_limit(&id, "", &options, SourceLimit::Number(-1.)))
            ["code"],
        "limit"
    );
    let missing = error(empty.parse_as_with_source_limit(
        &id,
        "",
        &options,
        SourceLimit::Number(f64::INFINITY),
    ));
    assert_eq!(missing["code"], "unsupported-diagram");
    assert_eq!(missing["type"], "source-probe");
}
#[test]
fn edge_and_legacy_source_limits_keep_their_existing_contract() {
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    let source = "flowchart TD\nA-->B\nB-->C\nC-->D";
    let mut options = ParseOptions::default();
    options.max_edges = 1;
    let expected = error(engine.parse(source, &options));
    assert_eq!(expected["code"], "limit");
    assert_eq!(expected["type"], "flowchart");
    options.preparation.limit = 0;
    assert_eq!(
        error(engine.parse_with_source_limit(source, &options, SourceLimit::Number(f64::NAN))),
        expected
    );
    assert_eq!(
        error(public.parse_with_source_limit(source, &options, SourceLimit::Number(f64::INFINITY))),
        expected
    );
    let pie = "pie\n\"A\":1";
    let old = error(engine.parse(pie, &options));
    assert_eq!(
        old["message"],
        "Diagram source is longer than the limit of 0 characters."
    );
    assert_eq!(error(public.parse(pie, &options)), old);
    assert!(engine
        .parse_with_source_limit(
            pie,
            &options,
            SourceLimit::Number(pie.encode_utf16().count() as f64)
        )
        .is_ok());
    assert!(public
        .parse_with_source_limit(pie, &options, SourceLimit::Undefined)
        .is_ok());
}
#[test]
fn native_byte_budget_precedes_comparison_detection_and_extension_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut public = PublicEngine::empty();
    public.register(Probe {
        calls: calls.clone(),
        generation: 1,
    });
    let engine = Engine::empty();
    let id = DiagramId::extension("source-probe");
    let options = ParseOptions::default();
    let expected = json!({"name":"PeleError","code":"limit","message":"Native transport: declared input exceeds budget","type":null,"line":0,"column":0,"snippet":""});
    let oversized = " ".repeat(MAX_PUBLIC_SOURCE_BYTES + 1);
    for argument in [
        SourceLimit::Undefined,
        SourceLimit::Null,
        SourceLimit::Number(f64::NAN),
        SourceLimit::Number(f64::INFINITY),
        SourceLimit::Number(f64::NEG_INFINITY),
        SourceLimit::Number(-0.),
    ] {
        assert_eq!(
            error(engine.inspect_with_source_limit(&oversized, &options.preparation, argument)),
            expected
        );
        assert_eq!(
            error(engine.parse_with_source_limit(&oversized, &options, argument)),
            expected
        );
        assert_eq!(
            error(public.parse_as_with_source_limit(&id, &oversized, &options, argument)),
            expected
        );
    }
    // This is a UTF-8 transport ceiling, independently of UTF-16 source length.
    let multibyte = "é".repeat(MAX_PUBLIC_SOURCE_BYTES / 2 + 1);
    assert!(multibyte.encode_utf16().count() < MAX_PUBLIC_SOURCE_BYTES);
    assert_eq!(
        error(engine.inspect_with_source_limit(
            &multibyte,
            &options.preparation,
            SourceLimit::Number(f64::INFINITY)
        )),
        expected
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    // Existing Rust inspection keeps its caller-selected usize source contract.
    let legacy = PrepareOptions {
        limit: usize::MAX,
        config: Object::default(),
    };
    assert!(engine.inspect(&oversized, &legacy).is_ok());
    let boundary = " ".repeat(MAX_PUBLIC_SOURCE_BYTES);
    assert!(engine
        .inspect_with_source_limit(
            &boundary,
            &options.preparation,
            SourceLimit::Number(f64::INFINITY)
        )
        .is_ok());
}
