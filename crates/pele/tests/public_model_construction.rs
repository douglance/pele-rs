//! Prepared diagram source and raw rendering through native owned providers.
use pele::compat::{public::*, *};
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
    fn host_function(&mut self, id: u32) -> Result<std::rc::Rc<dyn HostFunction>, PublicError> {
        assert_eq!(id, 0);
        Ok(std::rc::Rc::new(LiteralTasks))
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        self.calls.push(json!(["clock", bits(1704067200000.)]));
        Ok(1704067200000.)
    }
}
fn canonical(g: &ModelGraph, schema: ModelSchemaView, observations: &J) -> J {
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
            Atom::HostReference(id) => serde_json::json!(["functionObservation", id]),
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
    if let Some(values) = observations.as_array() {
        if !values.is_empty() {
            result["functionObservations"] = observations.clone();
        }
    }
    result
}

struct LiteralTasks;
impl HostFunction for LiteralTasks {
    fn call(
        &self,
        model: &mut dyn ActiveModel,
        receiver: Atom,
        args: &[Atom],
        _: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        assert_eq!(args.len(), 1);
        model.read(&receiver, "tasks")
    }
}
fn decode_graph(input: &J) -> ModelGraph {
    fn string(g: &mut ModelGraph, value: &J) -> u32 {
        let s: JsString = serde_json::from_value(value.clone()).unwrap();
        if let Some(i) = g.strings.iter().position(|v| v == &s) {
            return i as u32;
        }
        g.strings.push(s);
        (g.strings.len() - 1) as u32
    }
    fn atom(g: &mut ModelGraph, a: &J) -> Atom {
        match a[0].as_str().unwrap() {
            "undefined" => Atom::Undefined,
            "null" => Atom::Null,
            "bool" => Atom::Bool(a[1].as_bool().unwrap()),
            "number" => Atom::Number(f64::from_bits(
                u64::from_str_radix(a[1].as_str().unwrap(), 16).unwrap(),
            )),
            "string" => Atom::String(string(g, &a[1])),
            "ref" => Atom::Ref(a[1].as_u64().unwrap() as u32),
            "functionObservation" => Atom::HostReference(a[1].as_u64().unwrap() as u32),
            other => panic!("unknown atom {other}"),
        }
    }
    let mut g = ModelGraph {
        version: 1,
        root: Atom::Undefined,
        strings: vec![],
        nodes: vec![],
    };
    g.root = atom(&mut g, &input["root"]);
    for n in input["nodes"].as_array().unwrap() {
        let mut properties = vec![];
        for p in n["properties"].as_array().unwrap() {
            let key = string(&mut g, &p[0]);
            let value = atom(&mut g, &p[1]);
            properties.push(Property {
                key,
                value,
                flags: p[2].as_u64().unwrap() as u8,
            });
        }
        let node = match n["kind"].as_str().unwrap() {
            "object" => Node::Object { properties },
            "array" => Node::Array {
                length: n["length"].as_u64().unwrap() as u32,
                properties,
            },
            "map" => Node::Map {
                entries: n["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| (atom(&mut g, &p[0]), atom(&mut g, &p[1])))
                    .collect(),
                properties,
            },
            "set" => Node::Set {
                values: n["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| atom(&mut g, p))
                    .collect(),
                properties,
            },
            "date" => Node::Date {
                milliseconds: f64::from_bits(
                    u64::from_str_radix(n["milliseconds"].as_str().unwrap(), 16).unwrap(),
                ),
                properties,
            },
            other => panic!("unexpected fresh node {other}"),
        };
        g.nodes.push(node);
    }
    g
}
fn error_view(e: PublicError) -> J {
    match e {
        PublicError::JavaScript { name, message } => {
            json!({"name":name,"message":message.well_formed().unwrap()})
        }
        PublicError::Pele(e) => serde_json::to_value(e).unwrap(),
        other => panic!("unexpected error {other:?}"),
    }
}
#[test]
fn all_33_declared_plain_models_render_without_parser_or_prototype_injection() {
    let fixture: J = serde_json::from_str(include_str!(
        "../../../fixtures/pele/diagram-model-construction.json"
    ))
    .unwrap();
    let engine = PublicEngine::with_builtins();
    let mut failures = vec![];
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 67);
    for c in fixture["cases"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let check = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let kind = DiagramId::Builtin(
                *DIAGRAM_TYPES
                    .iter()
                    .find(|k| k.as_str() == c["family"].as_str().unwrap())
                    .unwrap(),
            );
            let graph = decode_graph(&c["expected"]["before"]);
            let config: Object = serde_json::from_value(c["config"].clone()).unwrap();
            let mut document = engine
                .create_from_graph_as(&kind, graph, config.clone())
                .unwrap();
            assert_eq!(document.revision(), 0);
            let observations = &c["expected"]["before"]["functionObservations"];
            assert_eq!(
                canonical(
                    &document.snapshot().unwrap(),
                    document.schema_view(),
                    observations
                ),
                c["expected"]["before"],
                "before {id}"
            );
            let options: RenderSettings = serde_json::from_value(c["options"].clone()).unwrap();
            let mut host = Host::default();
            let outcome = document
                .render_raw_operation(0, &config, &options, &mut host)
                .unwrap();
            assert_eq!(outcome.revision, 1);
            if c["expected"].get("error").is_some() {
                assert_eq!(
                    error_view(outcome.result.unwrap_err()),
                    c["expected"]["error"],
                    "error {id}"
                );
            } else {
                let result = outcome
                    .result
                    .unwrap_or_else(|e| panic!("render {id}: {e:?}"));
                assert_eq!(
                    serde_json::to_value(&result).unwrap(),
                    c["expected"]["result"],
                    "SVG {id}"
                );
                assert_eq!(bits(result.width), c["expected"]["widthBits"], "width {id}");
                assert_eq!(
                    bits(result.height),
                    c["expected"]["heightBits"],
                    "height {id}"
                );
            }
            assert_eq!(json!(host.calls), c["expected"]["calls"], "trace {id}");
            assert_eq!(
                canonical(&outcome.graph, document.schema_view(), observations),
                c["expected"]["after"],
                "after {id}"
            );
        }));
        if check.is_err() {
            failures.push(id.to_owned());
        }
    }
    assert!(failures.is_empty(), "construction failures: {failures:?}");
}

