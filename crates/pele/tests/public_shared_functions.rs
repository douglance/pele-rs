//! Shared callable identity is a native extension descriptor, not JS diagram logic.
use pele::compat::{
    diagrams::{
        kanban::{Kanban, KanbanModel, KanbanPublicState},
        mindmap::Mindmap,
    },
    public::*,
    text::MetricsResources,
    *,
};
struct Registration {
    shared: &'static [SharedFunction],
}
impl DiagramModule for Registration {
    type Model = KanbanModel;
    fn descriptor(&self) -> ModuleDescriptor {
        Kanban.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        Kanban.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Kanban.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Registration {
    type PublicState = KanbanPublicState;
    fn model_schema(&self) -> &'static ModelSchema {
        Kanban.model_schema()
    }
    fn shared_functions(&self) -> &'static [SharedFunction] {
        self.shared
    }
    fn create_public(&self, m: &KanbanModel) -> Result<Self::PublicState, PeleError> {
        Kanban.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Kanban.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        Kanban.apply_public(s, g)
    }
    fn invoke_public(
        &self,
        s: &mut Self::PublicState,
        c: MethodCall,
        o: &Object,
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        Kanban.invoke_public(s, c, o, h)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Kanban.render_public(s, c, o, h)
    }
}
fn call_shape(doc: &mut PublicDocument) -> f64 {
    let mut graph = doc.snapshot().unwrap();
    let start = graph.strings.len() as u32;
    graph.strings.push(JsString::from("("));
    let end = graph.strings.len() as u32;
    graph.strings.push(JsString::from(")"));
    let receiver = graph.root.clone();
    doc.apply(doc.revision(), graph).unwrap();
    let outcome = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: MethodId(0),
                receiver,
                arguments: vec![Atom::String(start), Atom::String(end)],
            },
            &mut MetricsResources,
        )
        .unwrap();
    if let Ok(Atom::Number(value)) = outcome.result {
        value
    } else {
        panic!("native getType result")
    }
}
#[test]
fn outline_documents_share_the_actual_callable_descriptor_and_native_behavior() {
    let mut registry = PublicEngine::empty();
    registry.register(Kanban);
    registry.register(Mindmap);
    let mut kanban = registry
        .parse("kanban\n  Work\n    Task", &ParseOptions::default())
        .unwrap();
    let mut mindmap = registry
        .parse(
            "mindmap\n  root((Core))\n    Leaf",
            &ParseOptions::default(),
        )
        .unwrap();
    for document in [&kanban, &mindmap] {
        let view = document.schema_view();
        view.validate().unwrap();
        assert_eq!(view.shared_functions.len(), 1);
        assert_eq!(
            view.shared_functions[0].identity,
            "pele/common/outline/getType@1"
        );
        assert!(matches!(
            document.snapshot().unwrap().field("getType").unwrap(),
            Some(Atom::BuiltinFunction(0))
        ));
        let json = serde_json::to_value(view).unwrap();
        assert_eq!(json["sharedFunctions"][0]["method"], 0);
        assert_eq!(json["methods"][0]["name"], "getType");
    }
    assert_eq!(call_shape(&mut kanban), 1.);
    assert_eq!(call_shape(&mut mindmap), 1.);
    assert!(kanban
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap()
        .rendered
        .svg
        .contains("Task"));
    assert!(mindmap
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap()
        .rendered
        .svg
        .contains("Leaf"));
}
#[test]
fn invalid_replacement_metadata_rejects_new_documents_and_keeps_retained_handlers() {
    static VALID: [SharedFunction; 1] = [SharedFunction {
        method: MethodId(0),
        identity: "consumer/outline@1",
    }];
    static UNKNOWN: [SharedFunction; 1] = [SharedFunction {
        method: MethodId(999),
        identity: "consumer/outline@1",
    }];
    static DUPLICATE: [SharedFunction; 2] = [
        SharedFunction {
            method: MethodId(0),
            identity: "one",
        },
        SharedFunction {
            method: MethodId(0),
            identity: "two",
        },
    ];
    let mut registry = PublicEngine::empty();
    registry.register(Registration { shared: &VALID });
    let mut retained = registry
        .parse("kanban\n  Kept", &ParseOptions::default())
        .unwrap();
    registry.register(Registration { shared: &UNKNOWN });
    let error = registry
        .parse("kanban\n  New", &ParseOptions::default())
        .err()
        .unwrap();
    assert_eq!(error.message, "Unknown shared public method");
    registry.register(Registration { shared: &DUPLICATE });
    assert_eq!(
        registry
            .parse("kanban\n  New", &ParseOptions::default())
            .err()
            .unwrap()
            .message,
        "Invalid shared public function identity"
    );
    drop(registry);
    assert_eq!(
        retained.schema_view().shared_functions[0].identity,
        "consumer/outline@1"
    );
    assert_eq!(call_shape(&mut retained), 1.);
    assert!(retained
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap()
        .rendered
        .svg
        .contains("Kept"));
}
