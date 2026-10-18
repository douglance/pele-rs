//! Public ErDb feature parity, Pele 1.6.0 / 458322a5.
//! Pinned db.ts SHA256 3a1aa534d68c819d9c003812e6a83e6d9a8d977f6662db3e7ad78ac8044306ea.
//! Methods/descriptors probed against the returned instance. Whole SVG hashes
//! below come from real pinned native builder calls, never a second Rust renderer.
//! Constructor/callback dispatch and active lone-surrogate SVG text are explicit
//! capability gaps. Array scalar length has an optional own flags0 descriptor;
//! default flags2 is implicit. Typed from_module reconstructs private lookup
//! from current groups; a typed Vec cannot retain erased JS private history.
use pele::compat::{
    diagrams::er::{Er, ErModel},
    public::{
        Atom, InvocationOutcome, JavaScriptErrorKind, JsString, MethodCall, ModelGraph,
        ModelSchema, Node, Property, PublicDiagramModule, PublicDocument, PublicEngine,
        PublicError,
    },
    text::MetricsResources,
    *,
};
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
const SOURCE: &str = "erDiagram\nA";
fn reference(value: &Atom) -> u32 {
    if let Atom::Ref(id) = value {
        *id
    } else {
        panic!("object reference expected")
    }
}
fn string(g: &mut ModelGraph, text: &str) -> Atom {
    let id = g.strings.len() as u32;
    g.strings.push(JsString::from(text));
    Atom::String(id)
}
fn property(g: &mut ModelGraph, id: u32, key: &str, value: Atom) {
    let existing = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key));
    if let Some(at) = existing {
        properties_mut(&mut g.nodes[id as usize])[at].value = value;
    } else {
        let key_id = g.strings.len() as u32;
        g.strings.push(JsString::from(key));
        properties_mut(&mut g.nodes[id as usize]).push(Property {
            key: key_id,
            value,
            flags: 7,
        });
    }
}
fn properties_mut(node: &mut Node) -> &mut Vec<Property> {
    match node {
        Node::Object { properties }
        | Node::Instance { properties, .. }
        | Node::Array { properties, .. }
        | Node::Map { properties, .. }
        | Node::Set { properties, .. }
        | Node::Date { properties, .. } => properties,
    }
}
fn field<'a>(g: &'a ModelGraph, id: u32, key: &str) -> &'a Atom {
    &g.nodes[id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(key))
        .unwrap()
        .value
}
fn root(g: &ModelGraph) -> u32 {
    reference(&g.root)
}
fn array(g: &ModelGraph, id: u32) -> Vec<Atom> {
    let Node::Array { length, properties } = &g.nodes[id as usize] else {
        panic!()
    };
    (0..*length)
        .map(|i| {
            properties
                .iter()
                .find(|p| g.strings[p.key as usize].is(&i.to_string()))
                .unwrap()
                .value
                .clone()
        })
        .collect()
}
fn push_object(g: &mut ModelGraph, fields: Vec<(&str, Atom)>) -> u32 {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (k, v) in fields {
        property(g, id, k, v)
    }
    id
}
fn push_array(g: &mut ModelGraph, values: Vec<Atom>) -> u32 {
    let id = g.nodes.len() as u32;
    let length = values.len() as u32;
    g.nodes.push(Node::Array {
        length,
        properties: vec![],
    });
    for (i, v) in values.into_iter().enumerate() {
        property(g, id, &i.to_string(), v)
    }
    id
}
struct Counted(Arc<AtomicUsize>);
impl DiagramModule for Counted {
    type Model = ErModel;
    fn descriptor(&self) -> ModuleDescriptor {
        Er.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Er.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Er.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Counted {
    type PublicState = <Er as PublicDiagramModule>::PublicState;
    fn model_schema(&self) -> &'static ModelSchema {
        Er.model_schema()
    }
    fn invoke_public(
        &self,
        s: &mut Self::PublicState,
        call: MethodCall,
        c: &Object,
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        Er.invoke_public(s, call, c, h)
    }
    fn create_public(&self, m: &ErModel) -> Result<Self::PublicState, PeleError> {
        Er.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Er.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        Er.apply_public(s, g)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Er.render_public(s, c, o, h)
    }
}
fn parsed(source: &str) -> PublicDocument {
    let mut e = PublicEngine::empty();
    e.register(Er);
    e.parse(source, &Default::default()).unwrap()
}
fn mapped(g: &ModelGraph, map: u32, name: &str) -> Atom {
    let Node::Map { entries, .. } = &g.nodes[map as usize] else {
        panic!()
    };
    entries
        .iter()
        .find(|(k, _)| matches!(k,Atom::String(id) if g.strings[*id as usize].is(name)))
        .unwrap()
        .1
        .clone()
}
fn entity(g: &ModelGraph, name: &str) -> u32 {
    reference(&mapped(g, reference(field(g, root(g), "entities")), name))
}
fn keys(g: &ModelGraph, id: u32) -> Vec<String> {
    g.nodes[id as usize]
        .properties()
        .iter()
        .map(|p| g.strings[p.key as usize].well_formed().unwrap())
        .collect()
}
fn texts_of(g: &ModelGraph, id: u32) -> Vec<String> {
    array(g, id)
        .iter()
        .map(|a| g.text(a).unwrap().well_formed().unwrap())
        .collect()
}
fn method(doc: &mut PublicDocument, name: &str, args: Vec<Atom>) -> InvocationOutcome {
    let method = doc
        .model_schema()
        .methods
        .iter()
        .find(|m| m.name == name)
        .unwrap()
        .id;
    let receiver = doc.snapshot().unwrap().root;
    doc.invoke(
        doc.revision(),
        MethodCall {
            method,
            receiver,
            arguments: args,
        },
        &mut MetricsResources,
    )
    .unwrap()
}
fn apply_args(
    doc: &mut PublicDocument,
    make: impl FnOnce(&mut ModelGraph) -> Vec<Atom>,
) -> Vec<Atom> {
    let mut g = doc.snapshot().unwrap();
    let args = make(&mut g);
    doc.apply(doc.revision(), g).unwrap();
    args
}
fn invoke(
    doc: &mut PublicDocument,
    name: &str,
    make: impl FnOnce(&mut ModelGraph) -> Vec<Atom>,
) -> InvocationOutcome {
    let args = apply_args(doc, make);
    method(doc, name, args)
}
fn svg(doc: &mut PublicDocument) -> Rendered {
    doc.render(&Default::default(), &mut MetricsResources)
        .unwrap()
        .rendered
}
fn hash(doc: &mut PublicDocument, expected: &str, w: f64, h: f64) {
    let out = svg(doc);
    assert_eq!(
        format!("{:x}", Sha256::digest(out.svg.as_bytes())),
        expected
    );
    assert_eq!((out.width, out.height), (w, h))
}
fn same_source(doc: &mut PublicDocument, source: &str) {
    assert_eq!(
        svg(doc),
        Engine::with_builtins()
            .render_source(source, &Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered
    )
}
fn is_type_error(out: &InvocationOutcome, text: &str) {
    match &out.result {
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            message,
        }) => assert_eq!(message.well_formed().as_deref(), Some(text)),
        other => panic!("expected TypeError: {other:?}"),
    }
}
fn delete(g: &mut ModelGraph, id: u32, key: &str) {
    let position = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key));
    if let Some(n) = position {
        g.nodes[id as usize].properties_mut().remove(n);
    }
}
#[test]
fn er_source_own_order_private_aliases_and_mutable_maps() {
    let source="erDiagram\nA { string absent\nstring empty \"\"\nstring comment \"note\"\nint keyed PK\nint both FK \"\" }\nsubgraph title1\nCUSTOMER\nend\nsubgraph title2\nCAR ||--o{ NAMED-DRIVER : allows\nsubgraph title3\nPERSON\nend\nend";
    let mut doc = parsed(source);
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    assert_eq!(
        keys(&g, r),
        [
            "type",
            "direction",
            "entities",
            "relationships",
            "classes",
            "subgraphs",
            "title",
            "accTitle",
            "accDescr",
            "subgraphLookup",
            "claimed"
        ]
    );
    assert!(g.nodes[r as usize]
        .properties()
        .iter()
        .all(|p| p.flags == 7));
    assert!(matches!(field(&g, r, "title"), Atom::Undefined));
    assert_eq!(doc.model_schema().id, "pele/er@1");
    assert_eq!(
        doc.model_schema()
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        [
            ("addEntity", 1),
            ("addAttributes", 2),
            ("addRelationship", 4),
            ("setDirection", 1),
            ("addCssStyles", 2),
            ("addClass", 2),
            ("setClass", 2),
            ("addSubGraph", 3),
            ("setAccTitle", 1),
            ("setAccDescription", 1)
        ]
    );
    assert!(doc.model_schema().prototypes[0]
        .members
        .iter()
        .all(|m| m.flags == 6));
    let attrs = array(&g, reference(field(&g, entity(&g, "A"), "attributes")));
    for (n, attr) in attrs.iter().enumerate() {
        assert_eq!(
            keys(&g, reference(attr)),
            if n == 1 || n == 2 {
                vec!["type", "name", "comment", "keys"]
            } else {
                vec!["type", "name", "keys", "comment"]
            }
        );
    }
    let groups = array(&g, reference(field(&g, r, "subgraphs")));
    let lookup = reference(field(&g, r, "subgraphLookup"));
    for (key, group) in ["title1", "title3", "title2"].iter().zip(&groups) {
        assert_eq!(reference(&mapped(&g, lookup, key)), reference(group));
        assert_eq!(
            keys(&g, reference(group)),
            [
                "id",
                "nodes",
                "title",
                "classes",
                "cssStyles",
                "dir",
                "labelType"
            ]
        );
        assert!(matches!(
            field(&g, reference(group), "dir"),
            Atom::Undefined
        ));
    }
    let Node::Set { values, .. } = &g.nodes[reference(field(&g, r, "claimed")) as usize] else {
        panic!()
    };
    assert_eq!(
        values
            .iter()
            .map(|a| g.text(a).unwrap().well_formed().unwrap())
            .collect::<Vec<_>>(),
        ["CUSTOMER", "PERSON", "CAR", "NAMED-DRIVER", "title3"]
    );
    same_source(&mut doc, source);
    // Extra cyclic objects, nonfinite bits, raw UTF16 and Map replacement survive
    // publication; rendering only traverses actual drawable fields.
    let extra = push_object(&mut g, vec![]);
    property(&mut g, extra, "cycle", Atom::Ref(extra));
    property(
        &mut g,
        extra,
        "nan",
        Atom::Number(f64::from_bits(0x7ff8_0000_0000_0042)),
    );
    let lone_id = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 97, 0xdc00]));
    property(&mut g, extra, "raw", Atom::String(lone_id));
    property(&mut g, r, "extra", Atom::Ref(extra));
    let title = string(&mut g, "Changed");
    property(&mut g, r, "title", title);
    delete(&mut g, r, "accDescr");
    let before = serde_json::to_string(&g).unwrap();
    doc.apply(doc.revision(), g).unwrap();
    assert_eq!(
        serde_json::to_string(&doc.snapshot().unwrap()).unwrap(),
        before
    );
    assert!(svg(&mut doc).svg.contains("Changed"));
    let mut simple = parsed("erDiagram\nA\nB");
    let mut g = simple.snapshot().unwrap();
    let map = reference(field(&g, root(&g), "entities"));
    let a = entity(&g, "A");
    let b = entity(&g, "B");
    let new_name = string(&mut g, "Bee");
    property(&mut g, b, "alias", new_name);
    let replacement = push_object(&mut g, vec![]);
    g.nodes[replacement as usize] = g.nodes[a as usize].clone();
    let key_a = string(&mut g, "A");
    let key_b = string(&mut g, "B");
    g.nodes[map as usize] = Node::Map {
        entries: vec![(key_b, Atom::Ref(b)), (key_a, Atom::Ref(replacement))],
        properties: vec![],
    };
    simple.apply(simple.revision(), g).unwrap();
    let retained = simple.snapshot().unwrap();
    assert_eq!(entity(&retained, "A"), replacement);
    // Existing entity IDs are retained despite changed Map insertion order.
    let mut immutable = Engine::with_builtins()
        .parse("erDiagram\nB[Bee]\nA", &Default::default())
        .unwrap()
        .model::<ErModel>()
        .unwrap()
        .clone();
    immutable.entities[0].1.id = "entity-B-1".into();
    immutable.entities[1].1.id = "entity-A-0".into();
    assert_eq!(
        svg(&mut simple),
        Er.render(
            &immutable,
            &Object::default(),
            &RenderSettings::default(),
            &mut MetricsResources
        )
        .unwrap()
    );
}
#[test]
fn er_all_real_methods_match_pinned_whole_svg_and_argument_aliases() {
    let mut doc = parsed(SOURCE);
    let b = invoke(&mut doc, "addEntity", |g| {
        vec![string(g, "B"), string(g, "Bee")]
    });
    assert!(b.result.is_ok());
    let mut argument_attrs = vec![];
    let attrs = invoke(&mut doc, "addAttributes", |g| {
        let last_type = string(g, "string");
        let last_name = string(g, "last");
        let last = push_object(g, vec![("type", last_type), ("name", last_name)]);
        let first_type = string(g, "int");
        let first_name = string(g, "first");
        let pk = string(g, "PK");
        let pk = push_array(g, vec![pk]);
        let comment = string(g, "id");
        let first = push_object(
            g,
            vec![
                ("type", first_type),
                ("name", first_name),
                ("keys", Atom::Ref(pk)),
                ("comment", comment),
            ],
        );
        argument_attrs = vec![first, last];
        let a = push_array(g, vec![Atom::Ref(last), Atom::Ref(first)]);
        vec![string(g, "A"), Atom::Ref(a)]
    });
    assert!(attrs.result.is_ok());
    let mut spec_id = 0;
    assert!(invoke(&mut doc, "addRelationship", |g| {
        let a = string(g, "ZERO_OR_MORE");
        let b = string(g, "ONLY_ONE");
        let t = string(g, "IDENTIFYING");
        spec_id = push_object(g, vec![("cardA", a), ("relType", t), ("cardB", b)]);
        vec![
            string(g, "A"),
            string(g, "owns"),
            string(g, "B"),
            Atom::Ref(spec_id),
        ]
    })
    .result
    .is_ok());
    assert!(invoke(&mut doc, "setDirection", |g| vec![string(g, "LR")])
        .result
        .is_ok());
    let g = doc.snapshot().unwrap();
    let a = entity(&g, "A");
    assert_eq!(
        array(&g, reference(field(&g, a, "attributes")))
            .iter()
            .map(reference)
            .collect::<Vec<_>>(),
        argument_attrs
    );
    let rel = array(&g, reference(field(&g, root(&g), "relationships")));
    assert_eq!(reference(field(&g, reference(&rel[0]), "relSpec")), spec_id);
    hash(
        &mut doc,
        "fe43d187e548a79e6a10da669325d555e3de23da60798a034c3bc71ea67ce5a4",
        377.,
        118.,
    );
    assert!(invoke(&mut doc, "addClass", |g| {
        let hot = string(g, "hot");
        let ids = push_array(g, vec![hot]);
        let values = ["fill:#abc", "color:#123", "font-weight:bold"]
            .map(|s| string(g, s))
            .to_vec();
        let styles = push_array(g, values);
        vec![Atom::Ref(ids), Atom::Ref(styles)]
    })
    .result
    .is_ok());
    assert!(invoke(&mut doc, "setClass", |g| {
        let a = string(g, "A");
        let hot = string(g, "hot");
        let ids = push_array(g, vec![a]);
        let names = push_array(g, vec![hot]);
        vec![Atom::Ref(ids), Atom::Ref(names)]
    })
    .result
    .is_ok());
    assert!(invoke(&mut doc, "addCssStyles", |g| {
        let b = string(g, "B");
        let style = string(g, "stroke:#f00");
        let ids = push_array(g, vec![b]);
        let styles = push_array(g, vec![style]);
        vec![Atom::Ref(ids), Atom::Ref(styles)]
    })
    .result
    .is_ok());
    hash(
        &mut doc,
        "8c321b20fe2cf06281fe31dff644b8c54c664954d4bdbe598ce85d255dc1e014",
        388.,
        118.,
    );
    let group = invoke(&mut doc, "addSubGraph", |g| {
        let id = string(g, " g ");
        let id = push_object(g, vec![("text", id)]);
        let a = string(g, "A");
        let nested = push_array(g, vec![a]);
        let stmt = string(g, "dir");
        let value = string(g, "TB");
        let dir = push_object(g, vec![("stmt", stmt), ("value", value)]);
        let b = string(g, "B");
        let duplicate = string(g, " A ");
        let list = push_array(g, vec![Atom::Ref(nested), Atom::Ref(dir), b, duplicate]);
        let title = string(g, " Group ");
        let ty = string(g, "text");
        let title = push_object(g, vec![("text", title), ("type", ty)]);
        vec![Atom::Ref(id), Atom::Ref(list), Atom::Ref(title)]
    });
    assert!(
        matches!(group.result,Ok(Atom::String(id)) if group.graph.strings[id as usize].is("g"))
    );
    assert!(invoke(&mut doc, "setClass", |g| {
        let id = string(g, "g");
        let hot = string(g, "hot");
        let ids = push_array(g, vec![id]);
        let names = push_array(g, vec![hot]);
        vec![Atom::Ref(ids), Atom::Ref(names)]
    })
    .result
    .is_ok());
    assert!(invoke(&mut doc, "addCssStyles", |g| {
        let id = string(g, "g");
        let style = string(g, "stroke:#369");
        let ids = push_array(g, vec![id]);
        let styles = push_array(g, vec![style]);
        vec![Atom::Ref(ids), Atom::Ref(styles)]
    })
    .result
    .is_ok());
    assert!(invoke(&mut doc, "setAccTitle", |g| vec![string(
        g,
        "  Accessible  "
    )])
    .result
    .is_ok());
    assert!(invoke(&mut doc, "setAccDescription", |g| vec![string(
        g,
        "first\n   second\n\tthird"
    )])
    .result
    .is_ok());
    hash(
        &mut doc,
        "df8018bb13abe0df66072a712b1057a395221feee7c62ab9b9edc1be8c8bee1c",
        223.,
        315.,
    );
    // The passed relSpec stays live after method completion.
    let mut g = doc.snapshot().unwrap();
    let ty = string(&mut g, "NON_IDENTIFYING");
    property(&mut g, spec_id, "relType", ty);
    doc.apply(doc.revision(), g).unwrap();
    assert_ne!(
        format!("{:x}", Sha256::digest(svg(&mut doc).svg.as_bytes())),
        "df8018bb13abe0df66072a712b1057a395221feee7c62ab9b9edc1be8c8bee1c"
    );
}
#[test]
fn er_partial_errors_publish_real_mutation_order_and_recover() {
    let mut doc = parsed(SOURCE);
    let revision = doc.revision();
    let out = invoke(&mut doc, "addAttributes", |g| {
        let ty = string(g, "int");
        let name = string(g, "ok");
        let attr = push_object(g, vec![("type", ty), ("name", name)]);
        let attrs = push_array(g, vec![Atom::Null, Atom::Ref(attr)]);
        vec![string(g, "B"), Atom::Ref(attrs)]
    });
    is_type_error(&out, "Cannot read properties of null (reading 'keys')");
    assert!(out.revision > revision);
    let b = entity(&out.graph, "B");
    let attrs = array(&out.graph, reference(field(&out.graph, b, "attributes")));
    assert_eq!(attrs.len(), 1);
    assert_eq!(
        keys(&out.graph, reference(&attrs[0])),
        ["type", "name", "keys", "comment"]
    );
    same_source(&mut doc, "erDiagram\nA\nB { int ok }");
    let out = invoke(&mut doc, "addClass", |g| {
        let id = string(g, "x");
        let ids = push_array(g, vec![id]);
        let s = string(g, "color:red");
        let styles = push_array(g, vec![s, Atom::Null]);
        vec![Atom::Ref(ids), Atom::Ref(styles)]
    });
    // A class record and its first style precede the failure in the second style.
    is_type_error(&out, "Cannot read properties of null (reading 'includes')");
    let class = mapped(
        &out.graph,
        reference(field(&out.graph, root(&out.graph), "classes")),
        "x",
    );
    assert_eq!(
        texts_of(
            &out.graph,
            reference(field(&out.graph, reference(&class), "styles"))
        ),
        ["color:red"]
    );
    assert_eq!(
        texts_of(
            &out.graph,
            reference(field(&out.graph, reference(&class), "textStyles"))
        ),
        ["color:red"]
    );
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    property(&mut g, r, "relationships", Atom::Null);
    doc.apply(doc.revision(), g).unwrap();
    let out = invoke(&mut doc, "addRelationship", |g| {
        vec![
            string(g, "A"),
            string(g, "owns"),
            string(g, "C"),
            Atom::Null,
        ]
    });
    is_type_error(&out, "Cannot read properties of null (reading 'push')");
    assert_eq!(
        out.graph
            .text(field(&out.graph, entity(&out.graph, "C"), "id"))
            .unwrap()
            .well_formed()
            .as_deref(),
        Some("entity-C-2")
    );
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let relationships = push_array(&mut g, vec![]);
    property(&mut g, r, "relationships", Atom::Ref(relationships));
    doc.apply(doc.revision(), g).unwrap();
    same_source(&mut doc, "erDiagram\nA\nB { int ok }\nC");
    // A readonly destination length still allows defaulting the argument record.
    let mut g = doc.snapshot().unwrap();
    let b = entity(&g, "B");
    let attrs = reference(field(&g, b, "attributes"));
    let length = string(&mut g, "length");
    let Atom::String(length) = length else {
        panic!()
    };
    g.nodes[attrs as usize].properties_mut().push(Property {
        key: length,
        value: Atom::Number(1.),
        flags: 0,
    });
    doc.apply(doc.revision(), g).unwrap();
    let mut input = 0;
    let out = invoke(&mut doc, "addAttributes", |g| {
        let ty = string(g, "int");
        let name = string(g, "blocked");
        input = push_object(g, vec![("type", ty), ("name", name)]);
        let attrs = push_array(g, vec![Atom::Ref(input)]);
        vec![string(g, "B"), Atom::Ref(attrs)]
    });
    is_type_error(
        &out,
        "Cannot assign to read only property 'length' of object '[object Array]'",
    );
    assert_eq!(keys(&out.graph, input), ["type", "name", "keys", "comment"]);
    assert_eq!(array(&out.graph, attrs).len(), 1);
}
#[test]
fn er_group_claims_private_cache_and_duplicate_id_boundaries() {
    let mut doc = parsed(SOURCE);
    let out = invoke(&mut doc, "addSubGraph", |g| {
        let id = string(g, " g ");
        let id = push_object(g, vec![("text", id)]);
        let a = string(g, "A");
        let list = push_array(g, vec![a]);
        vec![Atom::Ref(id), Atom::Ref(list), Atom::Null]
    });
    is_type_error(&out, "Cannot read properties of null (reading 'text')");
    let claimed = reference(field(&out.graph, root(&out.graph), "claimed"));
    let Node::Set { values, .. } = &out.graph.nodes[claimed as usize] else {
        panic!()
    };
    assert_eq!(values.len(), 1);
    assert!(array(
        &out.graph,
        reference(field(&out.graph, root(&out.graph), "subgraphs"))
    )
    .is_empty());
    let make_group = |g: &mut ModelGraph, title: &str| {
        let id = string(g, "g");
        let id = push_object(g, vec![("text", id)]);
        let a = string(g, "A");
        let list = push_array(g, vec![a]);
        let title = string(g, title);
        let title = push_object(g, vec![("text", title)]);
        vec![Atom::Ref(id), Atom::Ref(list), Atom::Ref(title)]
    };
    assert!(invoke(&mut doc, "addSubGraph", |g| make_group(g, "First"))
        .result
        .is_ok());
    assert!(invoke(&mut doc, "addSubGraph", |g| make_group(g, "Second"))
        .result
        .is_ok());
    let g = doc.snapshot().unwrap();
    let groups = array(&g, reference(field(&g, root(&g), "subgraphs")));
    let first = reference(&groups[0]);
    let second = reference(&groups[1]);
    let lookup = reference(field(&g, root(&g), "subgraphLookup"));
    assert_eq!(reference(&mapped(&g, lookup, "g")), second);
    assert!(array(&g, reference(field(&g, first, "nodes"))).is_empty());
    assert!(array(&g, reference(field(&g, second, "nodes"))).is_empty());
    assert!(invoke(&mut doc, "setClass", |g| {
        let id = string(g, "g");
        let class = string(g, "latest");
        let ids = push_array(g, vec![id]);
        let names = push_array(g, vec![class]);
        vec![Atom::Ref(ids), Atom::Ref(names)]
    })
    .result
    .is_ok());
    let g = doc.snapshot().unwrap();
    assert!(array(&g, reference(field(&g, first, "classes"))).is_empty());
    assert_eq!(
        texts_of(&g, reference(field(&g, second, "classes"))),
        ["latest"]
    );
    // Replacing the public group array never repairs or retargets private lookup.
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let empty = push_array(&mut g, vec![]);
    property(&mut g, r, "subgraphs", Atom::Ref(empty));
    property(&mut g, r, "subgraphLookup", Atom::Null);
    doc.apply(doc.revision(), g).unwrap();
    let out = invoke(&mut doc, "addSubGraph", |g| {
        make_group(g, "Published first")
    });
    is_type_error(&out, "Cannot read properties of null (reading 'set')");
    assert_eq!(array(&out.graph, empty).len(), 1);
    let mut g = out.graph;
    let r = root(&g);
    let map = g.nodes.len() as u32;
    g.nodes.push(Node::Map {
        entries: vec![],
        properties: vec![],
    });
    property(&mut g, r, "subgraphLookup", Atom::Ref(map));
    doc.apply(doc.revision(), g).unwrap();
    assert!(svg(&mut doc).svg.contains("Published first"));
}
#[test]
fn er_borrowed_receivers_utf16_identity_and_readonly_properties() {
    let mut doc = parsed(SOURCE);
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let map = field(&g, r, "entities").clone();
    let borrowed = push_object(&mut g, vec![("entities", map)]);
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    doc.apply(doc.revision(), g).unwrap();
    let add = doc.model_schema().methods[0].id;
    let out = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: add,
                receiver: Atom::Ref(borrowed),
                arguments: vec![Atom::String(raw)],
            },
            &mut MetricsResources,
        )
        .unwrap();
    let object = reference(out.result.as_ref().unwrap());
    assert_eq!(
        out.graph
            .text(field(&out.graph, object, "label"))
            .unwrap()
            .0,
        [0xd800, 65, 0xdc00]
    );
    let mut expected = "entity-".encode_utf16().collect::<Vec<_>>();
    expected.extend([0xd800, 65, 0xdc00]);
    expected.extend("-1".encode_utf16());
    assert_eq!(
        out.graph.text(field(&out.graph, object, "id")).unwrap().0,
        expected
    );
    let out = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: add,
                receiver: Atom::Ref(borrowed),
                arguments: vec![Atom::String(raw)],
            },
            &mut MetricsResources,
        )
        .unwrap();
    assert_eq!(reference(out.result.as_ref().unwrap()), object);
    let result = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap_err();
    assert!(result.message.contains("lone UTF-16"));
    let mut g = doc.snapshot().unwrap();
    let entities = reference(field(&g, r, "entities"));
    let Node::Map { entries, .. } = &mut g.nodes[entities as usize] else {
        panic!()
    };
    entries.pop();
    doc.apply(doc.revision(), g).unwrap();
    same_source(&mut doc, SOURCE);
    let args = apply_args(&mut doc, |g| {
        let list = push_array(g, vec![]);
        vec![string(g, "A"), Atom::Ref(list)]
    });
    let attr_method = doc.model_schema().methods[1].id;
    let out = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: attr_method,
                receiver: Atom::Ref(borrowed),
                arguments: args.clone(),
            },
            &mut MetricsResources,
        )
        .unwrap();
    is_type_error(&out, "this.addEntity is not a function");
    let mut g = doc.snapshot().unwrap();
    property(&mut g, borrowed, "addEntity", Atom::BuiltinFunction(add.0));
    doc.apply(doc.revision(), g).unwrap();
    assert!(doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: attr_method,
                receiver: Atom::Ref(borrowed),
                arguments: args
            },
            &mut MetricsResources
        )
        .unwrap()
        .result
        .is_ok());
    let mut g = doc.snapshot().unwrap();
    for p in g.nodes[r as usize].properties_mut() {
        if out
            .graph
            .strings
            .get(p.key as usize)
            .is_some_and(|s| s.is("direction"))
        {
            p.flags = 5;
        }
    }
    doc.apply(doc.revision(), g).unwrap();
    let out = invoke(&mut doc, "setDirection", |g| vec![string(g, "LR")]);
    assert!(matches!(
        out.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert!(out
        .graph
        .text(field(&out.graph, r, "direction"))
        .unwrap()
        .is("TB"));
    let raw = apply_args(&mut doc, |g| {
        let id = g.strings.len() as u32;
        g.strings.push(JsString(vec![32, 0xd800, 32, 32]));
        vec![Atom::String(id)]
    });
    assert!(method(&mut doc, "setAccTitle", raw).result.is_ok());
    let g = doc.snapshot().unwrap();
    assert_eq!(
        g.text(field(&g, r, "accTitle")).unwrap().0,
        [0xd800, 32, 32]
    );
    let raw = apply_args(&mut doc, |g| {
        let id = g.strings.len() as u32;
        g.strings
            .push(JsString(vec![65, 10, 32, 10, 9, 0xdc00, 32]));
        vec![Atom::String(id)]
    });
    assert!(method(&mut doc, "setAccDescription", raw).result.is_ok());
    let g = doc.snapshot().unwrap();
    assert_eq!(
        g.text(field(&g, r, "accDescr")).unwrap().0,
        [65, 10, 0xdc00, 32]
    );
}
#[test]
fn er_retained_registration_resources_output_and_deep_graph_recovery() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut e = PublicEngine::empty();
    e.register(Counted(count.clone()));
    let mut doc = e.parse(SOURCE, &Default::default()).unwrap();
    e.register(Er);
    drop(e);
    assert!(invoke(&mut doc, "addEntity", |g| vec![
        string(g, "B"),
        string(g, "Bee")
    ])
    .result
    .is_ok());
    same_source(&mut doc, "erDiagram\nA\nB[Bee]");
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let stable = serde_json::to_string(&doc.snapshot().unwrap()).unwrap();
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(32);
    assert_eq!(
        doc.render(&options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(
        serde_json::to_string(&doc.snapshot().unwrap()).unwrap(),
        stable
    );
    struct Broken;
    impl RenderResources for Broken {
        fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
            Err(PeleError::new("host metric failed", ErrorCode::Semantic))
        }
    }
    assert_eq!(
        doc.render(&Default::default(), &mut Broken)
            .unwrap_err()
            .message,
        "host metric failed"
    );
    same_source(&mut doc, "erDiagram\nA\nB[Bee]");
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let mut prev = r;
    for _ in 0..6000 {
        let id = push_object(&mut g, vec![("next", Atom::Ref(prev))]);
        prev = id;
    }
    property(&mut g, r, "deep", Atom::Ref(prev));
    let json = serde_json::to_string(&g).unwrap();
    let graph: ModelGraph = serde_json::from_str(&json).unwrap();
    doc.apply(doc.revision(), graph).unwrap();
    assert_eq!(
        serde_json::to_string(&doc.snapshot().unwrap()).unwrap(),
        json
    );
    same_source(&mut doc, "erDiagram\nA\nB[Bee]");
    // Malformed/stale invocation is rejected before any method mutation.
    let before = serde_json::to_string(&doc.snapshot().unwrap()).unwrap();
    let rev = doc.revision();
    let call = MethodCall {
        method: doc.model_schema().methods[0].id,
        receiver: Atom::Ref(r),
        arguments: vec![Atom::Ref(u32::MAX)],
    };
    assert!(doc.invoke(rev, call, &mut MetricsResources).is_err());
    assert_eq!(doc.revision(), rev);
    assert_eq!(
        serde_json::to_string(&doc.snapshot().unwrap()).unwrap(),
        before
    );
    let args = apply_args(&mut doc, |g| vec![string(g, "C")]);
    let call = MethodCall {
        method: doc.model_schema().methods[0].id,
        receiver: Atom::Ref(r),
        arguments: args,
    };
    assert!(doc.invoke(rev, call, &mut MetricsResources).is_err());
    assert_eq!(count.load(Ordering::SeqCst), 1);
}
#[test]
fn er_public_source_corpus_preserves_complete_models_svg_options_and_errors() {
    use serde_json::Value;
    // Only the acyclic, fully authored oracle documents enter this JSON view.
    // Mutated cyclic/UTF16/IEEE graphs are tested losslessly in the other slices.
    fn project(g: &ModelGraph, a: &Atom, root: bool) -> Value {
        match a {
            Atom::Undefined | Atom::Null => Value::Null,
            Atom::Bool(v) => (*v).into(),
            Atom::Number(v) => serde_json::json!(v),
            Atom::String(id) => g.strings[*id as usize].well_formed().unwrap().into(),
            Atom::Ref(id) => match &g.nodes[*id as usize] {
                Node::Map { entries, .. } => entries
                    .iter()
                    .map(|(k, v)| Value::Array(vec![project(g, k, false), project(g, v, false)]))
                    .collect(),
                Node::Array { length, properties } => {
                    let mut values = vec![Value::Null; *length as usize];
                    for p in properties {
                        if let Some(index) = g.strings[p.key as usize]
                            .well_formed()
                            .and_then(|s| s.parse::<usize>().ok())
                        {
                            if index < values.len() {
                                values[index] = project(g, &p.value, false)
                            }
                        }
                    }
                    Value::Array(values)
                }
                Node::Set { values, .. } => values.iter().map(|a| project(g, a, false)).collect(),
                node => {
                    let mut object = serde_json::Map::new();
                    for p in node.properties() {
                        let key = g.strings[p.key as usize].well_formed().unwrap();
                        if matches!(p.value, Atom::Undefined)
                            || root && ["subgraphLookup", "claimed"].contains(&key.as_str())
                        {
                            continue;
                        }
                        object.insert(key, project(g, &p.value, false));
                    }
                    Value::Object(object)
                }
            },
            _ => panic!("source corpus has no callable or host-reference fields"),
        }
    }
    struct Host<'a>(&'a Value);
    impl RenderResources for Host<'_> {
        fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
            if self.0["measure"] == "fixed" {
                Ok(r.text.encode_utf16().count() as f64
                    * r.size
                    * if r.style & 1 != 0 { 0.71 } else { 0.6 })
            } else {
                MetricsResources.text_width(r)
            }
        }
        fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
            Ok(self.0["icons"][name]
                .as_str()
                .map(|s| TrustedSvg::from_trusted_markup(s.into())))
        }
    }
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/er.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut e = PublicEngine::empty();
    e.register(Er);
    let mut failures = vec![];
    for case in cases {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let expected = &case["expected"];
        let mut doc = match e.parse(case["source"].as_str().unwrap(), &options.parse) {
            Ok(doc) => doc,
            Err(error) => {
                if expected.get("model").is_some()
                    || serde_json::to_value(error).unwrap() != expected["error"]
                {
                    failures.push(format!("{}:parse", case["id"]));
                }
                continue;
            }
        };
        let graph = doc.snapshot().unwrap();
        if project(&graph, &graph.root, true) != expected["model"] {
            failures.push(format!("{}:model", case["id"]));
        }
        let actual = match doc.render(&options, &mut Host(&case["host"])) {
            Ok(rendered) => {
                let mut value = serde_json::to_value(rendered).unwrap();
                if expected["result"].get("svgSha256").is_some() {
                    let svg = value.as_object_mut().unwrap().remove("svg").unwrap();
                    value["svgSha256"] =
                        format!("{:x}", Sha256::digest(svg.as_str().unwrap().as_bytes())).into();
                }
                value
            }
            Err(error) => serde_json::json!({"error":error}),
        };
        let expected = if expected.get("error").is_some() {
            serde_json::json!({"error":expected["error"]})
        } else {
            expected["result"].clone()
        };
        if actual != expected {
            if failures.len() < 5 {
                eprintln!(
                    "{}:render actual={} expected={}",
                    case["id"], actual, expected
                )
            }
            failures.push(format!("{}:render", case["id"]));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures / {} cases: {:?}",
        failures.len(),
        cases.len(),
        failures.get(..failures.len().min(12)).unwrap()
    );
}
#[test]
fn er_live_alias_iteration_and_import_expansion_have_recoverable_budgets() {
    let mut doc = parsed(SOURCE);
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let e = entity(&g, "A");
    let css = reference(field(&g, e, "cssStyles"));
    let style = string(&mut g, "fill:#abc");
    property(&mut g, css, "0", style);
    if let Node::Array { length, .. } = &mut g.nodes[css as usize] {
        *length = 1
    }
    let a = string(&mut g, "A");
    let ids = push_array(&mut g, vec![a]);
    doc.apply(doc.revision(), g).unwrap();
    // The exact native for-of is live: appending styles into the same styles array
    // keeps growing it. A finite copied input would incorrectly report success.
    let out = method(
        &mut doc,
        "addCssStyles",
        vec![Atom::Ref(ids), Atom::Ref(css)],
    );
    assert!(matches!(
        out.result,
        Err(PublicError::Pele(PeleError {
            code: ErrorCode::Limit,
            ..
        }))
    ));
    let grown = array(&out.graph, css);
    assert!(grown.len() > 1);
    let mut g = out.graph;
    let empty = push_array(&mut g, vec![]);
    property(&mut g, e, "cssStyles", Atom::Ref(empty));
    doc.apply(doc.revision(), g).unwrap();
    same_source(&mut doc, SOURCE);
    // An otherwise compact arena can repeat one large attribute record thousands
    // of times. Bound the concrete typed model before cloning all that text.
    let mut g = doc.snapshot().unwrap();
    let ty = string(&mut g, &"x".repeat(65536));
    let name = string(&mut g, "v");
    let keys = push_array(&mut g, vec![]);
    let comment = string(&mut g, "");
    let attr = push_object(
        &mut g,
        vec![
            ("type", ty),
            ("name", name),
            ("keys", Atom::Ref(keys)),
            ("comment", comment),
        ],
    );
    let attrs = push_array(&mut g, vec![Atom::Ref(attr); 1024]);
    property(&mut g, e, "attributes", Atom::Ref(attrs));
    doc.apply(doc.revision(), g).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let mut g = doc.snapshot().unwrap();
    let empty = push_array(&mut g, vec![]);
    property(&mut g, e, "attributes", Atom::Ref(empty));
    doc.apply(doc.revision(), g).unwrap();
    assert_eq!(root(&doc.snapshot().unwrap()), r);
    same_source(&mut doc, SOURCE);
    // A single style shared by thousands of entity records is small on the
    // wire but expands during the existing renderer's class compilation.
    let mut g = doc.snapshot().unwrap();
    let class_name = string(&mut g, "big");
    let style = string(&mut g, &"x".repeat(65536));
    let styles = push_array(&mut g, vec![style]);
    let text_styles = push_array(&mut g, vec![]);
    let class = push_object(
        &mut g,
        vec![
            ("id", class_name.clone()),
            ("styles", Atom::Ref(styles)),
            ("textStyles", Atom::Ref(text_styles)),
        ],
    );
    let classes = reference(field(&g, r, "classes"));
    let Node::Map { entries, .. } = &mut g.nodes[classes as usize] else {
        panic!()
    };
    entries.push((class_name, Atom::Ref(class)));
    let name = string(&mut g, "default big");
    property(&mut g, e, "cssClasses", name);
    let entities = reference(field(&g, r, "entities"));
    for n in 0..2048 {
        let key = string(&mut g, &format!("Alias{n}"));
        let Node::Map { entries, .. } = &mut g.nodes[entities as usize] else {
            panic!()
        };
        entries.push((key, Atom::Ref(e)));
    }
    doc.apply(doc.revision(), g).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let mut g = doc.snapshot().unwrap();
    let Node::Map { entries, .. } = &mut g.nodes[entities as usize] else {
        panic!()
    };
    entries.truncate(1);
    let default = string(&mut g, "default");
    property(&mut g, e, "cssClasses", default);
    doc.apply(doc.revision(), g).unwrap();
    same_source(&mut doc, SOURCE);
}
#[test]
fn er_pinned_mutable_text_security_styles_directions_and_narrow_output() {
    let vectors = [
        (
            "plain",
            "37f930b1ceb7a1c89a4d498be0e7ec20af2b0a9c90f650cb25eb70492102eb8a",
            402.,
        ),
        (
            "**Bold** _italic_",
            "4a3aed18b90ce9b4090fd386eccf67080dbde17858ee774e87b1b56133fd8c4b",
            469.,
        ),
        (
            "<script>alert(1)</script>",
            "8bce08a5228756a06b62a781bb07ed89f6ef4905d03657524d360257c94894c7",
            432.,
        ),
        (
            "&lt;tag&gt; &amp; value",
            "91864c6e5c1c5121c56e38dc8165148953ce596f1afc2993450926716ec326f5",
            516.,
        ),
        (
            "fa:fa-user Person",
            "99f5b913810336b846e66a98d479493897f7ad57b5708239d2bdb95fb58e4ae3",
            432.,
        ),
        (
            "漢字🙂",
            "a17abbf8b3bdbc5adb08daad3441fb19af3e14c815d54a3212e0c0a4833d150c",
            427.,
        ),
        (
            "__proto__",
            "da608d297254790e7f5e09b58155021c5a0286eaf748cfe8b9896f4849487156",
            414.,
        ),
        (
            "a\"<>&'",
            "f93b67a18b48c6f8dbf0ca0188d405115d6f0521d81bd0372ea6778953e0e110",
            425.,
        ),
    ];
    for (n, (label, expected, w)) in vectors.into_iter().enumerate() {
        let mut doc = parsed("erDiagram\nA\nB");
        let name = format!("N{n}");
        assert!(invoke(&mut doc, "addEntity", |g| vec![
            string(g, &name),
            string(g, label)
        ])
        .result
        .is_ok());
        assert!(invoke(&mut doc, "addAttributes", |g| {
            let ty = string(g, "List~string~");
            let name = string(g, label);
            let comment = string(g, label);
            let pk = string(g, "PK");
            let fk = string(g, "FK");
            let keys = push_array(g, vec![pk, fk]);
            let attr = push_object(
                g,
                vec![
                    ("type", ty),
                    ("name", name),
                    ("comment", comment),
                    ("keys", Atom::Ref(keys)),
                ],
            );
            let attrs = push_array(g, vec![Atom::Ref(attr)]);
            vec![string(g, "A"), Atom::Ref(attrs)]
        })
        .result
        .is_ok());
        assert!(invoke(&mut doc, "addRelationship", |g| {
            let a = string(g, "ZERO_OR_ONE");
            let b = string(g, "ONE_OR_MORE");
            let rel = string(
                g,
                if n % 2 == 0 {
                    "IDENTIFYING"
                } else {
                    "NON_IDENTIFYING"
                },
            );
            let spec = push_object(g, vec![("cardA", a), ("relType", rel), ("cardB", b)]);
            vec![
                string(g, "A"),
                string(g, label),
                string(g, &name),
                Atom::Ref(spec),
            ]
        })
        .result
        .is_ok());
        assert!(invoke(&mut doc, "addClass", |g| {
            let c = string(g, "c");
            let ids = push_array(g, vec![c]);
            let values = ["fill:#abc", "fill-color:#123", "font-style:italic"]
                .map(|s| string(g, s))
                .to_vec();
            let styles = push_array(g, values);
            vec![Atom::Ref(ids), Atom::Ref(styles)]
        })
        .result
        .is_ok());
        assert!(invoke(&mut doc, "setClass", |g| {
            let a = string(g, "A");
            let c = string(g, "c");
            let ids = push_array(g, vec![a]);
            let names = push_array(g, vec![c]);
            vec![Atom::Ref(ids), Atom::Ref(names)]
        })
        .result
        .is_ok());
        assert!(invoke(&mut doc, "addCssStyles", |g| {
            let b = string(g, "B");
            let s = string(g, "stroke:#369");
            let ids = push_array(g, vec![b]);
            let styles = push_array(g, vec![s]);
            vec![Atom::Ref(ids), Atom::Ref(styles)]
        })
        .result
        .is_ok());
        assert!(invoke(&mut doc, "setDirection", |g| vec![string(
            g,
            ["TB", "LR", "BT", "RL"][n % 4]
        )])
        .result
        .is_ok());
        let mut options = RenderOptions::default();
        options.render.max_width = Some(240.);
        let result = doc.render(&options, &mut MetricsResources).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(result.rendered.svg.as_bytes())),
            expected,
            "case {n}"
        );
        assert_eq!((result.rendered.width, result.rendered.height), (w, 225.));
    }
}
#[test]
fn er_collection_replacements_raw_cache_values_and_builtin_error_messages() {
    let mut doc = parsed(SOURCE);
    let mut g = doc.snapshot().unwrap();
    let r = root(&g);
    let map = reference(field(&g, r, "entities"));
    let key = string(&mut g, "cached");
    let Node::Map { entries, .. } = &mut g.nodes[map as usize] else {
        panic!()
    };
    entries.push((key, Atom::Number(f64::from_bits(0x8000_0000_0000_0000))));
    doc.apply(doc.revision(), g).unwrap();
    let out = invoke(&mut doc, "addEntity", |g| vec![string(g, "cached")]);
    assert!(matches!(out.result,Ok(Atom::Number(n)) if n.to_bits()==0x8000_0000_0000_0000));
    let zero = method(&mut doc, "addEntity", vec![Atom::Number(-0.)]);
    let object = reference(zero.result.as_ref().unwrap());
    let Node::Map { entries, .. } = &zero.graph.nodes[map as usize] else {
        panic!()
    };
    assert!(matches!(entries.last().unwrap().0,Atom::Number(n) if n.to_bits()==0));
    assert!(
        matches!(field(&zero.graph,object,"label"),Atom::Number(n) if n.to_bits()==0x8000_0000_0000_0000)
    );
    let nan = method(
        &mut doc,
        "addEntity",
        vec![Atom::Number(f64::from_bits(0x7ff8_0000_0000_0001))],
    );
    let first = reference(nan.result.as_ref().unwrap());
    let nan = method(
        &mut doc,
        "addEntity",
        vec![Atom::Number(f64::from_bits(0x7ff8_0000_0000_0002))],
    );
    assert_eq!(reference(nan.result.as_ref().unwrap()), first);
    let mut g = doc.snapshot().unwrap();
    let new_map = g.nodes.len() as u32;
    g.nodes.push(Node::Map {
        entries: vec![],
        properties: vec![],
    });
    property(&mut g, r, "entities", Atom::Ref(new_map));
    doc.apply(doc.revision(), g).unwrap();
    assert!(invoke(&mut doc, "addEntity", |g| vec![string(g, "C")])
        .result
        .is_ok());
    let g = doc.snapshot().unwrap();
    assert!(g
        .text(field(&g, entity(&g, "C"), "id"))
        .unwrap()
        .is("entity-C-0"));
    same_source(&mut doc, "erDiagram\nC");
    let mut g = doc.snapshot().unwrap();
    property(&mut g, new_map, "get", Atom::Null);
    doc.apply(doc.revision(), g).unwrap();
    let out = invoke(&mut doc, "addEntity", |g| vec![string(g, "D")]);
    is_type_error(&out, "this.entities.get is not a function");
    let mut g = out.graph;
    property(&mut g, new_map, "get", Atom::HostReference(77));
    doc.apply(doc.revision(), g).unwrap();
    let out = invoke(&mut doc, "addEntity", |g| vec![string(g, "D")]);
    assert!(matches!(
        out.result,
        Err(PublicError::Pele(PeleError {
            code: ErrorCode::Semantic,
            ..
        }))
    ));
    let mut g = out.graph;
    delete(&mut g, new_map, "get");
    doc.apply(doc.revision(), g).unwrap();
    same_source(&mut doc, "erDiagram\nC");
    let method_id = doc.model_schema().methods[3].id;
    for (receiver, expected) in [
        (
            Atom::Bool(true),
            "Cannot create property 'direction' on boolean 'true'",
        ),
        (
            Atom::Number(3.),
            "Cannot create property 'direction' on number '3'",
        ),
        (
            Atom::Null,
            "Cannot set properties of null (setting 'direction')",
        ),
        (
            Atom::Undefined,
            "Cannot set properties of undefined (setting 'direction')",
        ),
    ] {
        let out = doc
            .invoke(
                doc.revision(),
                MethodCall {
                    method: method_id,
                    receiver,
                    arguments: vec![],
                },
                &mut MetricsResources,
            )
            .unwrap();
        is_type_error(&out, expected);
    }
    let args = apply_args(&mut doc, |g| {
        let id = g.strings.len() as u32;
        g.strings.push(JsString(vec![0xd800]));
        vec![Atom::String(id)]
    });
    let out = doc
        .invoke(
            doc.revision(),
            MethodCall {
                method: method_id,
                receiver: args[0].clone(),
                arguments: vec![],
            },
            &mut MetricsResources,
        )
        .unwrap();
    let Err(PublicError::JavaScript { message, .. }) = &out.result else {
        panic!()
    };
    let mut expected = "Cannot create property 'direction' on string '"
        .encode_utf16()
        .collect::<Vec<_>>();
    expected.extend([0xd800, 39]);
    assert_eq!(message.0, expected);
    // Native Array.includes accepts array style values; it stores an unmatched
    // array by alias. An active class then fails at the renderer boundary.
    let mut style_id = 0;
    let out = invoke(&mut doc, "addClass", |g| {
        let x = string(g, "x");
        let ids = push_array(g, vec![x]);
        let style = string(g, "fill:red");
        style_id = push_array(g, vec![style]);
        let styles = push_array(g, vec![Atom::Ref(style_id)]);
        vec![Atom::Ref(ids), Atom::Ref(styles)]
    });
    assert!(out.result.is_ok());
    let class = mapped(&out.graph, reference(field(&out.graph, r, "classes")), "x");
    assert_eq!(
        reference(
            &array(
                &out.graph,
                reference(field(&out.graph, reference(&class), "styles"))
            )[0]
        ),
        style_id
    );
    same_source(&mut doc, "erDiagram\nC");
}
