//! Exact output-limit feature slice against pinned Pele 458322a5 / npm1.6.0.
//! Complete SVGs, error fields and host-call traces use bounded real diagrams.
//! The separate returned-output byte ceiling is not an incremental allocation cap.
use pele::compat::{public::*, *};
use serde::Serialize;
use serde_json::{json, Value as J};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
const NOW: f64 = 1704067200000.;
fn fixture() -> J {
    serde_json::from_str(include_str!("../../../fixtures/pele/output-limit.json")).unwrap()
}
fn limit(j: &J) -> OutputLimit {
    match j["kind"].as_str().unwrap() {
        "undefined" => OutputLimit::Undefined,
        "null" => OutputLimit::Null,
        "number" => OutputLimit::Number(f64::from_bits(
            u64::from_str_radix(j["bits"].as_str().unwrap(), 16).unwrap(),
        )),
        _ => panic!("fixture limit"),
    }
}
#[derive(Default)]
struct Host {
    calls: Vec<J>,
    fail: bool,
}
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        self.calls.push(json!([
            r.text.encode_utf16().collect::<Vec<_>>(),
            format!("{:016x}", r.size.to_bits()),
            r.style
        ]));
        if self.fail {
            return Err(PeleError::new(
                "output-limit host stopped",
                ErrorCode::Semantic,
            ));
        }
        Ok(r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 })
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        Ok(NOW)
    }
}
fn result(actual: Result<RenderResult, PeleError>, expected: &J, id: &str) {
    match actual {
        Ok(actual) => {
            assert_eq!(
                serde_json::to_value(&actual).unwrap(),
                expected["result"],
                "SVG {id}"
            );
            assert_eq!(
                format!("{:016x}", actual.rendered.width.to_bits()),
                expected["widthBits"],
                "width {id}"
            );
            assert_eq!(
                format!("{:016x}", actual.rendered.height.to_bits()),
                expected["heightBits"],
                "height {id}"
            );
            assert_eq!(
                actual.rendered.svg.encode_utf16().count(),
                expected["outputUTF16Length"].as_u64().unwrap() as usize,
                "UTF16 {id}"
            );
        }
        Err(error) => assert_eq!(
            serde_json::to_value(error).unwrap(),
            expected["error"],
            "error {id}"
        ),
    }
}
fn public_result(actual: Result<RenderResult, PublicError>, expected: &J, id: &str) {
    result(
        actual.map_err(|e| match e {
            PublicError::Pele(e) => e,
            other => panic!("unexpected category {id}: {other:?}"),
        }),
        expected,
        id,
    )
}
fn reference(a: &Atom) -> u32 {
    if let Atom::Ref(id) = a {
        *id
    } else {
        panic!("reference")
    }
}
fn field<'a>(g: &'a ModelGraph, id: u32, key: &str) -> &'a Atom {
    &g.nodes[id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .value
}
fn root_field<'a>(g: &'a ModelGraph, key: &str) -> &'a Atom {
    field(g, reference(&g.root), key)
}
fn array(g: &ModelGraph, id: u32) -> Vec<Atom> {
    let Node::Array { length, properties } = &g.nodes[id as usize] else {
        panic!("array")
    };
    (0..*length)
        .map(|i| {
            properties
                .iter()
                .find(|p| g.strings[p.key as usize].is(&i.to_string()))
                .map(|p| p.value.clone())
                .unwrap_or(Atom::Undefined)
        })
        .collect()
}
fn scalar(g: &ModelGraph, a: &Atom) -> J {
    match a {
        Atom::Undefined => json!("undefined"),
        Atom::Null => J::Null,
        Atom::Bool(b) => json!(b),
        Atom::Number(n) => serde_json::to_value(Value::Number(*n)).unwrap(),
        Atom::String(id) => json!(g.strings[*id as usize].well_formed().unwrap()),
        Atom::Ref(id) => match &g.nodes[*id as usize] {
            Node::Date { milliseconds, .. } => {
                serde_json::to_value(Value::Number(*milliseconds)).unwrap()
            }
            _ => panic!("scalar date"),
        },
        _ => panic!("scalar"),
    }
}
/// The fixture's complete public projection expands Maps/Sets and follows
/// JSON's omission/date rules. Alias identities are asserted separately below.
/// These eight bounded, acyclic documents never require a transport-sized walk.
fn expanded(g: &ModelGraph, atom: &Atom) -> Option<J> {
    match atom {
        Atom::Undefined | Atom::BuiltinFunction(_) => None,
        Atom::Ref(id) => Some(match &g.nodes[*id as usize] {
            Node::Map { entries, .. } => J::Array(
                entries
                    .iter()
                    .map(|(k, v)| {
                        json!([
                            expanded(g, k).unwrap_or(J::Null),
                            expanded(g, v).unwrap_or(J::Null)
                        ])
                    })
                    .collect(),
            ),
            Node::Set { values, .. } => J::Array(
                values
                    .iter()
                    .map(|v| expanded(g, v).unwrap_or(J::Null))
                    .collect(),
            ),
            Node::Array { .. } => J::Array(
                array(g, *id)
                    .iter()
                    .map(|v| expanded(g, v).unwrap_or(J::Null))
                    .collect(),
            ),
            Node::Date { milliseconds, .. } => {
                fields_at_offset(*milliseconds, 0).map_or(J::Null, |f| {
                    let c = f.civil;
                    json!(format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                        c.year, c.month, c.day, c.hour, c.minute, c.second, c.millisecond
                    ))
                })
            }
            Node::Object { properties }
            | Node::Instance { properties, .. }
            | Node::Error { properties, .. } => {
                let mut out = serde_json::Map::new();
                for p in properties.iter().filter(|p| p.flags & 1 != 0) {
                    if let Some(value) = expanded(g, &p.value) {
                        out.insert(g.strings[p.key as usize].well_formed().unwrap(), value);
                    }
                }
                J::Object(out)
            }
        }),
        Atom::HostReference(_) => panic!("fixture has no host references"),
        _ => Some(scalar(g, atom)),
    }
}
fn schedule(g: &ModelGraph) -> J {
    let tasks = array(g, reference(root_field(g, "tasks")));
    let Node::Map { entries, .. } = &g.nodes[reference(root_field(g, "byId")) as usize] else {
        panic!("byId")
    };
    let records:Vec<_>=tasks.iter().enumerate().map(|(i,t)|{
        let t_id=reference(t);let id=field(g,t_id,"id");
        json!({"id":scalar(g,id),"startTime":scalar(g,field(g,t_id,"startTime")),"endTime":scalar(g,field(g,t_id,"endTime")),"renderEndTime":scalar(g,field(g,t_id,"renderEndTime")),"processed":scalar(g,field(g,t_id,"processed")),"manualEndTime":scalar(g,field(g,t_id,"manualEndTime")),"byIdSame":entries.iter().any(|(key,value)|scalar(g,key)==scalar(g,id)&&matches!((value,t),(Atom::Ref(a),Atom::Ref(b)) if a==b)),"previousEndSame":i>0&&matches!((field(g,t_id,"startTime"),field(g,reference(&tasks[i-1]),"endTime")),(Atom::Ref(a),Atom::Ref(b)) if a==b)})
    }).collect();
    json!({"now":scalar(g,root_field(g,"now")),"work":scalar(g,root_field(g,"work")),"excludedDays":array(g,reference(root_field(g,"excludedDays"))).iter().map(|a|scalar(g,a)).collect::<Vec<_>>(),"datedTokens":scalar(g,root_field(g,"datedTokens")),"tasks":records})
}
#[test]
fn pinned_numeric_nullish_and_utf16_limits_follow_real_rendering_and_callbacks() {
    let f = fixture();
    assert_eq!(f["revision"], UPSTREAM_REVISION);
    let options: RenderOptions = serde_json::from_value(f["options"].clone()).unwrap();
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 192);
    for c in cases {
        let id = c["id"].as_str().unwrap();
        let source = c["source"].as_str().unwrap();
        let argument = limit(&c["limit"]);
        let expected = &c["expected"];
        let typed = engine.parse(source, &options.parse).unwrap();
        assert_eq!(
            serde_json::to_value(typed.model_view().unwrap()).unwrap(),
            expected["typedProjectionBefore"],
            "model {id}"
        );
        let mut host = Host::default();
        result(
            engine.render_document_with_output_limit(&typed, &options, argument, &mut host),
            expected,
            id,
        );
        assert_eq!(
            json!(host.calls),
            expected["calls"],
            "retained callbacks {id}"
        );
        let mut host = Host::default();
        result(
            engine.render_source_with_output_limit(source, &options, argument, &mut host),
            expected,
            id,
        );
        assert_eq!(
            json!(host.calls),
            expected["calls"],
            "source callbacks {id}"
        );
        let mut doc = public.parse(source, &options.parse).unwrap();
        let before = doc.snapshot().unwrap();
        assert_eq!(
            expanded(&before, &before.root).unwrap(),
            expected["modelBefore"],
            "public before {id}"
        );
        let mut host = Host::default();
        let outcome = doc
            .render_operation_with_output_limit(0, &options, argument, &mut host)
            .unwrap();
        assert_eq!(outcome.revision, 1);
        public_result(outcome.result, expected, id);
        assert_eq!(
            json!(host.calls),
            expected["calls"],
            "mutable callbacks {id}"
        );
        outcome.graph.validate().unwrap();
        assert_eq!(
            expanded(&outcome.graph, &outcome.graph.root).unwrap(),
            expected["modelAfter"],
            "public after {id}"
        );
        if let Some(expected_schedule) = expected.get("schedule") {
            assert_eq!(
                schedule(&outcome.graph),
                *expected_schedule,
                "scheduled state {id}"
            );
            assert!(expected["tasksIdentityRetained"].as_bool().unwrap());
            assert!(
                matches!((root_field(&before,"tasks"),root_field(&outcome.graph,"tasks")),(Atom::Ref(a),Atom::Ref(b)) if a==b)
            );
        }
    }
}
#[test]
fn host_errors_win_after_scheduling_and_failed_output_operations_recover() {
    let f = fixture();
    let options: RenderOptions = serde_json::from_value(f["options"].clone()).unwrap();
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    for c in f["ordering"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let source = c["source"].as_str().unwrap();
        let expected = &c["expected"];
        let mut doc = public.parse(source, &options.parse).unwrap();
        let mut host = Host {
            fail: true,
            ..Default::default()
        };
        let outcome = doc
            .render_operation_with_output_limit(0, &options, OutputLimit::Number(0.), &mut host)
            .unwrap();
        assert_eq!(outcome.revision, 1);
        assert!(
            matches!(outcome.result,Err(PublicError::Pele(ref e)) if e.message==expected["hostError"]["message"].as_str().unwrap()&&e.code==ErrorCode::Semantic)
        );
        assert_eq!(
            json!(host.calls),
            expected["calls"],
            "host error callbacks {id}"
        );
        assert_eq!(
            expanded(&outcome.graph, &outcome.graph.root).unwrap(),
            expected["modelAfter"],
            "public host-error state {id}"
        );
        if let Some(expected_schedule) = expected.get("schedule") {
            assert_eq!(
                schedule(&outcome.graph),
                *expected_schedule,
                "partial state {id}"
            )
        }
        let mut host = Host::default();
        assert_eq!(
            doc.render_operation_with_output_limit(
                0,
                &options,
                OutputLimit::Number(f64::INFINITY),
                &mut host
            )
            .unwrap_err()
            .message,
            "Stale public model revision"
        );
        assert!(host.calls.is_empty());
        let failed = doc
            .render_operation_with_output_limit(1, &options, OutputLimit::Number(0.), &mut host)
            .unwrap();
        assert!(matches!(failed.result,Err(PublicError::Pele(ref e)) if e.code==ErrorCode::Limit));
        let recovered = doc
            .render_operation_with_output_limit(
                2,
                &options,
                OutputLimit::Number(f64::INFINITY),
                &mut Host::default(),
            )
            .unwrap();
        assert!(recovered.result.is_ok());
        assert_eq!(recovered.revision, 3);
        let typed = engine.parse(source, &options.parse).unwrap();
        let mut host = Host {
            fail: true,
            ..Default::default()
        };
        assert_eq!(
            engine
                .render_document_with_output_limit(
                    &typed,
                    &options,
                    OutputLimit::Number(f64::NEG_INFINITY),
                    &mut host
                )
                .err()
                .unwrap()
                .message,
            "output-limit host stopped"
        );
        assert_eq!(json!(host.calls), expected["calls"]);
    }
}
#[derive(Serialize)]
struct Model;
struct Extension {
    generation: u32,
    calls: Arc<AtomicUsize>,
    oversized: bool,
}
struct State {
    graph: ModelGraph,
}
impl DiagramModule for Extension {
    type Model = Model;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("output-probe"))
    }
    fn parse(&self, _: ParseContext<'_>) -> Result<Model, PeleError> {
        Ok(Model)
    }
    fn render(
        &self,
        _: &Model,
        _: &Object,
        _: &RenderSettings,
        _: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Ok(Rendered {
            svg: "legacy".into(),
            width: 1.,
            height: 1.,
            links: vec![],
        })
    }
    fn render_with_output_limit(
        &self,
        _: &Model,
        _: &Object,
        _: &RenderSettings,
        host: &mut dyn RenderResources,
        _: OutputLimit,
    ) -> Result<Rendered, PeleError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        host.text_width(TextRequest {
            text: "extension",
            size: 16.,
            style: 0,
            font_family: None,
            font_family_mono: None,
        })?;
        Ok(Rendered {
            svg: if self.oversized {
                // One owned allocation; no diagram geometry or huge fixture.
                "x".repeat(MAX_PUBLIC_OUTPUT_BYTES + 1)
            } else {
                String::new()
            },
            width: self.generation as f64,
            height: -0.,
            links: vec![],
        })
    }
}
impl PublicDiagramModule for Extension {
    type PublicState = State;
    fn create_public(&self, _: &Model) -> Result<State, PeleError> {
        let mut w = GraphWriter::default();
        let p = w.property("renders", Atom::Number(0.));
        let root = w.node(Node::Object {
            properties: vec![p],
        });
        Ok(State {
            graph: w.finish(Atom::Ref(root))?,
        })
    }
    fn export_public(&self, s: &State) -> Result<ModelGraph, PeleError> {
        Ok(s.graph.clone())
    }
    fn apply_public(&self, s: &mut State, g: ModelGraph) -> Result<(), PeleError> {
        s.graph = g;
        Ok(())
    }
    fn render_public(
        &self,
        _: &mut State,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.render(&Model, c, o, h)
    }
    fn render_public_operation_with_output_limit(
        &self,
        s: &mut State,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
        limit: OutputLimit,
    ) -> Result<Rendered, PublicError> {
        let root = reference(&s.graph.root);
        let Atom::Number(n) = &mut s.graph.nodes[root as usize].properties_mut()[0].value else {
            panic!()
        };
        *n += 1.;
        self.render_with_output_limit(&Model, c, o, h, limit)
            .map_err(Into::into)
    }
}
#[test]
fn empty_output_exact_hooks_keep_registration_legacy_behavior_and_operation_state() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = PublicEngine::empty();
    registry.register(Extension {
        generation: 1,
        calls: calls.clone(),
        oversized: false,
    });
    let id = DiagramId::extension("output-probe");
    let mut doc = registry
        .parse_as(&id, "custom", &ParseOptions::default())
        .unwrap();
    registry.register(Extension {
        generation: 9,
        calls: calls.clone(),
        oversized: false,
    });
    drop(registry);
    let options = RenderOptions::default();
    assert_eq!(
        doc.render(&options, &mut Host::default())
            .unwrap()
            .rendered
            .svg,
        "legacy"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for (revision, argument) in [
        OutputLimit::Undefined,
        OutputLimit::Null,
        OutputLimit::Number(-0.),
        OutputLimit::Number(f64::NAN),
        OutputLimit::Number(f64::INFINITY),
    ]
    .into_iter()
    .enumerate()
    {
        let outcome = doc
            .render_operation_with_output_limit(
                revision as u64,
                &options,
                argument,
                &mut Host::default(),
            )
            .unwrap();
        let rendered = outcome.result.unwrap();
        assert!(rendered.rendered.svg.is_empty());
        assert_eq!(rendered.rendered.width, 1.);
        assert_eq!(rendered.rendered.height.to_bits(), (-0f64).to_bits());
        assert_eq!(
            scalar(&outcome.graph, root_field(&outcome.graph, "renders")),
            json!(revision + 1)
        );
    }
    let failed = doc
        .render_operation_with_output_limit(
            5,
            &options,
            OutputLimit::Number(-0.5),
            &mut Host::default(),
        )
        .unwrap();
    assert!(
        matches!(failed.result,Err(PublicError::Pele(ref e)) if e.message=="Diagram output is longer than the limit of -0.5 characters."&&e.diagram_type.as_deref()==Some("output-probe"))
    );
    assert_eq!(
        scalar(&failed.graph, root_field(&failed.graph, "renders")),
        json!(6)
    );
    let failed = doc
        .render_operation_with_output_limit(
            6,
            &options,
            OutputLimit::Number(f64::NEG_INFINITY),
            &mut Host {
                fail: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        matches!(failed.result,Err(PublicError::Pele(ref e)) if e.message=="output-limit host stopped")
    );
    assert_eq!(
        scalar(&failed.graph, root_field(&failed.graph, "renders")),
        json!(7)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 7);
    let engine = Engine::with_builtins();
    let mut old = RenderOptions::default();
    old.render.output_limit = Some(0);
    assert_eq!(
        engine
            .render_source("info", &old, &mut Host::default())
            .err()
            .unwrap()
            .message,
        "Diagram output is longer than the limit of 0 characters."
    );
    assert!(engine
        .render_source_with_output_limit("info", &old, OutputLimit::Null, &mut Host::default())
        .is_ok());
    assert!(engine
        .render_source_with_output_limit("info", &old, OutputLimit::Undefined, &mut Host::default())
        .is_ok());
}

#[test]
fn nonfinite_output_policy_keeps_the_independent_returned_byte_ceiling() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = PublicEngine::empty();
    registry.register(Extension {
        generation: 1,
        calls: calls.clone(),
        oversized: true,
    });
    let mut doc = registry
        .parse_as(
            &DiagramId::extension("output-probe"),
            "custom",
            &ParseOptions::default(),
        )
        .unwrap();
    let outcome = doc
        .render_operation_with_output_limit(
            0,
            &RenderOptions::default(),
            OutputLimit::Number(f64::NAN),
            &mut Host::default(),
        )
        .unwrap();
    assert_eq!(outcome.revision, 1);
    assert_eq!(
        scalar(&outcome.graph, root_field(&outcome.graph, "renders")),
        json!(1)
    );
    assert!(
        matches!(outcome.result, Err(PublicError::Pele(ref error)) if serde_json::to_value(error).unwrap()==json!({"name":"PeleError","message":"Native render output exceeds byte budget","code":"limit","type":"output-probe","line":0,"column":0,"snippet":""}))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn class_exact_preflight_uses_emitted_ids_and_preserves_callback_error_order() {
    let registry = PublicEngine::with_builtins();
    let source = "classDiagram\nclass A";
    let options = RenderOptions::default();
    let baseline = Engine::with_builtins()
        .render_source_with_output_limit(
            source,
            &options,
            OutputLimit::Number(f64::INFINITY),
            &mut Host::default(),
        )
        .unwrap();
    {
        let mut doc = registry.parse(source, &options.parse).unwrap();
        let mut graph = doc.snapshot().unwrap();
        let classes = reference(root_field(&graph, "classes"));
        let key = graph.strings.len() as u32;
        // The Map key is private lookup data; the emitted class.id remains A.
        // Escaping this key would exceed64MiB, but it never enters the drawing.
        graph.strings.push(JsString(vec![b'"' as u16; 12_000_000]));
        let Node::Map { entries, .. } = &mut graph.nodes[classes as usize] else {
            panic!("classes")
        };
        entries[0].0 = Atom::String(key);
        doc.apply(0, graph).unwrap();
        let outcome = doc
            .render_operation_with_output_limit(
                1,
                &options,
                OutputLimit::Number(f64::INFINITY),
                &mut Host::default(),
            )
            .unwrap();
        assert_eq!(outcome.result.unwrap(), baseline);
    }
    let mut doc = registry.parse(source, &options.parse).unwrap();
    let mut graph = doc.snapshot().unwrap();
    let classes = reference(root_field(&graph, "classes"));
    let Node::Map { entries, .. } = &graph.nodes[classes as usize] else {
        panic!("classes")
    };
    let record = reference(&entries[0].1);
    let id = graph.strings.len() as u32;
    graph.strings.push(JsString::from("ﬂ°°1.5¶ß"));
    let property = graph.nodes[record as usize]
        .properties()
        .iter()
        .position(|p| graph.strings[p.key as usize].is("id"))
        .unwrap();
    graph.nodes[record as usize].properties_mut()[property].value = Atom::String(id);
    doc.apply(0, graph).unwrap();
    let mut host = Host {
        fail: true,
        ..Default::default()
    };
    let outcome = doc
        .render_operation_with_output_limit(
            1,
            &options,
            OutputLimit::Number(f64::INFINITY),
            &mut host,
        )
        .unwrap();
    assert!(
        matches!(outcome.result,Err(PublicError::Pele(ref e)) if e.message=="output-limit host stopped")
    );
    assert_eq!(host.calls.len(), 1);
    assert_eq!(outcome.revision, 2);
}
