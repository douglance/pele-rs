//! Mutable parsed Sankey feature: real collection/object aliases, retained
//! provenance, typed rendering, edit/error recovery and lossless flat graphs.
use pele::compat::{
    diagrams::sankey::{Sankey, SankeyLink, SankeyModel, SankeyNode},
    public::{
        Atom, JsString, ModelGraph, Node, Property, PublicDiagramModule, PublicDocument,
        PublicEngine,
    },
    text::MetricsResources,
    DiagramModule, DiagramType, Engine, ErrorCode, ModuleDescriptor, Object, ParseContext,
    PeleError, RenderOptions, RenderResources, RenderSettings, Rendered, TextRequest,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
const SOURCE: &str = "sankey-beta\nA,B,2\nA,C,3";
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
fn fingerprint(g: &ModelGraph) -> String {
    serde_json::to_string(g).unwrap()
}
fn typed(
    nodes: &[(&str, usize)],
    links: &[(usize, usize, f64)],
    title: Option<&str>,
) -> SankeyModel {
    SankeyModel {
        diagram_type: DiagramType::Sankey,
        nodes: nodes
            .iter()
            .map(|(id, index)| SankeyNode {
                id: (*id).into(),
                index: *index,
            })
            .collect(),
        links: links
            .iter()
            .map(|(source, target, value)| SankeyLink {
                source: SankeyNode {
                    id: "unused endpoint label".into(),
                    index: *source,
                },
                target: SankeyNode {
                    id: "unused endpoint label".into(),
                    index: *target,
                },
                value: *value,
            })
            .collect(),
        title: title.map(str::to_owned),
        acc_title: None,
        acc_descr: None,
    }
}
fn direct(model: &SankeyModel) -> Rendered {
    Sankey
        .render(
            model,
            &Object::default(),
            &RenderSettings::default(),
            &mut MetricsResources,
        )
        .unwrap()
}
struct Counted(Arc<AtomicUsize>);
impl DiagramModule for Counted {
    type Model = SankeyModel;
    fn descriptor(&self) -> ModuleDescriptor {
        Sankey.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Sankey.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Sankey.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Counted {
    type PublicState = <Sankey as PublicDiagramModule>::PublicState;
    fn create_public(&self, m: &SankeyModel) -> Result<Self::PublicState, PeleError> {
        Sankey.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Sankey.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        Sankey.apply_public(s, g)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Sankey.render_public(s, c, o, h)
    }
}
#[test]
fn mutable_sankey_aliases_order_lookup_and_object_replacement() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted(count.clone()));
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let mut g = doc.snapshot().unwrap();
    let root = root(&g);
    assert_eq!(
        g.root_properties()
            .unwrap()
            .iter()
            .map(|p| g.strings[p.key as usize].well_formed().unwrap())
            .collect::<Vec<_>>(),
        ["type", "nodes", "links", "title", "accTitle", "accDescr", "lookup"]
    );
    assert!(g.root_properties().unwrap().iter().all(|p| p.flags == 7));
    assert!(matches!(g.field("title").unwrap(), Some(Atom::Undefined)));
    let nodes_id = reference(g.field("nodes").unwrap().unwrap());
    let links_id = reference(g.field("links").unwrap().unwrap());
    let lookup_id = reference(g.field("lookup").unwrap().unwrap());
    let node_values = array(&g, nodes_id);
    let ids: Vec<_> = node_values.iter().map(reference).collect();
    let link_values = array(&g, links_id);
    let l0 = reference(&link_values[0]);
    let l1 = reference(&link_values[1]);
    assert_eq!(reference(field(&g, l0, "source")), ids[0]);
    assert_eq!(reference(field(&g, l1, "source")), ids[0]);
    assert_eq!(reference(field(&g, l0, "target")), ids[1]);
    let Node::Map { entries, .. } = &g.nodes[lookup_id as usize] else {
        panic!()
    };
    assert_eq!(
        entries
            .iter()
            .map(|(_, v)| reference(v))
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        Engine::with_builtins()
            .render_source(SOURCE, &Default::default(), &mut MetricsResources)
            .unwrap()
    );
    let renamed = string(&mut g, "Edited A");
    property(&mut g, ids[0], "id", renamed);
    property(&mut g, l0, "value", Atom::Number(4.));
    property(&mut g, l1, "value", Atom::Number(2.));
    let after = string(&mut g, "After");
    property(&mut g, root, "title", after);
    let fake = string(&mut g, "pie");
    property(&mut g, root, "type", fake);
    // Moving records leaves their index fields and every link/lookup alias intact.
    property(&mut g, nodes_id, "0", node_values[2].clone());
    property(&mut g, nodes_id, "1", node_values[0].clone());
    property(&mut g, nodes_id, "2", node_values[1].clone());
    doc.apply(0, g.clone()).unwrap();
    let result = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    let expected = Engine::with_builtins()
        .render_source(
            "---\ntitle: After\n---\nsankey-beta\nC,Edited A,4\nC,B,2",
            &Default::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert_eq!(result, expected);
    assert_eq!(doc.diagram_type().as_str(), "sankey");
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    let replacement_label = string(&mut g, "  Replacement  ");
    let replacement = push_object(
        &mut g,
        vec![("id", replacement_label), ("index", Atom::Number(9999.))],
    );
    property(&mut g, nodes_id, "0", Atom::Ref(replacement));
    // A replaced array record does not replace either old endpoint or lookup value.
    assert_eq!(reference(field(&g, l1, "target")), ids[2]);
    let map_id = g.nodes.len() as u32;
    g.nodes.push(Node::Map {
        entries: vec![],
        properties: vec![],
    });
    property(&mut g, root, "lookup", Atom::Ref(map_id));
    let stale_key = string(&mut g, "stale");
    let Node::Map { entries, .. } = &mut g.nodes[map_id as usize] else {
        panic!()
    };
    entries.push((stale_key, Atom::Ref(ids[2])));
    entries.push((Atom::Number(f64::NAN), Atom::Ref(ids[0])));
    doc.apply(1, g.clone()).unwrap();
    let expected = direct(&typed(
        &[("  Replacement  ", 9999), ("Edited A", 0), ("B", 1)],
        &[(0, 1, 4.), (0, 2, 2.)],
        Some("After"),
    ));
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        expected
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    let Node::Map { entries, .. } = &mut g.nodes[map_id as usize] else {
        panic!()
    };
    entries.remove(0);
    entries.clear();
    let key_ids: Vec<_> = g
        .strings
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is("lookup"))
        .map(|(i, _)| i as u32)
        .collect();
    properties_mut(&mut g.nodes[root as usize]).retain(|p| !key_ids.contains(&p.key));
    doc.apply(2, g.clone()).unwrap();
    assert!(doc.snapshot().unwrap().field("lookup").unwrap().is_none());
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        expected
    );
    // Endpoint objects can be detached and need only the field the renderer reads.
    let detached = push_object(&mut g, vec![("index", Atom::Number(2.))]);
    let link = push_object(
        &mut g,
        vec![
            ("source", Atom::Ref(detached)),
            ("target", Atom::Ref(ids[0])),
            ("value", Atom::Number(7.)),
        ],
    );
    let replacement_links = push_array(&mut g, vec![Atom::Ref(link)]);
    property(&mut g, root, "links", Atom::Ref(replacement_links));
    doc.apply(3, g.clone()).unwrap();
    let expected = direct(&typed(
        &[("  Replacement  ", 9999), ("Edited A", 0), ("B", 1)],
        &[(2, 0, 7.)],
        Some("After"),
    ));
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        expected
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    engine.register(Sankey);
    drop(engine);
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        expected
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn mutable_sankey_render_errors_preserve_edits_and_recover_without_parsing() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted(count.clone()));
    let mut doc = engine
        .parse("sankey-beta\nA,B,2", &Default::default())
        .unwrap();
    let mut g = doc.snapshot().unwrap();
    let root = root(&g);
    let nodes = reference(g.field("nodes").unwrap().unwrap());
    let link = reference(&array(&g, reference(g.field("links").unwrap().unwrap()))[0]);
    let source = field(&g, link, "source").clone();
    let target = field(&g, link, "target").clone();
    property(&mut g, link, "target", source);
    doc.apply(0, g.clone()).unwrap();
    let error = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap_err();
    let expected = Engine::with_builtins()
        .render_source(
            "sankey-beta\nA,A,2",
            &Default::default(),
            &mut MetricsResources,
        )
        .unwrap_err();
    assert_eq!(error, expected);
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    property(&mut g, link, "target", target.clone());
    property(&mut g, reference(&target), "index", Atom::Number(1e300));
    doc.apply(1, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Semantic
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    property(&mut g, reference(&target), "index", Atom::Number(1.));
    doc.apply(2, g.clone()).unwrap();
    let expected = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    struct Reject;
    impl RenderResources for Reject {
        fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
            Err(PeleError::new(
                "host measurement rejected",
                ErrorCode::Semantic,
            ))
        }
    }
    assert_eq!(
        doc.render(&Default::default(), &mut Reject)
            .unwrap_err()
            .message,
        "host measurement rejected"
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(1);
    assert_eq!(
        doc.render(&options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let fake = string(&mut g, "wrong kind");
    property(&mut g, root, "nodes", fake);
    doc.apply(3, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Semantic
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    property(&mut g, root, "nodes", Atom::Ref(nodes));
    doc.apply(4, g).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn mutable_sankey_retains_unknown_deep_cycles_utf16_and_ieee_fields() {
    let model = typed(&[("A", 0), ("B", 1)], &[(0, 1, 2.)], None);
    let mut doc = PublicDocument::from_module(Sankey, &model, Object::default()).unwrap();
    let mut g = doc.snapshot().unwrap();
    let root = root(&g);
    let mut tail = Atom::Ref(root);
    for _ in 0..6000 {
        tail = Atom::Ref(push_object(&mut g, vec![("next", tail)]));
    }
    property(&mut g, root, "cycle", tail);
    let string_id = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 0x0041, 0xdc00]));
    property(&mut g, root, "utf16", Atom::String(string_id));
    for (key, value) in [
        ("nan", f64::from_bits(0x7ff8000000001234)),
        ("positiveInfinity", f64::INFINITY),
        ("negativeInfinity", f64::NEG_INFINITY),
        ("negativeZero", -0.),
    ] {
        property(&mut g, root, key, Atom::Number(value));
    }
    let nodes_id = reference(g.field("nodes").unwrap().unwrap());
    let node = reference(&array(&g, nodes_id)[0]);
    property(&mut g, node, "owner", Atom::Ref(root));
    let link = reference(&array(&g, reference(g.field("links").unwrap().unwrap()))[0]);
    property(&mut g, link, "self", Atom::Ref(link));
    let lookup_id = reference(g.field("lookup").unwrap().unwrap());
    property(&mut g, lookup_id, "owner", Atom::Ref(root));
    properties_mut(&mut g.nodes[root as usize]).push(Property {
        key: string_id,
        value: Atom::Ref(lookup_id),
        flags: 5,
    });
    let encoded = fingerprint(&g);
    let decoded: ModelGraph = serde_json::from_str(&encoded).unwrap();
    doc.apply(0, decoded).unwrap();
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), encoded);
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        direct(&model)
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), encoded);
    for (number, value) in [
        (f64::NAN, 0.),
        (f64::INFINITY, 0.),
        (-5., 0.),
        (-0., 0.),
        (4.5, 4.5),
    ] {
        let mut edit = doc.snapshot().unwrap();
        property(&mut edit, link, "value", Atom::Number(number));
        let revision = doc.revision();
        doc.apply(revision, edit.clone()).unwrap();
        assert_eq!(
            doc.render(&Default::default(), &mut MetricsResources)
                .unwrap()
                .rendered,
            direct(&typed(&[("A", 0), ("B", 1)], &[(0, 1, value)], None))
        );
        assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&edit));
    }
}
