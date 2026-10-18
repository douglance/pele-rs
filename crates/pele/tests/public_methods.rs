//! Consumer-defined methods through the public document API. This extension
//! exercises descriptors, retained native dispatch and post-exception state;
//! no JavaScript interpreter or family-specific method list is involved.
use pele::compat::{public::*, text::MetricsResources, *};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
const ROOT: PrototypeId = PrototypeId(71);
static SCHEMA: ModelSchema = ModelSchema {
    id: "consumer/ledger@1",
    version: 1,
    root: Some(RecordId(1)),
    records: &[RecordSchema {
        id: RecordId(1),
        name: "Ledger",
        kind: RecordKind::Object,
        prototype: Some(ROOT),
        fields: &[
            FieldSchema {
                name: "count",
                kind: FieldKind::Number,
                optional: false,
                undefined: false,
                nullable: false,
                flags: 7,
            },
            FieldSchema {
                name: "child",
                kind: FieldKind::Object,
                optional: false,
                undefined: false,
                nullable: false,
                flags: 7,
            },
        ],
        members: &[MemberSchema {
            name: "getSelf",
            flags: 7,
            member: MemberKind::Method {
                method: MethodId(3),
            },
        }],
    }],
    prototypes: &[PrototypeSchema {
        id: ROOT,
        name: "LedgerDb",
        parent: PrototypeParent::Object,
        members: &[
            MemberSchema {
                name: "advance",
                flags: 6,
                member: MemberKind::Method {
                    method: MethodId(2),
                },
            },
            MemberSchema {
                name: "value",
                flags: 4,
                member: MemberKind::Accessor {
                    get: Some(MethodId(1)),
                    set: None,
                },
            },
            MemberSchema {
                name: "getChild",
                flags: 6,
                member: MemberKind::Method {
                    method: MethodId(4),
                },
            },
            MemberSchema {
                name: "failNative",
                flags: 6,
                member: MemberKind::Method {
                    method: MethodId(5),
                },
            },
            MemberSchema {
                name: "failType",
                flags: 6,
                member: MemberKind::Method {
                    method: MethodId(6),
                },
            },
            MemberSchema {
                name: "failHost",
                flags: 6,
                member: MemberKind::Method {
                    method: MethodId(7),
                },
            },
            MemberSchema {
                name: "newRecord",
                flags: 6,
                member: MemberKind::Method {
                    method: MethodId(8),
                },
            },
            MemberSchema {
                name: "badReturn",
                flags: 6,
                member: MemberKind::Method {
                    method: MethodId(9),
                },
            },
        ],
    }],
    methods: &[
        MethodSchema {
            id: MethodId(1),
            name: "get value",
            arity: 0,
        },
        MethodSchema {
            id: MethodId(2),
            name: "advance",
            arity: 1,
        },
        MethodSchema {
            id: MethodId(3),
            name: "getSelf",
            arity: 0,
        },
        MethodSchema {
            id: MethodId(4),
            name: "getChild",
            arity: 0,
        },
        MethodSchema {
            id: MethodId(5),
            name: "failNative",
            arity: 0,
        },
        MethodSchema {
            id: MethodId(6),
            name: "failType",
            arity: 0,
        },
        MethodSchema {
            id: MethodId(7),
            name: "failHost",
            arity: 1,
        },
        MethodSchema {
            id: MethodId(8),
            name: "newRecord",
            arity: 0,
        },
        MethodSchema {
            id: MethodId(9),
            name: "badReturn",
            arity: 0,
        },
        MethodSchema {
            id: MethodId(10),
            name: "failLosslessPele",
            arity: 0,
        },
    ],
};
#[derive(Serialize)]
struct LedgerModel {
    count: f64,
}
#[derive(Clone)]
struct Ledger {
    parses: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    generation: u32,
}
struct LedgerState {
    graph: ModelGraph,
    root: u32,
    child: u32,
}
fn property<'a>(g: &'a ModelGraph, id: u32, key: &str) -> &'a Atom {
    &g.nodes[id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(key))
        .unwrap()
        .value
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
        g.strings.push(JsString::from(key));
        g.nodes[id as usize].properties_mut().push(Property {
            key: key_id,
            value,
            flags: 7,
        })
    }
}
fn count(g: &ModelGraph, id: u32) -> f64 {
    match property(g, id, "count") {
        Atom::Number(n) => *n,
        _ => panic!("ledger count"),
    }
}
impl DiagramModule for Ledger {
    type Model = LedgerModel;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("consumer-ledger"))
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<LedgerModel, PeleError> {
        self.parses.fetch_add(1, Ordering::SeqCst);
        Ok(LedgerModel {
            count: c.source.trim().parse().unwrap(),
        })
    }
    fn render(
        &self,
        m: &LedgerModel,
        _: &Object,
        _: &RenderSettings,
        _: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Ok(Rendered {
            svg: format!("<svg><text>{}</text></svg>", m.count),
            width: 100.,
            height: 40.,
            links: vec![],
        })
    }
}
impl PublicDiagramModule for Ledger {
    type PublicState = LedgerState;
    fn model_schema(&self) -> &'static ModelSchema {
        &SCHEMA
    }
    fn create_public(&self, m: &LedgerModel) -> Result<LedgerState, PeleError> {
        let mut w = GraphWriter::default();
        let properties = vec![w.property("count", Atom::Number(m.count))];
        let child = w.node(Node::Object { properties });
        let properties = vec![
            w.property("count", Atom::Number(m.count)),
            w.property("child", Atom::Ref(child)),
            w.property("getSelf", Atom::BuiltinFunction(3)),
        ];
        let root = w.node(Node::Instance {
            prototype: ROOT,
            properties,
        });
        Ok(LedgerState {
            graph: w.finish(Atom::Ref(root))?,
            root,
            child,
        })
    }
    fn export_public(&self, s: &LedgerState) -> Result<ModelGraph, PeleError> {
        Ok(s.graph.clone())
    }
    fn apply_public(&self, s: &mut LedgerState, g: ModelGraph) -> Result<(), PeleError> {
        s.graph = g;
        Ok(())
    }
    fn render_public(
        &self,
        s: &mut LedgerState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.render(
            &LedgerModel {
                count: count(&s.graph, s.root),
            },
            c,
            o,
            h,
        )
    }
    fn invoke_public(
        &self,
        s: &mut LedgerState,
        call: MethodCall,
        c: &Object,
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if call.method == MethodId(3) {
            return Ok(call.receiver);
        }
        if !matches!(call.receiver,Atom::Ref(id) if id==s.root) {
            return Err(PublicError::JavaScript {
                name: JavaScriptErrorKind::TypeError,
                message: JsString::from("Ledger receiver required"),
            });
        }
        match call.method.0 {
            1 => Ok(Atom::Number(count(&s.graph, s.root))),
            2 => {
                let step = match c.get("step") {
                    Some(Value::Number(n)) => *n,
                    _ => 1.,
                };
                let by = match call.arguments.first() {
                    Some(Atom::Number(n)) => *n,
                    _ => 1.,
                };
                let n = count(&s.graph, s.root) + step * by * f64::from(self.generation);
                set(&mut s.graph, s.root, "count", Atom::Number(n));
                set(&mut s.graph, s.child, "count", Atom::Number(n));
                Ok(Atom::Ref(s.child))
            }
            4 => Ok(Atom::Ref(s.child)),
            5 | 6 | 7 | 9 => {
                let n = count(&s.graph, s.root) + 1.;
                set(&mut s.graph, s.root, "count", Atom::Number(n));
                match call.method.0 {
                    5 => {
                        h.text_width(TextRequest {
                            text: "after mutation",
                            size: 16.,
                            style: 0,
                            font_family: None,
                            font_family_mono: None,
                        })?;
                        Err(PeleError::new("ledger native failure", ErrorCode::Semantic).into())
                    }
                    6 => Err(PublicError::JavaScript {
                        name: match call.arguments.first() {
                            Some(Atom::Number(1.)) => JavaScriptErrorKind::RangeError,
                            Some(Atom::Number(2.)) => JavaScriptErrorKind::Error,
                            _ => JavaScriptErrorKind::TypeError,
                        },
                        message: JsString(vec![65, 0xd800, 66]),
                    }),
                    // A token-aware host bridge can return this opaque identity unchanged.
                    // Legacy RenderResources alone cannot originate such a token.
                    7 => match call.arguments.first() {
                        Some(Atom::HostReference(token)) => {
                            Err(PublicError::HostException(HostExceptionToken(*token)))
                        }
                        _ => panic!("host token argument"),
                    },
                    _ => Ok(Atom::Ref(u32::MAX)),
                }
            }
            10 => {
                let n = count(&s.graph, s.root) + 1.;
                set(&mut s.graph, s.root, "count", Atom::Number(n));
                Err(PublicPeleError {
                    message: JsString(vec![65, 0xd800, 66]),
                    code: ErrorCode::Semantic,
                    diagram_type: Some(JsString(vec![0xdc00])),
                    line: 7,
                    column: 9,
                    snippet: JsString(vec![10, 0xd800]),
                }
                .into())
            }
            8 => {
                let id = s.graph.nodes.len() as u32;
                s.graph.nodes.push(Node::Object { properties: vec![] });
                Ok(Atom::Ref(id))
            }
            _ => unreachable!(),
        }
    }
}
fn module() -> Ledger {
    Ledger {
        parses: Arc::new(AtomicUsize::new(0)),
        calls: Arc::new(AtomicUsize::new(0)),
        generation: 1,
    }
}
fn root_id(g: &ModelGraph) -> u32 {
    match g.root {
        Atom::Ref(id) => id,
        _ => panic!(),
    }
}
fn invoke(
    doc: &mut PublicDocument,
    method: u32,
    receiver: Atom,
    arguments: Vec<Atom>,
) -> InvocationOutcome {
    doc.invoke(
        doc.revision(),
        MethodCall {
            method: MethodId(method),
            receiver,
            arguments,
        },
        &mut MetricsResources,
    )
    .unwrap()
}
fn number_result(result: Result<Atom, PublicError>) -> f64 {
    match result {
        Ok(Atom::Number(n)) => n,
        _ => panic!("numeric return"),
    }
}
fn fingerprint(g: &ModelGraph) -> String {
    serde_json::to_string(g).unwrap()
}
#[test]
fn consumer_methods_getter_schema_receiver_aliases_and_retained_registration() {
    let original = module();
    let calls = original.calls.clone();
    let parses = original.parses.clone();
    let mut engine = PublicEngine::empty();
    engine.register(original.clone());
    let mut options = ParseOptions::default();
    options
        .preparation
        .config
        .insert("step".into(), Value::Number(3.));
    let mut doc = engine
        .parse_as(&DiagramId::extension("consumer-ledger"), "4", &options)
        .unwrap();
    let graph = doc.snapshot().unwrap();
    let root = root_id(&graph);
    assert!(matches!(
        graph.nodes[root as usize],
        Node::Instance {
            prototype: ROOT,
            ..
        }
    ));
    assert_eq!(graph.root_properties().unwrap().len(), 3);
    assert!(graph.field("value").unwrap().is_none());
    assert!(matches!(
        graph.field("getSelf").unwrap(),
        Some(Atom::BuiltinFunction(3))
    ));
    let schema = doc.model_schema();
    assert_eq!(schema.id, "consumer/ledger@1");
    assert_eq!(schema.method(MethodId(2)).unwrap().arity, 1);
    assert_eq!(schema.method(MethodId(1)).unwrap().name, "get value");
    let getter = schema
        .prototype(ROOT)
        .unwrap()
        .members
        .iter()
        .find(|m| m.name == "value")
        .unwrap();
    assert_eq!(getter.flags, 4);
    assert!(matches!(
        getter.member,
        MemberKind::Accessor {
            get: Some(MethodId(1)),
            set: None
        }
    ));
    let shape = serde_json::to_value(schema).unwrap();
    assert_eq!(shape["prototypes"][0]["members"][1]["kind"], "accessor");
    assert_eq!(
        number_result(invoke(&mut doc, 1, Atom::Ref(root), vec![]).result),
        4.
    );
    let outcome = invoke(
        &mut doc,
        2,
        Atom::Ref(root),
        vec![Atom::Number(2.), Atom::Undefined],
    );
    let child = match outcome.result {
        Ok(Atom::Ref(id)) => id,
        _ => panic!(),
    };
    assert_eq!(count(&outcome.graph, child), 10.);
    assert_eq!(count(&outcome.graph, root), 10.);
    assert!(matches!(outcome.graph.field("child").unwrap(),Some(Atom::Ref(id)) if *id==child));
    let alias = invoke(&mut doc, 4, Atom::Ref(root), vec![]);
    assert!(matches!(alias.result,Ok(Atom::Ref(id)) if id==child));
    let borrowed = invoke(&mut doc, 3, Atom::Ref(child), vec![]);
    assert!(matches!(borrowed.result,Ok(Atom::Ref(id)) if id==child));
    let detached = invoke(&mut doc, 8, Atom::Ref(root), vec![]);
    let detached = match detached.result {
        Ok(Atom::Ref(id)) => id,
        _ => panic!(),
    };
    let alias = invoke(&mut doc, 3, Atom::Ref(detached), vec![]);
    assert!(matches!(alias.result,Ok(Atom::Ref(id)) if id==detached));
    // Registry replacement/drop cannot retarget an already owned native module.
    let mut replacement = original;
    replacement.generation = 100;
    engine.register(replacement);
    drop(engine);
    let outcome = invoke(&mut doc, 2, Atom::Ref(root), vec![Atom::Number(1.)]);
    assert_eq!(count(&outcome.graph, root), 13.);
    assert_eq!(parses.load(Ordering::SeqCst), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 7);
    let rendered = doc
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    assert!(rendered.rendered.svg.contains(">13<"));
    // Existing typed construction keeps config and module ownership too.
    let mut config = Object::default();
    config.insert("step".into(), Value::Number(5.));
    let mut typed =
        PublicDocument::from_module(module(), &LedgerModel { count: 20. }, config).unwrap();
    let root = root_id(&typed.snapshot().unwrap());
    assert_eq!(
        count(&invoke(&mut typed, 2, Atom::Ref(root), vec![]).graph, root),
        25.
    );
}
struct RejectHost;
impl RenderResources for RejectHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new(
            "host measurement rejected",
            ErrorCode::Limit,
        ))
    }
}
#[test]
fn throwing_methods_publish_partial_edits_revision_and_exact_error_values() {
    let m = module();
    let mut doc =
        PublicDocument::from_module(m, &LedgerModel { count: 0. }, Object::default()).unwrap();
    let root = root_id(&doc.snapshot().unwrap());
    let native = invoke(&mut doc, 5, Atom::Ref(root), vec![]);
    assert_eq!(native.revision, 1);
    assert_eq!(count(&native.graph, root), 1.);
    assert!(
        matches!(native.result,Err(PublicError::Pele(PeleError{message,..})) if message=="ledger native failure")
    );
    let js = invoke(&mut doc, 6, Atom::Ref(root), vec![]);
    assert_eq!(js.revision, 2);
    assert_eq!(count(&js.graph, root), 2.);
    assert!(
        matches!(&js.result,Err(PublicError::JavaScript{name:JavaScriptErrorKind::TypeError,message}) if message.0==[65,0xd800,66])
    );
    let json = serde_json::to_string(&js).unwrap();
    let roundtrip: InvocationOutcome = serde_json::from_str(&json).unwrap();
    assert!(
        matches!(roundtrip.result,Err(PublicError::JavaScript{message,..}) if message.0==[65,0xd800,66])
    );
    let host = invoke(
        &mut doc,
        7,
        Atom::Ref(root),
        vec![Atom::HostReference(u32::MAX)],
    );
    assert_eq!(host.revision, 3);
    assert_eq!(count(&host.graph, root), 3.);
    assert!(matches!(
        host.result,
        Err(PublicError::HostException(HostExceptionToken(u32::MAX)))
    ));
    let encoded = serde_json::to_value(&host).unwrap();
    assert_eq!(encoded["result"]["Err"]["value"], u32::MAX);
    // Old measurement callbacks still expose PeleError, explicitly distinct from
    // token-aware callbacks: no fake reconstruction of a host exception identity.
    let measured = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: MethodId(5),
                receiver: Atom::Ref(root),
                arguments: vec![],
            },
            &mut RejectHost,
        )
        .unwrap();
    assert_eq!(measured.revision, 4);
    assert_eq!(count(&measured.graph, root), 4.);
    assert!(
        matches!(measured.result,Err(PublicError::Pele(PeleError{message,..})) if message=="host measurement rejected")
    );
    let invalid = invoke(&mut doc, 9, Atom::Ref(root), vec![]);
    assert_eq!(invalid.revision, 5);
    assert_eq!(count(&invalid.graph, root), 5.);
    assert!(matches!(invalid.result, Err(PublicError::Pele(_))));
    assert_eq!(
        fingerprint(&invalid.graph),
        fingerprint(&doc.snapshot().unwrap())
    );
    let ok = invoke(&mut doc, 1, Atom::Ref(root), vec![]);
    assert_eq!(number_result(ok.result), 5.);
    assert_eq!(doc.revision(), 6);
    for (arg, expected) in [
        (1., JavaScriptErrorKind::RangeError),
        (2., JavaScriptErrorKind::Error),
    ] {
        let outcome = invoke(&mut doc, 6, Atom::Ref(root), vec![Atom::Number(arg)]);
        assert!(
            matches!(outcome.result, Err(PublicError::JavaScript {name, message}) if name == expected && message.0 == [65, 0xd800, 66])
        );
        assert_eq!(count(&outcome.graph, root), 5. + arg);
        assert_eq!(doc.revision(), 6 + arg as u64);
    }
    let lossless = invoke(&mut doc, 10, Atom::Ref(root), vec![]);
    assert_eq!(count(&lossless.graph, root), 8.);
    assert_eq!(lossless.revision, 9);
    let encoded = serde_json::to_string(&lossless).unwrap();
    let decoded: InvocationOutcome = serde_json::from_str(&encoded).unwrap();
    let Err(PublicError::PeleUtf16(error)) = decoded.result else {
        panic!("lossless Pele error")
    };
    assert_eq!(error.name(), "PeleError");
    assert_eq!(error.message.0, [65, 0xd800, 66]);
    assert_eq!(error.diagram_type.unwrap().0, [0xdc00]);
    assert_eq!(error.snippet.0, [10, 0xd800]);
    assert_eq!((error.line, error.column), (7, 9));
    assert_eq!(error.code, ErrorCode::Semantic);
    let normal = PublicPeleError::from(PeleError::new("normal", ErrorCode::Semantic));
    assert_eq!(normal.well_formed().unwrap().message, "normal");
    let mut corrupted = serde_json::to_value(&lossless).unwrap();
    corrupted["result"]["Err"]["value"]["name"] = "TypeError".into();
    assert!(serde_json::from_value::<InvocationOutcome>(corrupted).is_err());
}
#[test]
fn stale_malformed_calls_and_unknown_descriptors_never_dispatch_or_mutate() {
    let m = module();
    let calls = m.calls.clone();
    let mut doc =
        PublicDocument::from_module(m, &LedgerModel { count: 7. }, Object::default()).unwrap();
    let graph = doc.snapshot().unwrap();
    let root = root_id(&graph);
    let before = fingerprint(&graph);
    for call in [
        MethodCall {
            method: MethodId(2),
            receiver: Atom::Ref(u32::MAX),
            arguments: vec![],
        },
        MethodCall {
            method: MethodId(2),
            receiver: Atom::Ref(root),
            arguments: vec![Atom::String(u32::MAX)],
        },
        MethodCall {
            method: MethodId(u32::MAX),
            receiver: Atom::Ref(root),
            arguments: vec![],
        },
        MethodCall {
            method: MethodId(2),
            receiver: Atom::Ref(root),
            arguments: vec![Atom::BuiltinFunction(u32::MAX)],
        },
        MethodCall {
            method: MethodId(2),
            receiver: Atom::Ref(root),
            arguments: vec![Atom::Undefined; 1_000_001],
        },
    ] {
        assert!(doc.invoke(0, call, &mut MetricsResources).is_err());
        assert_eq!(doc.revision(), 0);
        assert_eq!(before, fingerprint(&doc.snapshot().unwrap()))
    }
    assert!(doc
        .invoke(
            5,
            MethodCall {
                method: MethodId(2),
                receiver: Atom::Ref(root),
                arguments: vec![]
            },
            &mut MetricsResources
        )
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut bad = graph.clone();
    if let Node::Instance { prototype, .. } = &mut bad.nodes[root as usize] {
        *prototype = PrototypeId(999)
    }
    assert!(doc.apply(0, bad).is_err());
    assert_eq!(doc.revision(), 0);
    let mut bad = graph;
    set(&mut bad, root, "getSelf", Atom::BuiltinFunction(999));
    assert!(doc.apply(0, bad).is_err());
    assert_eq!(doc.revision(), 0);
    assert_eq!(before, fingerprint(&doc.snapshot().unwrap()));
    let mut legacy = PublicDocument::from_module(
        diagrams::info::Info,
        &diagrams::info::InfoModel {
            diagram_type: DiagramType::Info,
            title: None,
            acc_title: None,
            acc_descr: None,
        },
        Object::default(),
    )
    .unwrap();
    assert_eq!(legacy.model_schema().id, "pele.opaque");
    assert!(legacy
        .invoke(
            0,
            MethodCall {
                method: MethodId(1),
                receiver: Atom::Undefined,
                arguments: vec![]
            },
            &mut MetricsResources
        )
        .is_err());
    assert_eq!(legacy.revision(), 0);
}
