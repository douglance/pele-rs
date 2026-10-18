//! Owned selective registry feature against pinned Pele458322a5 / npm1.6.0.
//! Complete lifecycle proof keeps independent registry and document ownership.
use pele::compat::{public::*, *};
use serde::Serialize;
use serde_json::{json, Value as J};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
fn fixture() -> J {
    serde_json::from_str(include_str!("../../../fixtures/pele/registry.json")).unwrap()
}
fn builtin(name: &str) -> DiagramType {
    *DIAGRAM_TYPES
        .iter()
        .find(|kind| kind.as_str() == name)
        .unwrap()
}
fn identity(name: &str) -> DiagramId {
    serde_json::from_value(json!(name)).unwrap()
}
fn error<T>(r: Result<T, PeleError>) -> J {
    match r {
        Ok(_) => panic!("expected rejection"),
        Err(e) => serde_json::to_value(e).unwrap(),
    }
}
fn detection(r: Result<Option<DiagramType>, PeleError>) -> J {
    match r {
        Ok(kind) => json!({"value":kind}),
        Err(e) => json!({"error":e}),
    }
}
fn parse(r: Result<PublicDocument, PeleError>) -> J {
    match r {
        Ok(doc) => json!({"value":{"type":doc.diagram_type()}}),
        Err(e) => json!({"error":e}),
    }
}
#[derive(Default)]
struct Host;
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        Ok(r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 })
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        Ok(1704067200000.)
    }
}
fn assert_membership(registry: &PublicEngine, expected: &J) {
    for (name, wanted) in expected.as_object().unwrap() {
        let id = identity(name);
        if !DIAGRAM_TYPES.iter().any(|kind| kind.as_str() == name) {
            assert!(matches!(id, DiagramId::Extension(_)))
        }
        assert_eq!(
            registry.registered(id),
            wanted.as_bool().unwrap(),
            "registered {name}"
        );
    }
    let capabilities = registry.capabilities();
    assert_eq!(capabilities.len(), 33);
    for kind in DIAGRAM_TYPES {
        let capability = capabilities
            .iter()
            .find(|c| c.diagram_type == DiagramId::Builtin(kind))
            .unwrap();
        assert!(capability.known, "catalogue {kind:?}");
        assert_eq!(
            capability.registered,
            expected[kind.as_str()].as_bool().unwrap(),
            "capability {kind:?}"
        );
    }
}
#[test]
fn owned_selective_and_full_registries_match_pinned_membership_detection_and_errors() {
    let f = fixture();
    assert_eq!(f["revision"], UPSTREAM_REVISION);
    assert_eq!(f["registries"].as_array().unwrap().len(), 5);
    for scenario in f["registries"].as_array().unwrap() {
        let mode = scenario["id"].as_str().unwrap();
        let mut registry = if mode == "full" {
            PublicEngine::with_builtins()
        } else {
            PublicEngine::empty()
        };
        assert_membership(&registry, &scenario["before"]);
        if mode != "full" {
            for name in scenario["selection"].as_array().unwrap() {
                registry.register_builtin(builtin(name.as_str().unwrap()));
            }
        }
        assert_membership(&registry, &scenario["registered"]);
        for c in scenario["cases"].as_array().unwrap() {
            let id = c["id"].as_str().unwrap();
            let source = c["source"].as_str().unwrap();
            let expected = &c["expected"];
            assert_eq!(
                detection(registry.detect_type(source)),
                expected["detection"],
                "detection {mode}/{id}"
            );
            assert_eq!(
                registry.supports(source),
                expected["supports"].as_bool().unwrap(),
                "supports {mode}/{id}"
            );
            if expected.get("parse").is_some() {
                assert_eq!(
                    parse(registry.parse_with_limits(
                        source,
                        &ParseOptions::default(),
                        ParseLimits::default()
                    )),
                    expected["parse"],
                    "parse {mode}/{id}"
                );
            }
        }
        // Duplicate registration replaces its slot without accumulating capabilities.
        for name in scenario["selection"].as_array().unwrap() {
            registry.register_builtin(builtin(name.as_str().unwrap()));
        }
        assert_membership(&registry, &scenario["registered"]);
    }
}
#[test]
fn registry_owners_are_independent_and_builtin_documents_keep_retained_config_and_identity() {
    let f = fixture();
    let mut pies = PublicEngine::empty();
    let mut infos = PublicEngine::empty();
    pies.register_builtin(DiagramType::Pie);
    infos.register_builtin(DiagramType::Info);
    assert!(pies.registered(DiagramType::Pie));
    assert!(!infos.registered(DiagramType::Pie));
    assert!(!pies.registered(DiagramType::Info));
    assert!(infos.registered(DiagramType::Info));
    assert_eq!(
        error(infos.parse_with_limits(
            "pie\n\"A\":1",
            &ParseOptions::default(),
            ParseLimits::default()
        ))["code"],
        "unsupported-diagram"
    );
    let owned = &f["registries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "pie")
        .unwrap()["ownership"];
    let options: ParseOptions =
        serde_json::from_value(json!({"config":{"pie":{"useMaxWidth":false}}})).unwrap();
    let mut doc = pies
        .parse_with_limits(
            owned["source"].as_str().unwrap(),
            &options,
            ParseLimits::default(),
        )
        .unwrap();
    let before = doc.render(&RenderOptions::default(), &mut Host).unwrap();
    assert_eq!(
        serde_json::to_value(&before).unwrap(),
        owned["expected"]["before"]
    );
    let mut graph = doc.snapshot().unwrap();
    let text = graph.strings.len() as u32;
    graph.strings.push(JsString::from("foreign-type"));
    let Atom::Ref(root) = graph.root else {
        panic!("pie root")
    };
    let property = graph.nodes[root as usize]
        .properties()
        .iter()
        .position(|p| graph.strings[p.key as usize].is("type"))
        .unwrap();
    graph.nodes[root as usize].properties_mut()[property].value = Atom::String(text);
    doc.apply(0, graph).unwrap();
    pies.register_builtin(DiagramType::Pie);
    let mut contradiction = RenderOptions::default();
    contradiction.parse.preparation.config =
        serde_json::from_value(json!({"pie":{"useMaxWidth":true}})).unwrap();
    let fresh = pies
        .parse_with_limits(
            owned["source"].as_str().unwrap(),
            &ParseOptions::default(),
            ParseLimits::default(),
        )
        .unwrap();
    let mut fresh = fresh;
    assert_eq!(
        serde_json::to_value(fresh.render(&RenderOptions::default(), &mut Host).unwrap()).unwrap(),
        owned["expected"]["fresh"]
    );
    drop(pies);
    drop(infos);
    assert_eq!(
        serde_json::to_value(doc.render(&contradiction, &mut Host).unwrap()).unwrap(),
        owned["expected"]["after"]
    );
    assert_eq!(doc.diagram_type().as_str(), "pie");
}
const OLD_SCHEMA: ModelSchema = ModelSchema {
    id: "consumer/old-registry@1",
    version: 1,
    root: None,
    records: &[],
    prototypes: &[],
    methods: &[MethodSchema {
        id: MethodId(1),
        name: "generation",
        arity: 0,
    }],
};
const NEW_SCHEMA: ModelSchema = ModelSchema {
    id: "consumer/new-registry@1",
    version: 1,
    root: None,
    records: &[],
    prototypes: &[],
    methods: &[MethodSchema {
        id: MethodId(1),
        name: "newGeneration",
        arity: 0,
    }],
};
#[derive(Serialize)]
struct Model {
    title: String,
}
#[derive(Default)]
struct Calls {
    parses: AtomicUsize,
    invokes: AtomicUsize,
    drops: AtomicUsize,
}
struct Replacement {
    generation: u32,
    calls: Arc<Calls>,
}
impl Drop for Replacement {
    fn drop(&mut self) {
        self.calls.drops.fetch_add(1, Ordering::SeqCst);
    }
}
impl DiagramModule for Replacement {
    type Model = Model;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: DiagramType::Info.into(),
            section: Some("consumer".into()),
            keep_comments: false,
        }
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Model, PeleError> {
        self.calls.parses.fetch_add(1, Ordering::SeqCst);
        Ok(Model {
            title: c.title.unwrap_or("None").into(),
        })
    }
    fn render(
        &self,
        m: &Model,
        c: &Object,
        o: &RenderSettings,
        _: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        let tag = c
            .get("consumer")
            .and_then(Value::as_object)
            .and_then(|c| c.get("tag"));
        assert_eq!(tag, Some(&Value::String("captured".into())));
        assert_eq!(o.responsive, Some(false));
        Ok(Rendered {
            svg: format!("<svg><text>{}:{}</text></svg>", self.generation, m.title),
            width: self.generation as f64,
            height: 1.,
            links: vec![],
        })
    }
}
impl PublicDiagramModule for Replacement {
    type PublicState = ModelGraph;
    fn model_schema(&self) -> &'static ModelSchema {
        if self.generation == 1 {
            &OLD_SCHEMA
        } else {
            &NEW_SCHEMA
        }
    }
    fn create_public(&self, m: &Model) -> Result<ModelGraph, PeleError> {
        let mut w = GraphWriter::default();
        let label = w.string(m.title.as_str());
        let p = w.property("title", Atom::String(label));
        let root = w.node(Node::Object {
            properties: vec![p],
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
    fn invoke_public(
        &self,
        s: &mut ModelGraph,
        call: MethodCall,
        _: &Object,
        _: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        assert_eq!(
            serde_json::to_value(&call.receiver).unwrap(),
            serde_json::to_value(&s.root).unwrap()
        );
        self.calls.invokes.fetch_add(1, Ordering::SeqCst);
        Ok(Atom::Number(self.generation as f64))
    }
    fn render_public(
        &self,
        s: &mut ModelGraph,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        let title = s.text(s.field("title")?.unwrap())?.well_formed().unwrap();
        self.render(&Model { title }, c, o, h)
    }
}
fn generation(doc: &mut PublicDocument) -> f64 {
    let receiver = doc.snapshot().unwrap().root;
    let outcome = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: MethodId(1),
                receiver,
                arguments: vec![],
            },
            &mut Host,
        )
        .unwrap();
    match outcome.result.unwrap() {
        Atom::Number(n) => n,
        _ => panic!("generation"),
    }
}
#[test]
fn builtin_replacement_retains_exact_provider_schema_methods_config_and_owner_lifetime() {
    let first = Arc::new(Calls::default());
    let second = Arc::new(Calls::default());
    let mut registry = PublicEngine::empty();
    registry.register(Replacement {
        generation: 1,
        calls: first.clone(),
    });
    let source="---\ntitle: Front\nconfig:\n  consumer:\n    tag: captured\n    useMaxWidth: false\n---\ninfo";
    let mut old = registry
        .parse_with_limits(source, &ParseOptions::default(), ParseLimits::default())
        .unwrap();
    assert_eq!(old.model_schema().id, OLD_SCHEMA.id);
    registry.register(Replacement {
        generation: 9,
        calls: second.clone(),
    });
    let mut newer = registry
        .parse_with_limits(source, &ParseOptions::default(), ParseLimits::default())
        .unwrap();
    assert_eq!(newer.model_schema().id, NEW_SCHEMA.id);
    registry.register_builtin(DiagramType::Info);
    assert!(registry.registered(DiagramType::Info));
    assert_eq!(registry.capabilities().len(), 33);
    let mut builtin = registry
        .parse_with_limits("info", &ParseOptions::default(), ParseLimits::default())
        .unwrap();
    assert_ne!(builtin.model_schema().id, OLD_SCHEMA.id);
    assert_ne!(builtin.model_schema().id, NEW_SCHEMA.id);
    assert!(builtin
        .render(&RenderOptions::default(), &mut Host)
        .unwrap()
        .rendered
        .svg
        .contains("pele-version"));
    drop(registry);
    assert_eq!(first.drops.load(Ordering::SeqCst), 0);
    assert_eq!(second.drops.load(Ordering::SeqCst), 0);
    assert_eq!(generation(&mut old), 1.);
    assert_eq!(generation(&mut newer), 9.);
    assert_eq!(old.model_schema().methods[0].name, "generation");
    assert_eq!(newer.model_schema().methods[0].name, "newGeneration");
    assert_eq!(
        old.render(&RenderOptions::default(), &mut Host)
            .unwrap()
            .rendered
            .svg,
        "<svg><text>1:Front</text></svg>"
    );
    assert_eq!(
        newer
            .render(&RenderOptions::default(), &mut Host)
            .unwrap()
            .rendered
            .svg,
        "<svg><text>9:Front</text></svg>"
    );
    assert_eq!(first.parses.load(Ordering::SeqCst), 1);
    assert_eq!(second.parses.load(Ordering::SeqCst), 1);
    assert_eq!(first.invokes.load(Ordering::SeqCst), 1);
    assert_eq!(second.invokes.load(Ordering::SeqCst), 1);
    drop(old);
    assert_eq!(first.drops.load(Ordering::SeqCst), 1);
    drop(newer);
    assert_eq!(second.drops.load(Ordering::SeqCst), 1);
}