struct LiteralProvider {
    generation: usize,
    parses: Arc<AtomicUsize>,
    creates: Arc<AtomicUsize>,
}
impl DiagramModule for LiteralProvider {
    type Model = ();
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("literal-consumer"))
    }
    fn parse(&self, _: ParseContext<'_>) -> Result<(), PeleError> {
        self.parses.fetch_add(1, Ordering::SeqCst);
        panic!("graph construction must not parse")
    }
    fn render(
        &self,
        _: &(),
        _: &Object,
        _: &RenderSettings,
        _: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        panic!("graph renderer uses its own state")
    }
}
impl PublicDiagramModule for LiteralProvider {
    type PublicState = ModelGraph;
    fn create_public(&self, _: &()) -> Result<ModelGraph, PeleError> {
        panic!("not a parsed model")
    }
    fn create_public_from_graph(&self, g: ModelGraph) -> Result<ModelGraph, PeleError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        Ok(g)
    }
    fn export_public(&self, g: &ModelGraph) -> Result<ModelGraph, PeleError> {
        Ok(g.clone())
    }
    fn apply_public(&self, g: &mut ModelGraph, next: ModelGraph) -> Result<(), PeleError> {
        *g = next;
        Ok(())
    }
    fn render_public(
        &self,
        g: &mut ModelGraph,
        c: &Object,
        o: &RenderSettings,
        host: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        let root = match g.root {
            Atom::Ref(id) => id,
            _ => panic!(),
        };
        let label = g.nodes[root as usize]
            .properties()
            .iter()
            .find(|p| g.strings[p.key as usize].is("label"))
            .unwrap();
        let label = g.text(&label.value)?.well_formed().unwrap();
        let width = host.text_width(TextRequest {
            text: &label,
            size: 13.,
            style: 0,
            font_family: None,
            font_family_mono: None,
        })?;
        assert_eq!(o.responsive, None);
        assert_eq!(c.get("retained"), Some(&Value::Bool(true)));
        Ok(Rendered {
            svg: format!("<svg data-generation=\"{}\">{label}</svg>", self.generation),
            width,
            height: 1.,
            links: vec![],
        })
    }
}
#[test]
fn direct_graph_factory_validates_first_and_retains_real_provider_config_and_aliases() {
    let parses = Arc::new(AtomicUsize::new(0));
    let creates = Arc::new(AtomicUsize::new(0));
    let make = |generation| LiteralProvider {
        generation,
        parses: parses.clone(),
        creates: creates.clone(),
    };
    let id = DiagramId::extension("literal-consumer");
    let mut engine = PublicEngine::empty();
    engine.register(make(1));
    let mut w = GraphWriter::default();
    let text = w.string("Literal");
    let label = w.property("label", Atom::String(text));
    let root = w.node(Node::Object {
        properties: vec![label],
    });
    let mut graph = w.finish(Atom::Ref(root)).unwrap();
    let key = graph.strings.len() as u32;
    graph.strings.push("cycle".into());
    graph.nodes[root as usize].properties_mut().push(Property {
        key,
        value: Atom::Ref(root),
        flags: 7,
    });
    let mut invalid = graph.clone();
    invalid.root = Atom::Ref(99);
    assert!(engine
        .create_from_graph_as(&id, invalid, Object::default())
        .is_err());
    assert_eq!(creates.load(Ordering::SeqCst), 0);
    let mut config = Object::default();
    config.insert("retained".into(), Value::Bool(true));
    let mut old = engine
        .create_from_graph_as(&id, graph.clone(), config.clone())
        .unwrap();
    engine.register(make(2));
    let mut new = engine
        .create_from_graph_as(&id, graph.clone(), config.clone())
        .unwrap();
    drop(engine);
    for (document, generation) in [(&mut old, 1), (&mut new, 2)] {
        let outcome = document
            .render_raw_operation(0, &config, &RenderSettings::default(), &mut Host::default())
            .unwrap();
        assert!(outcome
            .result
            .unwrap()
            .svg
            .contains(&format!("generation=\"{generation}\"")));
        assert_eq!(
            serde_json::to_value(outcome.graph).unwrap(),
            serde_json::to_value(&graph).unwrap()
        );
    }
    assert_eq!(creates.load(Ordering::SeqCst), 2);
    assert_eq!(parses.load(Ordering::SeqCst), 0);
    let mut direct = PublicDocument::from_graph(make(3), graph, config.clone()).unwrap();
    assert!(direct
        .render_raw_operation(0, &config, &RenderSettings::default(), &mut Host::default())
        .unwrap()
        .result
        .unwrap()
        .svg
        .contains("generation=\"3\""));
}
