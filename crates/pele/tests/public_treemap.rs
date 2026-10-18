//! Editable Treemap feature through the public document lane: hierarchy,
//! shared class arrays, retained identity, source-free edits and deep trees.
use pele::compat::{
    diagrams::treemap::{Treemap, TreemapModel, TreemapNode},
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
const SOURCE:&str="treemap\nclassDef cc fill:red,color:blue\n\"Root\"\n \"A\":1:::cc\n \"B\":2:::cc\n\"Other\"\n \"C\":3\n\"Empty\"";
fn reference(a: &Atom) -> u32 {
    if let Atom::Ref(id) = a {
        *id
    } else {
        panic!("object reference expected")
    }
}
fn root(g: &ModelGraph) -> u32 {
    reference(&g.root)
}
fn string(g: &mut ModelGraph, s: &str) -> Atom {
    let id = g.strings.len() as u32;
    g.strings.push(JsString::from(s));
    Atom::String(id)
}
fn properties(node: &mut Node) -> &mut Vec<Property> {
    match node {
        Node::Object { properties }
        | Node::Instance { properties, .. }
        | Node::Array { properties, .. }
        | Node::Map { properties, .. }
        | Node::Set { properties, .. }
        | Node::Date { properties, .. } => properties,
    }
}
fn property(g: &mut ModelGraph, id: u32, key: &str, value: Atom) {
    let position = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key));
    if let Some(at) = position {
        properties(&mut g.nodes[id as usize])[at].value = value
    } else {
        let key_id = g.strings.len() as u32;
        g.strings.push(JsString::from(key));
        properties(&mut g.nodes[id as usize]).push(Property {
            key: key_id,
            value,
            flags: 7,
        });
    }
}
fn delete(g: &mut ModelGraph, id: u32, key: &str) {
    let remove: Vec<_> = g.nodes[id as usize]
        .properties()
        .iter()
        .enumerate()
        .filter(|(_, p)| g.strings[p.key as usize].is(key))
        .map(|(i, _)| i)
        .collect();
    for i in remove.into_iter().rev() {
        properties(&mut g.nodes[id as usize]).remove(i);
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
fn push_styles(g: &mut ModelGraph, styles: &[&str]) -> u32 {
    let values = styles.iter().map(|s| string(g, s)).collect();
    push_array(g, values)
}
fn fingerprint(g: &ModelGraph) -> String {
    serde_json::to_string(g).unwrap()
}
fn leaf(
    name: &str,
    value: f64,
    selector: Option<&str>,
    styles: Option<Arc<Vec<String>>>,
) -> TreemapNode {
    TreemapNode {
        name: name.into(),
        children: None,
        value: Some(value),
        class_selector: selector.map(str::to_owned),
        css_compiled_styles: styles,
    }
}
fn section(name: &str, children: Vec<TreemapNode>) -> TreemapNode {
    TreemapNode {
        name: name.into(),
        children: Some(children),
        value: None,
        class_selector: None,
        css_compiled_styles: None,
    }
}
fn model(roots: Vec<TreemapNode>, title: Option<&str>) -> TreemapModel {
    TreemapModel {
        diagram_type: DiagramType::Treemap,
        title: title.map(str::to_owned),
        acc_title: None,
        acc_descr: None,
        roots,
        classes: vec![],
    }
}
fn direct(m: &TreemapModel) -> Rendered {
    Treemap
        .render(
            m,
            &Object::default(),
            &RenderSettings::default(),
            &mut MetricsResources,
        )
        .unwrap()
}
struct Counted(Arc<AtomicUsize>);
impl DiagramModule for Counted {
    type Model = TreemapModel;
    fn descriptor(&self) -> ModuleDescriptor {
        Treemap.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Treemap.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Treemap.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Counted {
    type PublicState = <Treemap as PublicDiagramModule>::PublicState;
    fn create_public(&self, m: &Self::Model) -> Result<Self::PublicState, PeleError> {
        Treemap.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Treemap.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        Treemap.apply_public(s, g)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Treemap.render_public(s, c, o, h)
    }
}
#[test]
fn mutable_treemap_styles_replacement_reparenting_and_root_identity() {
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
        ["type", "title", "accTitle", "accDescr", "roots", "classes"]
    );
    assert!(g.root_properties().unwrap().iter().all(|p| p.flags == 7));
    assert!(matches!(g.field("title").unwrap(), Some(Atom::Undefined)));
    let roots_id = reference(g.field("roots").unwrap().unwrap());
    let roots = array(&g, roots_id);
    let group = reference(&roots[0]);
    let other = reference(&roots[1]);
    let empty = reference(&roots[2]);
    let children_id = reference(field(&g, group, "children"));
    let children = array(&g, children_id);
    let a = reference(&children[0]);
    let b = reference(&children[1]);
    let other_children = reference(field(&g, other, "children"));
    let c = array(&g, other_children)[0].clone();
    assert!(matches!(field(&g, a, "children"), Atom::Undefined));
    assert!(matches!(field(&g, group, "classSelector"), Atom::Undefined));
    let node_keys: Vec<_> = g.nodes[a as usize]
        .properties()
        .iter()
        .map(|p| g.strings[p.key as usize].well_formed().unwrap())
        .collect();
    assert_eq!(
        node_keys,
        [
            "name",
            "children",
            "classSelector",
            "cssCompiledStyles",
            "value"
        ]
    );
    let styles_id = reference(field(&g, a, "cssCompiledStyles"));
    assert_eq!(styles_id, reference(field(&g, b, "cssCompiledStyles")));
    let classes_id = reference(g.field("classes").unwrap().unwrap());
    let Node::Map { entries, .. } = &g.nodes[classes_id as usize] else {
        panic!()
    };
    assert_eq!(reference(&entries[0].1), styles_id);
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        Engine::with_builtins()
            .render_source(SOURCE, &Default::default(), &mut MetricsResources)
            .unwrap()
    );
    property(&mut g, a, "value", Atom::Number(4.));
    let name = string(&mut g, "B after");
    property(&mut g, b, "name", name);
    let blue = string(&mut g, "fill:blue");
    property(&mut g, styles_id, "0", blue);
    let white = string(&mut g, "color:white");
    property(&mut g, styles_id, "1", white);
    let title = string(&mut g, "After");
    property(&mut g, root, "title", title);
    let fake = string(&mut g, "pie");
    property(&mut g, root, "type", fake);
    doc.apply(0, g.clone()).unwrap();
    let source="treemap\ntitle After\nclassDef cc fill:blue,color:white\n\"Root\"\n \"A\":4:::cc\n \"B after\":2:::cc\n\"Other\"\n \"C\":3\n\"Empty\"";
    let after = Engine::with_builtins()
        .render_source(source, &Default::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        after
    );
    assert_eq!(doc.diagram_type().as_str(), "treemap");
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    // Replacing a class entry leaves node.cssCompiledStyles pointing at the old array.
    let green = push_styles(&mut g, &["fill:green", "color:black"]);
    let key = string(&mut g, "cc");
    let new_map = g.nodes.len() as u32;
    g.nodes.push(Node::Map {
        entries: vec![(key, Atom::Ref(green))],
        properties: vec![],
    });
    property(&mut g, root, "classes", Atom::Ref(new_map));
    doc.apply(1, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        after
    );
    assert_eq!(
        reference(field(&doc.snapshot().unwrap(), a, "cssCompiledStyles")),
        styles_id
    );
    let moved = push_array(&mut g, vec![c.clone(), Atom::Ref(a)]);
    property(&mut g, other, "children", Atom::Ref(moved));
    let remaining = push_array(&mut g, vec![Atom::Ref(b)]);
    property(&mut g, group, "children", Atom::Ref(remaining));
    let reordered = push_array(
        &mut g,
        vec![Atom::Ref(other), Atom::Ref(group), Atom::Ref(empty)],
    );
    property(&mut g, root, "roots", Atom::Ref(reordered));
    doc.apply(2, g.clone()).unwrap();
    let source="treemap\ntitle After\nclassDef cc fill:blue,color:white\n\"Other\"\n \"C\":3\n \"A\":4:::cc\n\"Root\"\n \"B after\":2:::cc\n\"Empty\"";
    let moved_render = Engine::with_builtins()
        .render_source(source, &Default::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        moved_render
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    // A native edit can contain text the source grammar cannot express, and
    // explicit style replacement changes only the record carrying that reference.
    let name = string(&mut g, "A \"quoted\"\nline");
    property(&mut g, a, "name", name);
    property(&mut g, a, "cssCompiledStyles", Atom::Ref(green));
    let selector = string(&mut g, "manual");
    property(&mut g, a, "classSelector", selector);
    doc.apply(3, g.clone()).unwrap();
    let green_styles = Arc::new(vec!["fill:green".into(), "color:black".into()]);
    let blue_styles = Arc::new(vec!["fill:blue".into(), "color:white".into()]);
    let expected = model(
        vec![
            section(
                "Other",
                vec![
                    leaf("C", 3., None, None),
                    leaf(
                        "A \"quoted\"\nline",
                        4.,
                        Some("manual"),
                        Some(green_styles.clone()),
                    ),
                ],
            ),
            section(
                "Root",
                vec![leaf("B after", 2., Some("cc"), Some(blue_styles.clone()))],
            ),
            section("Empty", vec![]),
        ],
        Some("After"),
    );
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        direct(&expected)
    );
    // The same node may appear in two branches. Its exported identity stays shared.
    let shared = push_array(&mut g, vec![Atom::Ref(b), Atom::Ref(a)]);
    property(&mut g, group, "children", Atom::Ref(shared));
    let Node::Map { entries, .. } = &mut g.nodes[new_map as usize] else {
        panic!()
    };
    entries.clear();
    delete(&mut g, root, "classes");
    doc.apply(4, g.clone()).unwrap();
    let expected = model(
        vec![
            section(
                "Other",
                vec![
                    leaf("C", 3., None, None),
                    leaf(
                        "A \"quoted\"\nline",
                        4.,
                        Some("manual"),
                        Some(green_styles.clone()),
                    ),
                ],
            ),
            section(
                "Root",
                vec![
                    leaf("B after", 2., Some("cc"), Some(blue_styles)),
                    leaf("A \"quoted\"\nline", 4., Some("manual"), Some(green_styles)),
                ],
            ),
            section("Empty", vec![]),
        ],
        Some("After"),
    );
    let expected_render = direct(&expected);
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        expected_render
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    assert!(doc.snapshot().unwrap().field("classes").unwrap().is_none());
    assert_eq!(count.load(Ordering::SeqCst), 1);
    engine.register(Treemap);
    drop(engine);
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        expected_render
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn mutable_treemap_hierarchy_and_resource_errors_retain_edits_and_recover() {
    let mut engine = PublicEngine::empty();
    engine.register(Treemap);
    let mut doc = engine
        .parse("treemap\n\"Root\"\n \"Leaf\":2", &Default::default())
        .unwrap();
    let mut g = doc.snapshot().unwrap();
    let root = root(&g);
    let roots_id = reference(g.field("roots").unwrap().unwrap());
    let group = reference(&array(&g, roots_id)[0]);
    let children_id = reference(field(&g, group, "children"));
    let leaf = array(&g, children_id)[0].clone();
    let baseline = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    property(&mut g, children_id, "0", Atom::Ref(group));
    doc.apply(0, g.clone()).unwrap();
    assert!(doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("hierarchy contains a cycle"));
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    property(&mut g, children_id, "0", leaf.clone());
    doc.apply(1, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        baseline
    );
    let empty = push_array(&mut g, vec![]);
    property(&mut g, reference(&leaf), "children", Atom::Ref(empty));
    doc.apply(2, g.clone()).unwrap();
    let invisible = direct(&model(
        vec![section(
            "Root",
            vec![TreemapNode {
                name: "Leaf".into(),
                children: Some(vec![]),
                value: Some(2.),
                class_selector: None,
                css_compiled_styles: None,
            }],
        )],
        None,
    ));
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        invisible
    );
    delete(&mut g, reference(&leaf), "children");
    doc.apply(3, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        baseline
    );
    let fake = string(&mut g, "wrong kind");
    property(&mut g, root, "roots", fake);
    doc.apply(4, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Semantic
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    property(&mut g, root, "roots", Atom::Ref(roots_id));
    doc.apply(5, g.clone()).unwrap();
    struct Reject;
    impl RenderResources for Reject {
        fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
            Err(PeleError::new("measurer failed", ErrorCode::Semantic))
        }
    }
    assert_eq!(
        doc.render(&Default::default(), &mut Reject)
            .unwrap_err()
            .message,
        "measurer failed"
    );
    let mut limit = RenderOptions::default();
    limit.render.output_limit = Some(1);
    assert_eq!(
        doc.render(&limit, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap(),
        baseline
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
}

#[test]
fn mutable_treemap_iterative_deep_tree_and_lossless_extra_graphs() {
    let mut tree = leaf("Leaf", 1., None, None);
    for _ in 0..6000 {
        tree = section("Section", vec![tree]);
    }
    let original = model(vec![tree], None);
    let mut doc = PublicDocument::from_module(Treemap, &original, Object::default()).unwrap();
    let baseline = direct(&original);
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        baseline
    );
    let mut g = doc.snapshot().unwrap();
    let root = root(&g);
    let mut tail = Atom::Ref(root);
    for _ in 0..6000 {
        tail = Atom::Ref(push_object(&mut g, vec![("next", tail)]));
    }
    property(&mut g, root, "applicationCycle", tail);
    let units = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 0x0041, 0xdc00]));
    property(&mut g, root, "utf16", Atom::String(units));
    properties(&mut g.nodes[root as usize]).push(Property {
        key: units,
        value: Atom::Ref(root),
        flags: 5,
    });
    for (key, value) in [
        ("nan", f64::from_bits(0x7ff8000000001234)),
        ("positiveInfinity", f64::INFINITY),
        ("negativeInfinity", f64::NEG_INFINITY),
        ("negativeZero", -0.),
    ] {
        property(&mut g, root, key, Atom::Number(value));
    }
    let encoded = fingerprint(&g);
    let decoded: ModelGraph = serde_json::from_str(&encoded).unwrap();
    doc.apply(0, decoded).unwrap();
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), encoded);
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        baseline
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), encoded);
    // Replacing the whole roots array releases the deep typed tree iteratively.
    let label = string(&mut g, "Replacement");
    let leaf = push_object(
        &mut g,
        vec![
            ("name", label),
            ("children", Atom::Undefined),
            ("classSelector", Atom::Undefined),
            ("value", Atom::Number(f64::INFINITY)),
        ],
    );
    let replacement = push_array(&mut g, vec![Atom::Ref(leaf)]);
    property(&mut g, root, "roots", Atom::Ref(replacement));
    doc.apply(1, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        direct(&model(
            vec![self::leaf("Replacement", f64::INFINITY, None, None)],
            None
        ))
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
    property(&mut g, leaf, "value", Atom::Number(3.));
    doc.apply(2, g.clone()).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        direct(&model(
            vec![self::leaf("Replacement", 3., None, None)],
            None
        ))
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), fingerprint(&g));
}
