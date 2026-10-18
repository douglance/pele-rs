//! Prepared diagram source and raw rendering through native owned providers.
use pele::compat::{public::*, *};
use serde::Serialize;
use serde_json::{json, Value as J};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
fn bits(n: f64) -> String {
    format!("{:016x}", n.to_bits())
}
#[derive(Default)]
struct Host {
    calls: Vec<J>,
}
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        let w = r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 };
        self.calls.push(json!([
            "width",
            r.text.encode_utf16().collect::<Vec<_>>(),
            bits(r.size),
            r.style,
            bits(w)
        ]));
        Ok(w)
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        self.calls
            .push(json!(["icon", name.encode_utf16().collect::<Vec<_>>()]));
        Ok(Some(TrustedSvg::from_trusted_markup(
            "<path d=\"M0,0\"/>".into(),
        )))
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        self.calls.push(json!(["clock", bits(1704067200000.)]));
        Ok(1704067200000.)
    }
}
fn canonical(g: &ModelGraph, schema: ModelSchemaView) -> J {
    use std::collections::{HashMap, VecDeque};
    fn atom(
        g: &ModelGraph,
        a: &Atom,
        ids: &mut HashMap<u32, usize>,
        queue: &mut VecDeque<u32>,
        functions: &mut Vec<u32>,
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
            Atom::BuiltinFunction(id) => {
                let index = functions
                    .iter()
                    .position(|method| method == id)
                    .unwrap_or_else(|| {
                        functions.push(*id);
                        functions.len() - 1
                    });
                json!(["functionObservation", index])
            }
            Atom::HostReference(id) => serde_json::json!(["host", id]),
        }
    }
    let mut ids = HashMap::new();
    let mut queue = VecDeque::new();
    let mut functions = Vec::new();
    let root = atom(g, &g.root, &mut ids, &mut queue, &mut functions);
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
                            atom(g, k, &mut ids, &mut queue, &mut functions),
                            atom(g, v, &mut ids, &mut queue, &mut functions)
                        ])
                    })
                    .collect()
            }
            Node::Set { values, .. } => {
                out["values"] = values
                    .iter()
                    .map(|v| atom(g, v, &mut ids, &mut queue, &mut functions))
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
                    atom(g, &p.value, &mut ids, &mut queue, &mut functions),
                    p.flags
                ])
            })
            .collect();
        nodes.push(out);
    }
    let mut result = json!({"root":root,"nodes":nodes});
    if !functions.is_empty() {
        result["functionObservations"] = functions.iter().map(|id| {
            let arity = schema.model.methods.iter().find(|method| method.id.0 == *id).unwrap().arity;
            let constructible = schema.function_shapes.iter().find(|shape| shape.method.0 == *id).is_some_and(|shape| shape.constructible);
            json!({"arity":arity,"ownKeys":if constructible {vec!["length","name","prototype"]}else{vec!["length","name"]}})
        }).collect();
    }
    result
}

