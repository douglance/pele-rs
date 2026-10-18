//! Exact edge-limit public feature, pinned Pele 458322a5 / npm1.6.0.
//! Complete reachable graphs retain own order/descriptors, aliases and IEEE bits.
//! The one-million-edge generation ceiling is a separate native extension.
use pele::compat::{public::*, *};
use serde::Serialize;
use serde_json::{json, Value as J};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
fn fixture() -> J {
    serde_json::from_str(include_str!("../../../fixtures/pele/edge-limit.json")).unwrap()
}
fn edge(j: &J) -> EdgeLimit {
    match j["kind"].as_str().unwrap() {
        "undefined" => EdgeLimit::Undefined,
        "null" => EdgeLimit::Null,
        "number" => EdgeLimit::Number(f64::from_bits(
            u64::from_str_radix(j["bits"].as_str().unwrap(), 16).unwrap(),
        )),
        _ => panic!("edge tag"),
    }
}
fn source(j: &J) -> SourceLimit {
    match edge(j) {
        EdgeLimit::Undefined => SourceLimit::Undefined,
        EdgeLimit::Null => SourceLimit::Null,
        EdgeLimit::Number(n) => SourceLimit::Number(n),
    }
}
fn limits(j: &J) -> ParseLimits {
    ParseLimits {
        source: j.get("source").map(source),
        max_edges: j.get("maxEdges").map(edge),
    }
}
fn edge_tag(v: EdgeLimit) -> J {
    match v {
        EdgeLimit::Undefined => json!({"kind":"undefined"}),
        EdgeLimit::Null => json!({"kind":"null"}),
        EdgeLimit::Number(n) => json!({"kind":"number","bits":format!("{:016x}",n.to_bits())}),
    }
}
#[derive(Default)]
struct Host {
    calls: Vec<J>,
}
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        self.calls.push(json!([
            r.text.encode_utf16().collect::<Vec<_>>(),
            format!("{:016x}", r.size.to_bits()),
            r.style
        ]));
        Ok(r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 })
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        Ok(1704067200000.)
    }
}
fn rejection(r: Result<impl Sized, PeleError>) -> J {
    serde_json::to_value(r.err().expect("expected rejection")).unwrap()
}
fn reference(a: &Atom) -> u32 {
    if let Atom::Ref(id) = a {
        *id
    } else {
        panic!("object reference")
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
fn string(g: &mut ModelGraph, text: &str) -> Atom {
    let id = g.strings.len() as u32;
    g.strings.push(text.into());
    Atom::String(id)
}
fn set(g: &mut ModelGraph, id: u32, key: &str, value: Atom) {
    if let Some(at) = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key))
    {
        g.nodes[id as usize].properties_mut()[at].value = value
    } else {
        let key_id = g.strings.len() as u32;
        g.strings.push(key.into());
        g.nodes[id as usize].properties_mut().push(Property {
            key: key_id,
            value,
            flags: 7,
        })
    }
}
fn argument(g: &mut ModelGraph, j: &J) -> Atom {
    match j {
        J::Null => Atom::Null,
        J::Bool(v) => Atom::Bool(*v),
        J::Number(n) => Atom::Number(n.as_f64().unwrap()),
        J::String(s) => string(g, s),
        J::Array(values) => {
            let id = g.nodes.len() as u32;
            g.nodes.push(Node::Array {
                length: values.len() as u32,
                properties: vec![],
            });
            for (i, v) in values.iter().enumerate() {
                let a = argument(g, v);
                set(g, id, &i.to_string(), a)
            }
            Atom::Ref(id)
        }
        J::Object(values) => {
            let id = g.nodes.len() as u32;
            g.nodes.push(Node::Object { properties: vec![] });
            for (k, v) in values {
                let a = argument(g, v);
                set(g, id, k, a)
            }
            Atom::Ref(id)
        }
    }
}
fn render_result(r: &RenderResult, expected: &J, id: &str) {
    assert_eq!(
        serde_json::to_value(r).unwrap(),
        expected["result"],
        "SVG {id}"
    );
    assert_eq!(
        format!("{:016x}", r.rendered.width.to_bits()),
        expected["widthBits"],
        "width {id}"
    );
    assert_eq!(
        format!("{:016x}", r.rendered.height.to_bits()),
        expected["heightBits"],
        "height {id}"
    )
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
fn pinned_numeric_edges_compose_with_source_policy_and_preserve_full_models_graphs_and_svg() {
    let f = fixture();
    assert_eq!(f["revision"], UPSTREAM_REVISION);
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 313);
    for c in cases {
        let id = c["id"].as_str().unwrap();
        let text = c["source"].as_str().unwrap();
        let policy = limits(&c["limits"]);
        let options: ParseOptions =
            serde_json::from_value(c.get("options").unwrap_or(&f["options"]).clone()).unwrap();
        let expected = &c["expected"];
        let typed = engine.parse_with_limits(text, &options, policy);
        let mutable = public.parse_with_limits(text, &options, policy);
        if expected.get("error").is_some() {
            assert_eq!(rejection(typed), expected["error"], "typed {id}");
            assert_eq!(rejection(mutable), expected["error"], "mutable {id}");
            continue;
        }
        let typed = typed.unwrap_or_else(|e| panic!("typed {id}: {e}"));
        assert_eq!(
            typed.diagram_type().as_str(),
            expected["type"].as_str().unwrap()
        );
        assert_eq!(
            typed.edge_limit().map(edge_tag),
            policy.max_edges.map(edge_tag),
            "retained policy {id}"
        );
        assert_eq!(
            serde_json::to_value(typed.model_view().unwrap()).unwrap(),
            expected["model"],
            "typed model {id}"
        );
        let mut mutable = mutable.unwrap_or_else(|e| panic!("mutable {id}: {e}"));
        let snapshot = mutable.snapshot().unwrap();
        if c["parseOnly"] == true {
            assert_eq!(
                array(&snapshot, reference(root_field(&snapshot, "edges"))).len(),
                expected["edgeCount"].as_u64().unwrap() as usize,
                "boundary {id}"
            );
            continue;
        }
        assert_eq!(
            canonical(&snapshot),
            expected["graph"],
            "complete graph {id}"
        );
        if let Some(tag) = expected.get("maxEdges") {
            let actual = field(
                &snapshot,
                reference(root_field(&snapshot, "options")),
                "maxEdges",
            );
            let actual = match actual {
                Atom::Undefined => json!({"kind":"undefined"}),
                Atom::Null => json!({"kind":"null"}),
                Atom::Number(n) => edge_tag(EdgeLimit::Number(*n)),
                _ => panic!("numeric maxEdges"),
            };
            assert_eq!(actual, *tag, "own options bits {id}");
        }
        let mut rendering = RenderOptions::default();
        rendering.render.now = Some(1704067200000.);
        let mut host = Host::default();
        let result = engine
            .render_document(&typed, &rendering, &mut host)
            .unwrap();
        render_result(&result, expected, id);
        assert_eq!(json!(host.calls), expected["calls"], "typed callbacks {id}");
        let mut host = Host::default();
        let outcome = mutable.render_operation(0, &rendering, &mut host).unwrap();
        render_result(&outcome.result.unwrap(), expected, id);
        assert_eq!(
            json!(host.calls),
            expected["calls"],
            "public callbacks {id}"
        );
        assert_eq!(
            canonical(&outcome.graph),
            expected["graph"],
            "retained parse graph {id}"
        );
    }
}
fn edge_summaries(g: &ModelGraph) -> J {
    let Node::Map { entries, .. } = &g.nodes[reference(root_field(g, "edgeById")) as usize] else {
        panic!("edge lookup")
    };
    J::Array(array(g,reference(root_field(g,"edges"))).iter().map(|edge|{let id=reference(edge);let get=|key|{let Atom::String(s)=field(g,id,key)else{panic!("edge string")};g.strings[*s as usize].well_formed().unwrap()};let name=get("id");json!({"id":name,"start":get("start"),"end":get("end"),"edgeByIdSame":entries.iter().any(|(k,v)|matches!(k,Atom::String(s) if g.strings[*s as usize].is(&name))&&matches!(v,Atom::Ref(i) if *i==id))})}).collect())
}
#[test]
fn fractional_cartesian_methods_publish_partial_edges_and_recover_without_retargeting_aliases() {
    let f = fixture();
    let registry = PublicEngine::with_builtins();
    let options: ParseOptions = serde_json::from_value(f["options"].clone()).unwrap();
    for c in f["methods"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let mut doc = registry
            .parse_with_limits(
                c["source"].as_str().unwrap(),
                &options,
                limits(&c["limits"]),
            )
            .unwrap();
        let first = doc.snapshot().unwrap();
        assert_eq!(canonical(&first), c["initial"], "initial {id}");
        let edges = reference(root_field(&first, "edges"));
        for (index, step) in c["steps"].as_array().unwrap().iter().enumerate() {
            let op = &step["op"];
            let mut g = doc.snapshot().unwrap();
            let graph = if let Some(name) = op["method"].as_str() {
                let receiver = g.root.clone();
                let args = op["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| argument(&mut g, v))
                    .collect();
                doc.apply(doc.revision(), g).unwrap();
                let method = doc
                    .model_schema()
                    .methods
                    .iter()
                    .find(|m| m.name == name)
                    .unwrap()
                    .id;
                let out = doc
                    .invoke(
                        doc.revision(),
                        MethodCall {
                            method,
                            receiver,
                            arguments: args,
                        },
                        &mut Host::default(),
                    )
                    .unwrap();
                match out.result {
                    Ok(Atom::Undefined) => assert_eq!(
                        step["returned"],
                        json!(["undefined"]),
                        "return {id}/{index}"
                    ),
                    Err(PublicError::Pele(e)) => assert_eq!(
                        serde_json::to_value(e).unwrap(),
                        step["error"],
                        "partial error {id}/{index}"
                    ),
                    other => panic!("method {id}/{index}: {other:?}"),
                };
                out.graph
            } else {
                let options = reference(root_field(&g, "options"));
                let value = match edge(&op["setMaxEdges"]) {
                    EdgeLimit::Undefined => Atom::Undefined,
                    EdgeLimit::Null => Atom::Null,
                    EdgeLimit::Number(n) => Atom::Number(n),
                };
                set(&mut g, options, "maxEdges", value);
                doc.apply(doc.revision(), g).unwrap();
                doc.snapshot().unwrap()
            };
            assert_eq!(
                canonical(&graph),
                step["graph"],
                "full operation graph {id}/{index}"
            );
            assert_eq!(
                reference(root_field(&graph, "edges")),
                edges,
                "retained edge array {id}/{index}"
            );
            assert_eq!(step["edgesIdentityRetained"], true);
            assert_eq!(
                edge_summaries(&graph),
                step["edges"],
                "ordered cache aliases {id}/{index}"
            );
        }
        let mut host = Host::default();
        let r = doc.render(&RenderOptions::default(), &mut host).unwrap();
        render_result(&r, &c["expected"], id);
        assert_eq!(json!(host.calls), c["expected"]["calls"]);
    }
}
#[derive(Clone, Serialize)]
struct Seen {
    source: String,
    config: Object,
    max_edges: usize,
    generation: u32,
    policy: Option<String>,
}
struct Probe {
    calls: Arc<AtomicUsize>,
    generation: u32,
}
struct State {
    model: Seen,
    graph: ModelGraph,
}
impl DiagramModule for Probe {
    type Model = Seen;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("edge-probe"))
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Seen, PeleError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Seen {
            source: c.source.into(),
            config: c.config.clone(),
            max_edges: c.max_edges,
            generation: self.generation,
            policy: None,
        })
    }
    fn parse_with_edge_limit(
        &self,
        c: ParseContext<'_>,
        limit: EdgeLimit,
    ) -> Result<Seen, PeleError> {
        let mut m = self.parse(c)?;
        m.policy = Some(edge_tag(limit).to_string());
        Ok(m)
    }
    fn render(
        &self,
        m: &Seen,
        c: &Object,
        _: &RenderSettings,
        _: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        assert_eq!(&m.config, c);
        Ok(Rendered {
            svg: format!("<svg><text>{}</text></svg>", self.generation),
            width: m.max_edges as f64,
            height: 1.,
            links: vec![],
        })
    }
}
impl PublicDiagramModule for Probe {
    type PublicState = State;
    fn create_public(&self, m: &Seen) -> Result<State, PeleError> {
        let mut w = GraphWriter::default();
        let value = w.string(m.policy.as_deref().unwrap_or("legacy"));
        let p = w.property("parsedPolicy", Atom::String(value));
        let root = w.node(Node::Object {
            properties: vec![p],
        });
        Ok(State {
            model: m.clone(),
            graph: w.finish(Atom::Ref(root))?,
        })
    }
    fn create_public_with_edge_limit(
        &self,
        m: &Seen,
        limit: EdgeLimit,
    ) -> Result<State, PeleError> {
        let mut s = self.create_public(m)?;
        let root = reference(&s.graph.root);
        let value = string(&mut s.graph, &edge_tag(limit).to_string());
        set(&mut s.graph, root, "createdPolicy", value);
        Ok(s)
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
        s: &mut State,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.render(&s.model, c, o, h)
    }
}
#[test]
fn explicit_selection_keeps_hooks_config_legacy_options_and_registration_ownership() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut engine = Engine::empty();
    engine.register(Probe {
        calls: calls.clone(),
        generation: 1,
    });
    let mut public = PublicEngine::empty();
    public.register(Probe {
        calls: calls.clone(),
        generation: 1,
    });
    let id = DiagramId::extension("edge-probe");
    let options: ParseOptions =
        serde_json::from_value(json!({"limit":0,"maxEdges":0,"config":{"retained":"caller"}}))
            .unwrap();
    let text = "---\ntitle: Front\nconfig:\n  retained: front\n---\ncustom 😀";
    let policy = ParseLimits {
        source: Some(SourceLimit::Undefined),
        max_edges: Some(EdgeLimit::Number(2.5)),
    };
    assert_eq!(
        rejection(engine.parse_as_with_limits(&id, text, &options, ParseLimits::default()))["code"],
        "limit"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let typed = engine
        .parse_as_with_limits(&id, text, &options, policy)
        .unwrap();
    let m = typed.model::<Seen>().unwrap();
    assert_eq!(m.max_edges, 0);
    assert_eq!(
        m.config.get("retained"),
        Some(&Value::String("front".into()))
    );
    assert_eq!(
        m.policy.as_deref(),
        Some(edge_tag(EdgeLimit::Number(2.5)).to_string().as_str())
    );
    assert_eq!(
        typed.edge_limit().map(edge_tag),
        policy.max_edges.map(edge_tag)
    );
    let mut doc = public
        .parse_as_with_limits(&id, text, &options, policy)
        .unwrap();
    let g = doc.snapshot().unwrap();
    let Atom::String(p) = root_field(&g, "parsedPolicy") else {
        panic!()
    };
    let Atom::String(c) = root_field(&g, "createdPolicy") else {
        panic!()
    };
    assert_eq!(g.strings[*p as usize].0, g.strings[*c as usize].0);
    public.register(Probe {
        calls: calls.clone(),
        generation: 9,
    });
    drop(public);
    drop(engine);
    assert_eq!(
        doc.render(&RenderOptions::default(), &mut Host::default())
            .unwrap()
            .rendered
            .svg,
        "<svg><text>1</text></svg>"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let engine = Engine::with_builtins();
    let options = ParseOptions::default();
    let legacy = engine.parse("info", &options).unwrap();
    assert!(legacy.edge_limit().is_none());
    let modern = engine
        .parse_with_limits("info", &options, ParseLimits::default())
        .unwrap();
    assert!(modern.edge_limit().is_none());
    assert_eq!(
        serde_json::to_value(legacy.model_view().unwrap()).unwrap(),
        serde_json::to_value(modern.model_view().unwrap()).unwrap()
    );
}
#[test]
fn native_source_bytes_stay_bounded_for_omitted_and_nonfinite_overrides() {
    let text = " ".repeat(MAX_PUBLIC_SOURCE_BYTES + 1);
    let engine = Engine::with_builtins();
    let public = PublicEngine::with_builtins();
    let mut options = ParseOptions::default();
    options.preparation.limit = usize::MAX;
    let expected = json!({"name":"PeleError","message":"Native transport: declared input exceeds budget","code":"limit","type":null,"line":0,"column":0,"snippet":""});
    for policy in [
        ParseLimits::default(),
        ParseLimits {
            source: Some(SourceLimit::Number(f64::NAN)),
            max_edges: Some(EdgeLimit::Number(f64::INFINITY)),
        },
    ] {
        assert_eq!(
            rejection(engine.parse_with_limits(&text, &options, policy)),
            expected
        );
        assert_eq!(
            rejection(public.parse_as_with_limits(
                &DiagramType::Flowchart.into(),
                &text,
                &options,
                policy
            )),
            expected
        )
    }
}
#[test]
fn native_cartesian_generation_stops_at_the_independent_million_edge_ceiling() {
    assert_eq!(MAX_PUBLIC_EDGES, 1_000_000);
    let starts = (0..1000)
        .map(|i| format!("A{i}"))
        .collect::<Vec<_>>()
        .join(" & ");
    let ends = (0..1001)
        .map(|i| format!("B{i}"))
        .collect::<Vec<_>>()
        .join(" & ");
    let text = format!("flowchart TB\n{starts} --> {ends}");
    assert!(text.encode_utf16().count() < DEFAULT_LIMIT);
    let policy = ParseLimits {
        source: Some(SourceLimit::Undefined),
        max_edges: Some(EdgeLimit::Number(f64::NAN)),
    };
    let actual = rejection(Engine::with_builtins().parse_with_limits(
        &text,
        &ParseOptions::default(),
        policy,
    ));
    assert_eq!(
        actual,
        json!({"name":"PeleError","message":"Native edge generation exceeds work budget","code":"limit","type":"flowchart","line":0,"column":0,"snippet":""})
    );
}
