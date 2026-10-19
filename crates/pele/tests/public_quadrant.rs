//! Mutable QuadrantDb feature against pinned Pele 1.6.0, 458322a5.
//! Oracle dist SHA256 d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! db.ts SHA256 844813f73fdc1db02cc0855a1c68f6a6bba48fb605ffecf873d39d4f3b6f9dda;
//! render.ts SHA256 194428f6425d399bb60200b20aa0ad402de7b2870f44801de429ed905b137570.
//! Own descriptors and prototype were probed with Object.getOwnPropertyDescriptors
//! and Object.getOwnPropertyNames(Object.getPrototypeOf(parse(source))). Prototype
//! setter/addPoint/addClass dispatch is native; constructor calls remain a capability gap.
use pele::compat::{
    diagrams::quadrant_chart::{QuadrantChart, QuadrantModel},
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
const SOURCE:&str="quadrantChart\nx-axis Low --> High\ny-axis Down --> Up\nquadrant-1 First\nA:::cc: [0.2,0.4] color: #abc\nB:::cc: [0.7,0.8]\nclassDef cc radius: 7, color: #f00";
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
        | Node::Date { properties, .. }
        | Node::Error { properties, .. } => properties,
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
fn fingerprint(g: &ModelGraph) -> String {
    serde_json::to_string(g).unwrap()
}
struct Counted(Arc<AtomicUsize>);
impl DiagramModule for Counted {
    type Model = QuadrantModel;
    fn descriptor(&self) -> ModuleDescriptor {
        QuadrantChart.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        QuadrantChart.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        QuadrantChart.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Counted {
    type PublicState = <QuadrantChart as PublicDiagramModule>::PublicState;
    fn model_schema(&self) -> &'static ModelSchema {
        QuadrantChart.model_schema()
    }
    fn invoke_public(
        &self,
        s: &mut Self::PublicState,
        call: MethodCall,
        c: &Object,
        h: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError> {
        QuadrantChart.invoke_public(s, call, c, h)
    }
    fn create_public(&self, m: &Self::Model) -> Result<Self::PublicState, PeleError> {
        QuadrantChart.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        QuadrantChart.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        QuadrantChart.apply_public(s, g)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        QuadrantChart.render_public(s, c, o, h)
    }
}
fn source_render(source: &str) -> RenderResult {
    Engine::with_builtins()
        .render_source(source, &RenderOptions::default(), &mut MetricsResources)
        .unwrap()
}
fn same_source(doc: &mut PublicDocument, source: &str) {
    assert_eq!(
        doc.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        source_render(source)
    )
}
fn apply(doc: &mut PublicDocument, g: ModelGraph) {
    doc.apply(doc.revision(), g).unwrap()
}
#[test]
fn mutable_quadrant_own_fields_live_map_copied_overrides_and_replacement() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted(count.clone()));
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    same_source(&mut doc, SOURCE);
    let mut g = doc.snapshot().unwrap();
    let root = root(&g);
    assert_eq!(
        g.root_properties()
            .unwrap()
            .iter()
            .map(|p| g.strings[p.key as usize].well_formed().unwrap())
            .collect::<Vec<_>>(),
        [
            "type",
            "title",
            "accTitle",
            "accDescr",
            "quadrants",
            "xAxisLeft",
            "xAxisRight",
            "yAxisBottom",
            "yAxisTop",
            "points",
            "classes"
        ]
    );
    assert!(g.root_properties().unwrap().iter().all(|p| p.flags == 7));
    for key in ["title", "accTitle", "accDescr"] {
        assert!(matches!(g.field(key).unwrap(), Some(Atom::Undefined)))
    }
    let quadrants = reference(g.field("quadrants").unwrap().unwrap());
    let qs = array(&g, quadrants);
    assert_eq!(qs.len(), 4);
    assert!(matches!(qs[1], Atom::Undefined));
    let points = reference(g.field("points").unwrap().unwrap());
    let ps = array(&g, points);
    let a = reference(&ps[0]);
    let b = reference(&ps[1]);
    assert_eq!(
        g.nodes[a as usize]
            .properties()
            .iter()
            .map(|p| g.strings[p.key as usize].well_formed().unwrap())
            .collect::<Vec<_>>(),
        ["x", "y", "text", "className", "color"]
    );
    let classes = reference(g.field("classes").unwrap().unwrap());
    let Node::Map { entries, .. } = &g.nodes[classes as usize] else {
        panic!()
    };
    let style = reference(&entries[0].1);
    assert_ne!(style, a);
    assert_ne!(style, b);
    let blue = string(&mut g, "#00f");
    property(&mut g, style, "color", blue);
    apply(&mut doc, g);
    let blue_source = SOURCE.replace(
        "classDef cc radius: 7, color: #f00",
        "classDef cc radius: 7, color: #00f",
    );
    same_source(&mut doc, &blue_source);
    // A owns color:#abc; B gets current Map style. Replace Map object entirely.
    let mut g = doc.snapshot().unwrap();
    let green = string(&mut g, "#0f0");
    let new_style = push_object(
        &mut g,
        vec![("radius", Atom::Number(11.)), ("color", green)],
    );
    let key = string(&mut g, "cc");
    let new_classes = g.nodes.len() as u32;
    g.nodes.push(Node::Map {
        entries: vec![(key, Atom::Ref(new_style))],
        properties: vec![],
    });
    property(&mut g, root, "classes", Atom::Ref(new_classes));
    apply(&mut doc, g);
    let green_source = SOURCE.replace(
        "classDef cc radius: 7, color: #f00",
        "classDef cc radius: 11, color: #0f0",
    );
    same_source(&mut doc, &green_source);
    // Point fields copied from source remain independent of class object replacement.
    let mut g = doc.snapshot().unwrap();
    delete(&mut g, a, "color");
    property(&mut g, b, "x", Atom::Number(0.9));
    property(&mut g, b, "radius", Atom::Number(4.));
    let after = green_source
        .replace("A:::cc: [0.2,0.4] color: #abc", "A:::cc: [0.2,0.4]")
        .replace("B:::cc: [0.7,0.8]", "B:::cc: [0.9,0.8] radius: 4");
    apply(&mut doc, g);
    same_source(&mut doc, &after);
    // Same QuadrantText object aliases two axes, a quadrant and point text.
    let mut g = doc.snapshot().unwrap();
    let label = reference(field(&g, a, "text"));
    property(&mut g, root, "xAxisLeft", Atom::Ref(label));
    property(&mut g, root, "yAxisTop", Atom::Ref(label));
    property(&mut g, quadrants, "0", Atom::Ref(label));
    let shared = string(&mut g, "Shared");
    property(&mut g, label, "text", shared);
    apply(&mut doc, g);
    let shared_source = after
        .replace("x-axis Low --> High", "x-axis Shared --> High")
        .replace("y-axis Down --> Up", "y-axis Down --> Shared")
        .replace("quadrant-1 First", "quadrant-1 Shared")
        .replace("A:::cc:", "Shared:::cc:");
    same_source(&mut doc, &shared_source);
    let before = doc.snapshot().unwrap();
    assert_eq!(reference(field(&before, root, "xAxisLeft")), label);
    assert_eq!(reference(field(&before, root, "yAxisTop")), label);
    // Class value may alias an actual point object. Its own radius becomes live style.
    let mut g = before;
    let color = string(&mut g, "#a0b");
    property(&mut g, a, "color", color);
    property(&mut g, a, "radius", Atom::Number(13.));
    if let Node::Map { entries, .. } = &mut g.nodes[new_classes as usize] {
        entries[0].1 = Atom::Ref(a)
    }
    apply(&mut doc, g);
    let aliased = shared_source
        .replace(
            "Shared:::cc: [0.2,0.4]",
            "Shared:::cc: [0.2,0.4] color: #a0b, radius: 13",
        )
        .replace(
            "classDef cc radius: 11, color: #0f0",
            "classDef cc radius: 13, color: #a0b",
        );
    same_source(&mut doc, &aliased);
    // Reorder and replace points Array without touching retained record IDs.
    let mut g = doc.snapshot().unwrap();
    let replacement = push_array(&mut g, vec![Atom::Ref(b), Atom::Ref(a)]);
    property(&mut g, root, "points", Atom::Ref(replacement));
    let fake = string(&mut g, "application-type");
    property(&mut g, root, "type", fake);
    apply(&mut doc, g);
    let reordered = aliased.replace(
        "Shared:::cc: [0.2,0.4] color: #a0b, radius: 13\nB:::cc: [0.9,0.8] radius: 4",
        "B:::cc: [0.9,0.8] radius: 4\nShared:::cc: [0.2,0.4] color: #a0b, radius: 13",
    );
    same_source(&mut doc, &reordered);
    let mut g = doc.snapshot().unwrap();
    if let Node::Map { entries, .. } = &mut g.nodes[new_classes as usize] {
        entries.clear()
    }
    apply(&mut doc, g);
    let removed = reordered.replace("\nclassDef cc radius: 13, color: #a0b", "");
    same_source(&mut doc, &removed);
    engine.register(QuadrantChart);
    drop(engine);
    same_source(&mut doc, &removed);
    assert_eq!(doc.diagram_type().as_str(), "quadrantChart");
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn mutable_quadrant_pinned_svg_numbers_css_and_parse_int() {
    // Development oracle: parse(SOURCE); model.points[1][key]=value; render(model).
    // SHA256 covers the complete SVG, not selected geometry substrings.
    let vectors: [(&str, &str, Option<f64>, &str, &str); 15] = [
        (
            "nan",
            "radius",
            Some(f64::NAN),
            "NaN",
            "b69eeeab4dc29ede5f64cb912f80249b830767a53fdb51f24407506644e4b63d",
        ),
        (
            "positive-infinity",
            "radius",
            Some(f64::INFINITY),
            "Infinity",
            "5efe38f8882052bb012985030293bfacf9492583d36e2ec5d6ee938ccce29a4f",
        ),
        (
            "negative-infinity",
            "radius",
            Some(f64::NEG_INFINITY),
            "-Infinity",
            "c99d68101cd63056882735d10da57be12647856001a8fd48914587510833dc1f",
        ),
        (
            "negative-zero",
            "radius",
            Some(-0.),
            "0",
            "48f7260d18d82a9dc818f967dfe2c5b59806fd2265323c45e76286b1d5c8c754",
        ),
        (
            "fraction",
            "radius",
            Some(-3.25),
            "-3.25",
            "c03583e818d6137ea63b43f3df640bdc20616fa1bc05df3bf2e2c22f7a668f77",
        ),
        (
            "decimal-prefix",
            "strokeWidth",
            None,
            " -2.75px ",
            "7bdb47eafd28f927c74908113a61cb64e51054caaed6ed92055670b96830eec4",
        ),
        (
            "hex-prefix",
            "strokeWidth",
            None,
            " +0x10wat",
            "390646dd89497836c894f557eaf1646d407e6cbf8b1ddc41150a36a97eb373dd",
        ),
        (
            "binary-is-decimal-prefix",
            "strokeWidth",
            None,
            "0b100",
            "8f28384511f1f83a091cd0d53691b8acdd1f672eeb3da79bcb07b9356e901351",
        ),
        (
            "negative-large-hex",
            "strokeWidth",
            None,
            "-0x20000000000001junk",
            "cc7d64e258e4c31e2a2b92cb38edf71c16319c081e149c3144ce602304745acd",
        ),
        (
            "empty-width",
            "strokeWidth",
            None,
            "",
            "82c53101a1dc56c5848391333e8a02f262394e808f9755c1790b34ac7b38948d",
        ),
        (
            "invalid-width",
            "strokeWidth",
            None,
            "junk",
            "c76a7c98c4c4e492188b066e716cddd25fdf8ccb32274cad1e674d48fe34598b",
        ),
        (
            "unsafe-width",
            "strokeWidth",
            None,
            "2;fill:red",
            "a8258197d6de6dbb0a7c9bd12fc3cb4df6b7cecd8584140ab2f53d7729b1d933",
        ),
        (
            "important-color",
            "color",
            None,
            "abc !IMPORTANT",
            "43c318434acb42dd423bd038011f114d6e74a27a0d2f1c7a4991c7d2784e74d8",
        ),
        (
            "unsafe-color",
            "color",
            None,
            "abc;stroke:red",
            "9fc123d90428c47ade7c326dd5d49c292c8eec99863f2e4555683882df9370c7",
        ),
        (
            "unsafe-url",
            "color",
            None,
            "url(javascript:x)",
            "9fc123d90428c47ade7c326dd5d49c292c8eec99863f2e4555683882df9370c7",
        ),
    ];
    for (id, key, n, value, expected) in vectors {
        let mut engine = PublicEngine::empty();
        engine.register(QuadrantChart);
        let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
        let mut g = doc.snapshot().unwrap();
        let points = reference(g.field("points").unwrap().unwrap());
        let b = reference(&array(&g, points)[1]);
        let value = n.map_or_else(|| string(&mut g, value), Atom::Number);
        property(&mut g, b, key, value);
        let fingerprint = fingerprint(&g);
        apply(&mut doc, g);
        let r = doc
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap();
        assert_eq!((r.rendered.width, r.rendered.height), (500., 500.), "{id}");
        assert_eq!(
            format!("{:x}", Sha256::digest(r.rendered.svg.as_bytes())),
            expected,
            "{id}"
        );
        assert_eq!(fingerprint, fingerprint_graph(&doc));
    }
}
fn fingerprint_graph(doc: &PublicDocument) -> String {
    fingerprint(&doc.snapshot().unwrap())
}
struct Reject;
impl RenderResources for Reject {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new(
            "quadrant host rejected measurement",
            ErrorCode::Semantic,
        ))
    }
}
#[test]
fn mutable_quadrant_deep_graph_identity_errors_and_recovery() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted(count.clone()));
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let initial = doc.snapshot().unwrap();
    let root = root(&initial);
    let mut g = initial.clone();
    let points = reference(g.field("points").unwrap().unwrap());
    let ps = array(&g, points);
    let a = reference(&ps[0]);
    let b = reference(&ps[1]);
    let label = reference(field(&g, a, "text"));
    // Detached graph extras need neither UTF-8 conversion nor recursive traversal.
    let base = g.nodes.len() as u32;
    for i in 0..6000 {
        let next = if i == 5999 { base } else { base + i + 1 };
        let id = push_object(
            &mut g,
            vec![("next", Atom::Ref(next)), ("root", Atom::Ref(root))],
        );
        assert_eq!(id, base + i)
    }
    property(&mut g, root, "application", Atom::Ref(base));
    property(&mut g, label, "application", Atom::Ref(base));
    let key = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    let value = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xdc00]));
    properties(&mut g.nodes[a as usize]).push(Property {
        key,
        value: Atom::String(value),
        flags: 1,
    });
    for (i, n) in [
        f64::from_bits(0x7ff8000000000042),
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.,
    ]
    .into_iter()
    .enumerate()
    {
        property(&mut g, base, &format!("number{i}"), Atom::Number(n))
    }
    let classes = reference(g.field("classes").unwrap().unwrap());
    let key = string(&mut g, "unused");
    if let Node::Map { entries, .. } = &mut g.nodes[classes as usize] {
        entries.push((key, Atom::Ref(base)))
    }
    // Skipped out-of-bounds point never reads text, class Map or malformed style.
    let skipped = push_object(
        &mut g,
        vec![
            ("x", Atom::Number(f64::NAN)),
            ("y", Atom::Number(0.)),
            ("text", Atom::Ref(base)),
            ("radius", Atom::Ref(base)),
        ],
    );
    let replacement = push_array(&mut g, vec![Atom::Ref(a), Atom::Ref(b), Atom::Ref(skipped)]);
    property(&mut g, root, "points", Atom::Ref(replacement));
    let expected = fingerprint(&g);
    apply(&mut doc, g);
    same_source(&mut doc, SOURCE);
    assert_eq!(expected, fingerprint_graph(&doc));
    let flat = doc.snapshot().unwrap();
    let json = serde_json::to_string(&flat).unwrap();
    let decoded: ModelGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(expected, fingerprint(&decoded));
    apply(&mut doc, decoded);
    let mut opts = RenderOptions::default();
    opts.render.output_limit = Some(1);
    assert_eq!(
        doc.render(&opts, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(
        doc.render(&RenderOptions::default(), &mut Reject)
            .unwrap_err()
            .message,
        "quadrant host rejected measurement"
    );
    same_source(&mut doc, SOURCE);
    assert_eq!(expected, fingerprint_graph(&doc));
    // Invalid edits remain visible; recover by applying a later valid snapshot.
    let valid = doc.snapshot().unwrap();
    let mut bad = valid.clone();
    property(&mut bad, root, "points", Atom::Null);
    let invalid = fingerprint(&bad);
    apply(&mut doc, bad);
    assert!(doc
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_err());
    assert_eq!(invalid, fingerprint_graph(&doc));
    apply(&mut doc, valid.clone());
    same_source(&mut doc, SOURCE);
    let mut bad = valid.clone();
    let key = bad.strings.len() as u32;
    bad.strings.push(JsString(vec![0xd800]));
    property(&mut bad, label, "text", Atom::String(key));
    apply(&mut doc, bad);
    assert!(doc
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    apply(&mut doc, valid);
    // Undefined/deleted optional metadata, holes in quadrants and whole object
    // replacement are authored fields, never source setters or a second parse.
    let mut g = doc.snapshot().unwrap();
    let title = string(&mut g, "Mutated");
    property(&mut g, root, "title", title);
    delete(&mut g, root, "xAxisRight");
    property(&mut g, root, "accTitle", Atom::Undefined);
    let qs = reference(g.field("quadrants").unwrap().unwrap());
    delete(&mut g, qs, "0");
    let text = string(&mut g, "Replacement");
    let kind = string(&mut g, "text");
    let text = push_object(&mut g, vec![("text", text), ("type", kind)]);
    let class = string(&mut g, "");
    let point = push_object(
        &mut g,
        vec![
            ("x", Atom::Number(0.1)),
            ("y", Atom::Number(0.9)),
            ("text", Atom::Ref(text)),
            ("className", class),
        ],
    );
    let points = push_array(&mut g, vec![Atom::Ref(point)]);
    property(&mut g, root, "points", Atom::Ref(points));
    apply(&mut doc, g);
    same_source(
        &mut doc,
        "quadrantChart\ntitle Mutated\nx-axis Low\ny-axis Down --> Up\nReplacement: [0.1,0.9]",
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    // Typed and mutable public lanes use the same renderer without parsing edits.
    let parsed = Engine::with_builtins()
        .parse(SOURCE, &Default::default())
        .unwrap();
    let mut typed = parsed.model::<QuadrantModel>().unwrap().clone();
    typed.title = Some("Typed override".into());
    typed.points[0].text.text = "Quotes \" and a\nsecond line <em>safe</em>".into();
    typed.points[0].style.radius = Some(2.5);
    typed.points[1].style.stroke_width = Some("+0x20tail".into());
    let direct = QuadrantChart
        .render(
            &typed,
            &Object::default(),
            &RenderSettings::default(),
            &mut MetricsResources,
        )
        .unwrap();
    let mut owned =
        PublicDocument::from_module(Counted(count.clone()), &typed, Object::default()).unwrap();
    drop(typed);
    drop(parsed);
    drop(engine);
    assert_eq!(
        owned
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        direct
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

fn method_call(
    doc: &mut PublicDocument,
    name: &str,
    receiver: Option<Atom>,
    arguments: Vec<Atom>,
) -> InvocationOutcome {
    let id = doc
        .model_schema()
        .methods
        .iter()
        .find(|m| m.name == name)
        .unwrap()
        .id;
    let receiver = receiver.unwrap_or_else(|| doc.snapshot().unwrap().root);
    doc.invoke(
        doc.revision(),
        MethodCall {
            method: id,
            receiver,
            arguments,
        },
        &mut MetricsResources,
    )
    .unwrap()
}
fn expect_void(out: InvocationOutcome) -> ModelGraph {
    assert!(matches!(out.result, Ok(Atom::Undefined)));
    out.graph
}
fn label_argument(g: &mut ModelGraph, text: &str, kind: &str) -> u32 {
    let text = string(g, text);
    let kind = string(g, kind);
    push_object(g, vec![("text", text), ("type", kind)])
}
fn styles_argument(g: &mut ModelGraph, styles: &[&str]) -> u32 {
    let values = styles.iter().map(|s| string(g, s)).collect();
    push_array(g, values)
}
#[test]
fn quadrant_native_builder_methods_keep_fresh_labels_map_replacement_and_render_provenance() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted(count.clone()));
    let mut doc = engine.parse("quadrantChart", &Default::default()).unwrap();
    let schema = doc.model_schema();
    assert_eq!(schema.id, "pele/quadrantChart@1");
    let prototype = &schema.prototypes[0];
    assert_eq!(prototype.name, "QuadrantDb");
    let expected = [
        ("setQuadrant1Text", 1),
        ("setQuadrant2Text", 1),
        ("setQuadrant3Text", 1),
        ("setQuadrant4Text", 1),
        ("setXAxisLeftText", 1),
        ("setXAxisRightText", 1),
        ("setYAxisBottomText", 1),
        ("setYAxisTopText", 1),
        ("addPoint", 5),
        ("addClass", 2),
        ("setDiagramTitle", 1),
        ("setAccTitle", 1),
        ("setAccDescription", 1),
    ];
    assert_eq!(
        prototype.members.iter().map(|m| m.name).collect::<Vec<_>>(),
        expected.map(|(n, _)| n)
    );
    assert!(prototype.members.iter().all(|m| m.flags == 6));
    assert_eq!(
        schema
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        expected
    );
    let mut g = doc.snapshot().unwrap();
    let root_id = root(&g);
    assert!(matches!(g.nodes[root_id as usize], Node::Instance { .. }));
    assert_eq!(g.root_properties().unwrap().len(), 11);
    let label = label_argument(&mut g, " \u{feff}Text \n ", "markdown");
    let point_styles = styles_argument(
        &mut g,
        &[
            "color: #f00",
            "radius: 3",
            "radius: 7",
            "stroke-color: abc",
            "stroke-width: 2px",
        ],
    );
    let class_styles = styles_argument(&mut g, &["radius: 11", "color: #0f0"]);
    let class = string(&mut g, "cc");
    let x = string(&mut g, "0x0");
    let y = string(&mut g, " .5 ");
    apply(&mut doc, g);
    for (name, _) in &expected[..8] {
        expect_void(method_call(&mut doc, name, None, vec![Atom::Ref(label)]));
    }
    let g = expect_void(method_call(
        &mut doc,
        "addPoint",
        None,
        vec![
            Atom::Ref(label),
            class.clone(),
            x,
            y,
            Atom::Ref(point_styles),
        ],
    ));
    let points = reference(g.field("points").unwrap().unwrap());
    let point = reference(&array(&g, points)[0]);
    let point_text = reference(field(&g, point, "text"));
    assert_ne!(point_text, label);
    let qs = reference(g.field("quadrants").unwrap().unwrap());
    let labels = array(&g, qs)
        .into_iter()
        .map(|a| reference(&a))
        .collect::<Vec<_>>();
    for (i, id) in labels.iter().enumerate() {
        assert_ne!(*id, label);
        assert_ne!(*id, point_text);
        assert!(!labels[..i].contains(id))
    }
    assert_eq!(
        g.text(field(&g, point_text, "text"))
            .unwrap()
            .well_formed()
            .unwrap(),
        "Text"
    );
    assert_eq!(
        g.nodes[point as usize]
            .properties()
            .iter()
            .map(|p| g.strings[p.key as usize].well_formed().unwrap())
            .collect::<Vec<_>>(),
        [
            "x",
            "y",
            "text",
            "className",
            "color",
            "radius",
            "strokeColor",
            "strokeWidth"
        ]
    );
    let g = expect_void(method_call(
        &mut doc,
        "addClass",
        None,
        vec![class.clone(), Atom::Ref(class_styles)],
    ));
    let classes = reference(g.field("classes").unwrap().unwrap());
    let Node::Map { entries, .. } = &g.nodes[classes as usize] else {
        panic!()
    };
    let old_style = reference(&entries[0].1);
    assert_ne!(old_style, point);
    let mut g = g;
    let title = string(&mut g, "Built title");
    let acc = string(&mut g, " \t Accessible");
    let desc = string(&mut g, "First\n \n  second\n\tthird");
    apply(&mut doc, g);
    expect_void(method_call(&mut doc, "setDiagramTitle", None, vec![title]));
    expect_void(method_call(&mut doc, "setAccTitle", None, vec![acc]));
    expect_void(method_call(&mut doc, "setAccDescription", None, vec![desc]));
    let expected_source="quadrantChart\ntitle Built title\naccTitle: Accessible\naccDescr { First\nsecond\nthird }\nx-axis \"`Text`\" --> \"`Text`\"\ny-axis \"`Text`\" --> \"`Text`\"\nquadrant-1 \"`Text`\"\nquadrant-2 \"`Text`\"\nquadrant-3 \"`Text`\"\nquadrant-4 \"`Text`\"\n\"`Text`\":::cc: [0.0,0.5] color:#f00,radius:7,stroke-color:abc,stroke-width:2px\nclassDef cc radius:11,color:#0f0";
    same_source(&mut doc, expected_source);
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(
                doc.render(&RenderOptions::default(), &mut MetricsResources)
                    .unwrap()
                    .rendered
                    .svg
                    .as_bytes()
            )
        ),
        "92aed5b5c93134adb214c99b0311ee91931dec3207401159f58280233d1b3370"
    );
    // Replacing an existing class allocates a fresh style; point own styles and
    // detached references to the prior class object stay unchanged.
    let mut g = doc.snapshot().unwrap();
    let new_styles = styles_argument(&mut g, &["radius:5", "color:#00f"]);
    apply(&mut doc, g);
    let g = expect_void(method_call(
        &mut doc,
        "addClass",
        None,
        vec![class, Atom::Ref(new_styles)],
    ));
    let Node::Map { entries, .. } = &g.nodes[classes as usize] else {
        panic!()
    };
    assert_ne!(reference(&entries[0].1), old_style);
    assert_eq!(
        g.text(field(&g, old_style, "color"))
            .unwrap()
            .well_formed()
            .unwrap(),
        "#0f0"
    );
    same_source(
        &mut doc,
        &expected_source.replace(
            "classDef cc radius:11,color:#0f0",
            "classDef cc radius:5,color:#00f",
        ),
    );
    // Borrowed real methods use the supplied receiver's collection aliases.
    let mut g = doc.snapshot().unwrap();
    let holder = push_object(
        &mut g,
        vec![
            ("classes", Atom::Ref(classes)),
            ("points", Atom::Ref(points)),
        ],
    );
    let empty = push_array(&mut g, vec![]);
    let x = string(&mut g, "0.3");
    let y = string(&mut g, "0.9");
    let class = field(&g, point, "className").clone();
    apply(&mut doc, g);
    let g = expect_void(method_call(
        &mut doc,
        "addClass",
        Some(Atom::Ref(holder)),
        vec![Atom::Ref(label), Atom::Ref(new_styles)],
    ));
    let Node::Map { entries, .. } = &g.nodes[classes as usize] else {
        panic!()
    };
    assert!(matches!(entries.last().unwrap().0,Atom::Ref(id) if id==label));
    expect_void(method_call(
        &mut doc,
        "addPoint",
        Some(Atom::Ref(holder)),
        vec![Atom::Ref(label), class, x, y, Atom::Ref(empty)],
    ));
    same_source(
        &mut doc,
        &(expected_source.replace(
            "classDef cc radius:11,color:#0f0",
            "classDef cc radius:5,color:#00f",
        ) + "\n\"`Text`\":::cc: [0.3,0.9]"),
    );
    engine.register(QuadrantChart);
    drop(engine);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(doc.diagram_type().as_str(), "quadrantChart");
}
#[test]
fn quadrant_method_utf16_storage_semantic_errors_readonly_fields_and_recovery() {
    let mut engine = PublicEngine::empty();
    engine.register(QuadrantChart);
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let mut g = doc.snapshot().unwrap();
    let root_id = root(&g);
    let label = label_argument(&mut g, " A ", "text");
    let class = string(&mut g, "cc");
    let valid = styles_argument(&mut g, &["radius:9"]);
    let invalid = styles_argument(&mut g, &["radius:bad"]);
    let zero = string(&mut g, "0");
    apply(&mut doc, g);
    let original = doc.snapshot().unwrap();
    let before = fingerprint(&original);
    let revision = doc.revision();
    let out = method_call(
        &mut doc,
        "addClass",
        None,
        vec![class.clone(), Atom::Ref(invalid)],
    );
    assert_eq!(out.revision, revision + 1);
    assert_eq!(before, fingerprint(&out.graph));
    assert!(
        matches!(out.result,Err(PublicError::Pele(PeleError{message,..})) if message=="value for radius bad is invalid, please use a valid number")
    );
    // A malformed style is a real PeleError whose message itself may be ill-formed.
    let mut g = doc.snapshot().unwrap();
    let id = g.strings.len() as u32;
    let mut units = JsString::from("color:abc").0;
    units.push(0xd800);
    g.strings.push(JsString(units));
    let bad = push_array(&mut g, vec![Atom::String(id)]);
    apply(&mut doc, g);
    let before = fingerprint_graph(&doc);
    let out = method_call(
        &mut doc,
        "addClass",
        None,
        vec![class.clone(), Atom::Ref(bad)],
    );
    assert_eq!(before, fingerprint(&out.graph));
    let Err(PublicError::PeleUtf16(error)) = &out.result else {
        panic!("exact UTF16 PeleError")
    };
    let mut expected = JsString::from("value for color abc").0;
    expected.push(0xd800);
    expected.extend(JsString::from(" is invalid, please use a valid hex code").0);
    assert_eq!(error.message.0, expected);
    assert_eq!(error.name(), "PeleError");
    assert_eq!(error.code, ErrorCode::Semantic);
    assert!(error.diagram_type.as_ref().unwrap().is("quadrantChart"));
    assert_eq!((error.line, error.column), (0, 0));
    assert!(error.snippet.0.is_empty());
    let roundtrip: InvocationOutcome =
        serde_json::from_str(&serde_json::to_string(&out).unwrap()).unwrap();
    assert!(
        matches!(roundtrip.result,Err(PublicError::PeleUtf16(error)) if error.message.0==expected)
    );
    same_source(&mut doc, SOURCE);
    // UTF16 trimming/storage succeeds. Only rendering the active label is limited.
    let mut g = doc.snapshot().unwrap();
    let id = g.strings.len() as u32;
    g.strings.push(JsString(vec![32, 0xd800, 65, 32]));
    let kind = string(&mut g, "text");
    let lone = push_object(&mut g, vec![("text", Atom::String(id)), ("type", kind)]);
    apply(&mut doc, g);
    let out = expect_void(method_call(
        &mut doc,
        "setXAxisLeftText",
        None,
        vec![Atom::Ref(lone)],
    ));
    let left = reference(out.field("xAxisLeft").unwrap().unwrap());
    assert_eq!(out.text(field(&out, left, "text")).unwrap().0, [0xd800, 65]);
    assert!(doc
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    apply(&mut doc, original.clone());
    same_source(&mut doc, SOURCE);
    let mut g = doc.snapshot().unwrap();
    let acc = g.strings.len() as u32;
    g.strings.push(JsString(vec![32, 0xd800, 65, 32]));
    let desc = g.strings.len() as u32;
    g.strings
        .push(JsString(vec![32, 65, 10, 32, 0xd800, 10, 9, 66, 32]));
    apply(&mut doc, g);
    let g = expect_void(method_call(
        &mut doc,
        "setAccTitle",
        None,
        vec![Atom::String(acc)],
    ));
    assert_eq!(
        g.text(g.field("accTitle").unwrap().unwrap()).unwrap().0,
        [0xd800, 65, 32]
    );
    let g = expect_void(method_call(
        &mut doc,
        "setAccDescription",
        None,
        vec![Atom::String(desc)],
    ));
    assert_eq!(
        g.text(g.field("accDescr").unwrap().unwrap()).unwrap().0,
        [32, 65, 10, 0xd800, 10, 66, 32]
    );
    apply(&mut doc, original.clone());
    // Readonly root collection properties still permit mutation of their contents.
    let mut g = doc.snapshot().unwrap();
    for p in g.nodes[root_id as usize].properties_mut() {
        if ["quadrants", "points", "classes"]
            .iter()
            .any(|k| g.strings[p.key as usize].is(k))
        {
            p.flags = 1
        }
    }
    apply(&mut doc, g);
    expect_void(method_call(
        &mut doc,
        "setQuadrant1Text",
        None,
        vec![Atom::Ref(label)],
    ));
    expect_void(method_call(
        &mut doc,
        "addClass",
        None,
        vec![class.clone(), Atom::Ref(valid)],
    ));
    expect_void(method_call(
        &mut doc,
        "addPoint",
        None,
        vec![
            Atom::Ref(label),
            class.clone(),
            zero.clone(),
            zero.clone(),
            Atom::Ref(valid),
        ],
    ));
    let mut g = doc.snapshot().unwrap();
    let qs = reference(g.field("quadrants").unwrap().unwrap());
    g.nodes[qs as usize]
        .properties_mut()
        .iter_mut()
        .find(|p| g.strings[p.key as usize].is("0"))
        .unwrap()
        .flags = 1;
    apply(&mut doc, g);
    let before = fingerprint_graph(&doc);
    let out = method_call(&mut doc, "setQuadrant1Text", None, vec![Atom::Ref(label)]);
    assert_eq!(before, fingerprint(&out.graph));
    assert!(matches!(
        out.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    // Null collection property access precedes argument processing for call sites.
    let mut bad = original.clone();
    property(&mut bad, root_id, "points", Atom::Null);
    apply(&mut doc, bad);
    let before = fingerprint_graph(&doc);
    let out = method_call(
        &mut doc,
        "addPoint",
        None,
        vec![
            Atom::Null,
            class.clone(),
            zero.clone(),
            zero.clone(),
            Atom::Ref(invalid),
        ],
    );
    assert_eq!(before, fingerprint(&out.graph));
    assert!(
        matches!(out.result,Err(PublicError::JavaScript{message,..}) if message.is("Cannot read properties of null (reading 'push')"))
    );
    let mut bad = original.clone();
    property(&mut bad, root_id, "classes", Atom::Null);
    apply(&mut doc, bad);
    let out = method_call(
        &mut doc,
        "addClass",
        None,
        vec![class.clone(), Atom::Ref(invalid)],
    );
    assert!(
        matches!(out.result,Err(PublicError::JavaScript{message,..}) if message.is("Cannot read properties of null (reading 'set')"))
    );
    // A non-callable override is checked after argument evaluation, so style
    // errors win first. Restoring ordinary data recovers without a new parse.
    let mut bad = original.clone();
    let classes = reference(bad.field("classes").unwrap().unwrap());
    property(&mut bad, classes, "set", Atom::Number(1.));
    apply(&mut doc, bad);
    let out = method_call(
        &mut doc,
        "addClass",
        None,
        vec![class.clone(), Atom::Ref(invalid)],
    );
    assert!(matches!(out.result, Err(PublicError::Pele(_))));
    let out = method_call(&mut doc, "addClass", None, vec![class, Atom::Ref(valid)]);
    assert!(
        matches!(out.result,Err(PublicError::JavaScript{message,..}) if message.is("this.classes.set is not a function"))
    );
    apply(&mut doc, original);
    same_source(&mut doc, SOURCE);
}

#[test]
fn quadrant_native_method_numeric_conversion_and_styles_match_pinned_models_and_svg() {
    // Oracle: addPoint({text:' P ',type:'text'},'k',x,'.5',styles), render(model).
    let vectors = [
        (
            "-0",
            0x8000000000000000u64,
            "5dbba9847831e0311a6296f00de01b4184415bb8cb2b9b3477a60bb9a80d1078",
        ),
        (
            "0",
            0x0000000000000000u64,
            "5dbba9847831e0311a6296f00de01b4184415bb8cb2b9b3477a60bb9a80d1078",
        ),
        (
            "0x1",
            0x3ff0000000000000u64,
            "253be8ab84e850290d28685682af38915e45e612ae40c34a693faacdc8dba969",
        ),
        (
            "0b1",
            0x3ff0000000000000u64,
            "253be8ab84e850290d28685682af38915e45e612ae40c34a693faacdc8dba969",
        ),
        (
            "0o1",
            0x3ff0000000000000u64,
            "253be8ab84e850290d28685682af38915e45e612ae40c34a693faacdc8dba969",
        ),
        (
            " .5 ",
            0x3fe0000000000000u64,
            "f8ba635b21095ef8aaa0795ed7e864d44b539cf874343496fb08d7a9c1428144",
        ),
        (
            "5e-1",
            0x3fe0000000000000u64,
            "f8ba635b21095ef8aaa0795ed7e864d44b539cf874343496fb08d7a9c1428144",
        ),
        (
            "",
            0x0000000000000000u64,
            "5dbba9847831e0311a6296f00de01b4184415bb8cb2b9b3477a60bb9a80d1078",
        ),
        (
            "﻿　 0.2 ",
            0x3fc999999999999au64,
            "95d814033fc11df1b3a7b679d2a8b404f14489d5f6cff1948d28487b4f10a4d1",
        ),
        (
            "Infinity",
            0x7ff0000000000000u64,
            "3a0fd66a343b8264e615c2355792d92a8cbad400272d324a8c80b3404563e5d0",
        ),
        (
            "-Infinity",
            0xfff0000000000000u64,
            "3a0fd66a343b8264e615c2355792d92a8cbad400272d324a8c80b3404563e5d0",
        ),
        (
            "NaN",
            0x7ff8000000000000u64,
            "3a0fd66a343b8264e615c2355792d92a8cbad400272d324a8c80b3404563e5d0",
        ),
        (
            "0x20000000000001",
            0x4340000000000000u64,
            "3a0fd66a343b8264e615c2355792d92a8cbad400272d324a8c80b3404563e5d0",
        ),
        (
            "0b111111111111111111111111111111111111111111111111111111",
            0x4350000000000000u64,
            "3a0fd66a343b8264e615c2355792d92a8cbad400272d324a8c80b3404563e5d0",
        ),
        (
            "1_0",
            0x7ff8000000000000u64,
            "3a0fd66a343b8264e615c2355792d92a8cbad400272d324a8c80b3404563e5d0",
        ),
        (
            "1e309",
            0x7ff0000000000000u64,
            "3a0fd66a343b8264e615c2355792d92a8cbad400272d324a8c80b3404563e5d0",
        ),
        (
            "1e-324",
            0x0000000000000000u64,
            "5dbba9847831e0311a6296f00de01b4184415bb8cb2b9b3477a60bb9a80d1078",
        ),
        (
            "4.9406564584124654e-324",
            0x0000000000000001u64,
            "5dbba9847831e0311a6296f00de01b4184415bb8cb2b9b3477a60bb9a80d1078",
        ),
    ];
    for (x, bits, expected) in vectors {
        let mut engine = PublicEngine::empty();
        engine.register(QuadrantChart);
        let mut doc = engine
            .parse(
                "quadrantChart\nx-axis L --> R\ny-axis D --> U",
                &Default::default(),
            )
            .unwrap();
        let mut g = doc.snapshot().unwrap();
        let label = label_argument(&mut g, " P ", "text");
        let class = string(&mut g, "k");
        let x = string(&mut g, x);
        let y = string(&mut g, ".5");
        let styles = styles_argument(
            &mut g,
            &[
                " radius : 0009 : ignored",
                "color:AbC",
                "stroke-color:#abcdef",
                "stroke-width:004px",
            ],
        );
        apply(&mut doc, g);
        let out = expect_void(method_call(
            &mut doc,
            "addPoint",
            None,
            vec![Atom::Ref(label), class, x, y, Atom::Ref(styles)],
        ));
        let points = reference(out.field("points").unwrap().unwrap());
        let point = reference(&array(&out, points)[0]);
        let Atom::Number(x) = field(&out, point, "x") else {
            panic!()
        };
        assert_eq!(x.to_bits(), bits);
        let Atom::Number(radius) = field(&out, point, "radius") else {
            panic!()
        };
        assert_eq!(*radius, 9.);
        let rendered = doc
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(rendered.rendered.svg.as_bytes())),
            expected
        );
    }
}