fn set(graph: &mut ModelGraph, id: u32, key: &str, value: Atom) {
    if let Some(p) = graph.nodes[id as usize]
        .properties_mut()
        .iter_mut()
        .find(|p| graph.strings[p.key as usize].is(key))
    {
        p.value = value;
    } else {
        let key_id = graph.strings.len() as u32;
        graph.strings.push(key.into());
        graph.nodes[id as usize].properties_mut().push(Property {
            key: key_id,
            value,
            flags: 7,
        });
    }
}
fn input(case: &J) -> (DiagramId, Object, Option<&str>, usize, RenderSettings) {
    (
        DIAGRAM_TYPES
            .iter()
            .find(|kind| kind.as_str() == case["family"].as_str().unwrap())
            .copied()
            .unwrap()
            .into(),
        serde_json::from_value(case["config"].clone()).unwrap(),
        case["title"].as_str(),
        case["lineOffset"].as_u64().unwrap() as usize,
        serde_json::from_value(case["options"].clone()).unwrap(),
    )
}
#[test]
fn all_33_prepared_entries_preserve_complete_models_raw_svg_and_callbacks() {
    let fixture: J =
        serde_json::from_str(include_str!("../../../fixtures/pele/diagram-entries.json")).unwrap();
    let typed = Engine::with_builtins();
    let engine = PublicEngine::with_builtins();
    assert_eq!(fixture["entries"].as_array().unwrap().len(), 33);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 99);
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let id = case["id"].as_str().unwrap();
            let (kind, config, title, line_offset, settings) = input(case);
            let source = case["source"].as_str().unwrap();
            let context = || ParseContext {
                source,
                config: &config,
                title,
                line_offset,
                max_edges: 5000,
            };
            let mut public = engine
                .parse_prepared_as(&kind, context(), EdgeLimit::Undefined)
                .unwrap_or_else(|e| panic!("prepared {id}: {e:?}"));
            assert_eq!(
                canonical(&public.snapshot().unwrap(), public.schema_view()),
                case["expected"]["before"],
                "graph before {id}"
            );
            if let Some(edit) = case["edit"].as_object() {
                let mut g = public.snapshot().unwrap();
                let Atom::Ref(root) = g.root else { panic!() };
                for (key, value) in edit {
                    let atom = Atom::String(g.strings.len() as u32);
                    g.strings.push(value.as_str().unwrap().into());
                    set(&mut g, root, key, atom);
                }
                public.apply(public.revision(), g).unwrap();
            }
            let mut host = Host::default();
            let outcome = public
                .render_raw_operation(public.revision(), &config, &settings, &mut host)
                .unwrap();
            let rendered = outcome.result.unwrap_or_else(|e| panic!("raw {id}: {e:?}"));
            assert_eq!(
                serde_json::to_value(&rendered).unwrap(),
                case["expected"]["result"],
                "raw whole SVG {id}"
            );
            assert_eq!(
                bits(rendered.width),
                case["expected"]["widthBits"],
                "width {id}"
            );
            assert_eq!(
                bits(rendered.height),
                case["expected"]["heightBits"],
                "height {id}"
            );
            assert_eq!(
                json!(host.calls),
                case["expected"]["calls"],
                "raw callbacks {id}"
            );
            assert_eq!(
                canonical(&outcome.graph, public.schema_view()),
                case["expected"]["after"],
                "graph after {id}"
            );
            if case["edit"].is_null() {
                let document = typed
                    .parse_prepared_as(&kind, context(), EdgeLimit::Undefined)
                    .unwrap();
                assert!(document.is_prepared());
                let mut host = Host::default();
                let actual = typed
                    .render_document_raw(&document, &config, &settings, &mut host)
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(actual).unwrap(),
                    case["expected"]["result"],
                    "typed raw {id}"
                );
                assert_eq!(
                    json!(host.calls),
                    case["expected"]["calls"],
                    "typed callbacks {id}"
                );
            }
        }));
        if outcome.is_err() {
            failures.push(case["id"].as_str().unwrap().to_owned());
        }
    }
    assert!(
        failures.is_empty(),
        "prepared vector failures: {failures:?}"
    );
    for case in fixture["coreIconCases"].as_array().unwrap() {
        let mut host = Host::default();
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = typed
            .render_source(case["source"].as_str().unwrap(), &options, &mut host)
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            case["expected"]["result"],
            "core icons {}",
            case["id"]
        );
        assert_eq!(
            json!(host.calls),
            case["expected"]["calls"],
            "core icon callback order {}",
            case["id"]
        );
    }
}
#[derive(Serialize)]
struct Captured {
    source: String,
    title: Option<String>,
    line_offset: usize,
    config: Object,
}
struct Provider {
    parses: Arc<AtomicUsize>,
    generation: usize,
}
impl DiagramModule for Provider {
    type Model = Captured;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("prepared-consumer"))
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Captured, PeleError> {
        self.parses.fetch_add(1, Ordering::SeqCst);
        Ok(Captured {
            source: c.source.into(),
            title: c.title.map(str::to_owned),
            line_offset: c.line_offset,
            config: c.config.clone(),
        })
    }
    fn render(
        &self,
        m: &Captured,
        c: &Object,
        o: &RenderSettings,
        r: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        assert_eq!(m.source, "---\nraw no appended newline");
        assert_eq!(m.title.as_deref(), Some("Given"));
        assert_eq!(m.line_offset, 7);
        assert!(r.icon("inline")?.is_none());
        r.direct_icon("ﬂ°°65¶ß")?;
        assert_eq!(r.now_millis()?, r.now_millis()?);
        Ok(Rendered {
            svg: format!(
                "<svg data-generation=\"{}\" data-responsive=\"{:?}\" data-config=\"{}\"/>",
                self.generation,
                o.responsive,
                match c.get("override").unwrap() {
                    Value::String(value) => value.as_str(),
                    _ => panic!("string config"),
                }
            ),
            width: 1.,
            height: 1.,
            links: vec![LinkInfo {
                id: "ﬂ°°65¶ß".into(),
                href: "https://example.com".into(),
                internal: false,
            }],
        })
    }
}
#[test]
fn prepared_source_is_verbatim_and_raw_render_retains_provider_without_core_postprocessing() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut engine = Engine::empty();
    engine.register(Provider {
        parses: count.clone(),
        generation: 1,
    });
    let kind = DiagramId::extension("prepared-consumer");
    let config = Object::default();
    let document = engine
        .parse_prepared_as(
            &kind,
            ParseContext {
                source: "---\nraw no appended newline",
                config: &config,
                title: Some("Given"),
                line_offset: 7,
                max_edges: 0,
            },
            EdgeLimit::Undefined,
        )
        .unwrap();
    engine.register(Provider {
        parses: count.clone(),
        generation: 2,
    });
    drop(engine);
    let mut render_config = Object::default();
    render_config.insert("override".into(), Value::String("passed".into()));
    let mut host = Host::default();
    let settings = RenderSettings {
        output_limit: Some(1),
        ..Default::default()
    };
    let result = Engine::empty()
        .render_document_raw(&document, &render_config, &settings, &mut host)
        .unwrap();
    assert!(result.svg.contains("generation=\"1\""));
    assert!(result.svg.contains("responsive=\"None\""));
    assert_eq!(result.links[0].id, "ﬂ°°65¶ß");
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(
        host.calls[0],
        json!(["icon", "ﬂ°°65¶ß".encode_utf16().collect::<Vec<_>>()])
    );
    assert_eq!(host.calls.len(), 2);
}
#[test]
fn prepared_undefined_edge_policy_is_raw_and_independent_source_budget_remains() {
    let engine = PublicEngine::with_builtins();
    for (kind, head) in [
        (DiagramType::Flowchart, "flowchart TD"),
        (DiagramType::Agentflow, "agentflow-beta TD"),
        (DiagramType::Swimlane, "swimlane-beta TD"),
    ] {
        let source = format!("{head}\n{}", "A-->B\n".repeat(5001));
        let config = Object::default();
        let doc = engine
            .parse_prepared_as(
                &kind.into(),
                ParseContext {
                    source: &source,
                    config: &config,
                    title: None,
                    line_offset: 0,
                    max_edges: 5000,
                },
                EdgeLimit::Undefined,
            )
            .unwrap();
        let graph = doc.snapshot().unwrap();
        let Atom::Ref(root) = graph.root else {
            panic!()
        };
        let fields = graph.nodes[root as usize].properties();
        let Atom::Ref(options) = fields
            .iter()
            .find(|p| graph.strings[p.key as usize].is("options"))
            .unwrap()
            .value
        else {
            panic!()
        };
        assert!(matches!(
            graph.nodes[options as usize]
                .properties()
                .iter()
                .find(|p| graph.strings[p.key as usize].is("maxEdges"))
                .unwrap()
                .value,
            Atom::Undefined
        ));
        let Atom::Ref(edges) = fields
            .iter()
            .find(|p| graph.strings[p.key as usize].is("edges"))
            .unwrap()
            .value
        else {
            panic!()
        };
        assert!(matches!(
            graph.nodes[edges as usize],
            Node::Array { length: 5001, .. }
        ));
    }
    let source = "x".repeat(MAX_PUBLIC_SOURCE_BYTES + 1);
    let config = Object::default();
    let error = Engine::with_builtins()
        .parse_prepared_as(
            &DiagramType::Pie.into(),
            ParseContext {
                source: &source,
                config: &config,
                title: None,
                line_offset: 0,
                max_edges: 0,
            },
            EdgeLimit::Number(f64::NAN),
        )
        .err()
        .unwrap();
    assert_eq!(
        error.message,
        "Native transport: declared input exceeds budget"
    );
    assert_eq!(error.code, ErrorCode::Limit);
}
