//! Mutable Packet feature against pinned Pele 1.6.0, 458322a5.
//! Public oracle: node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! model.ts ff920c719904f57bc2a1375a454d23e65faa968c80f7b308cf21e751a536f12d;
//! render.ts 58477d4d069a41da775f13afb1f3fadb73d928cddaa078d0e19178d94ed7cb73.
//! Probe: Object.getPrototypeOf(parse(source))===Object.prototype, all own
//! descriptors flags7; Reflect.ownKeys root/block asserted below. No custom
//! Packet methods or prototype getters. Split pieces are distinct objects.
use pele::compat::{
    diagrams::packet::{Packet, PacketModel},
    public::{Atom, JsString, ModelGraph, Node, Property, PublicDiagramModule, PublicEngine},
    text::MetricsResources,
    *,
};
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
const SOURCE: &str =
    "packet\ntitle Before\naccTitle: Accessible\naccDescr: Details\n+64: \"Shared\"\n+8: \"Tail\"";
fn reference(atom: &Atom) -> u32 {
    if let Atom::Ref(id) = atom {
        *id
    } else {
        panic!("object reference")
    }
}
fn root(g: &ModelGraph) -> u32 {
    reference(&g.root)
}
fn field<'a>(g: &'a ModelGraph, id: u32, key: &str) -> &'a Atom {
    &g.nodes[id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(key))
        .unwrap()
        .value
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
fn set(g: &mut ModelGraph, id: u32, key: &str, value: Atom) {
    if let Some(i) = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key))
    {
        properties(&mut g.nodes[id as usize])[i].value = value;
        return;
    }
    let key_id = g.strings.len() as u32;
    g.strings.push(key.into());
    properties(&mut g.nodes[id as usize]).push(Property {
        key: key_id,
        value,
        flags: 7,
    });
}
fn delete(g: &mut ModelGraph, id: u32, key: &str) {
    if let Some(i) = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key))
    {
        properties(&mut g.nodes[id as usize]).remove(i);
    }
}
fn string(g: &mut ModelGraph, text: &str) -> Atom {
    let id = g.strings.len() as u32;
    g.strings.push(text.into());
    Atom::String(id)
}
fn set_text(g: &mut ModelGraph, id: u32, key: &str, text: &str) {
    let atom = string(g, text);
    set(g, id, key, atom)
}
fn object(g: &mut ModelGraph) -> u32 {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    id
}
fn array(g: &mut ModelGraph, values: Vec<Atom>) -> u32 {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: values.len() as u32,
        properties: vec![],
    });
    for (i, v) in values.into_iter().enumerate() {
        set(g, id, &i.to_string(), v)
    }
    id
}
fn row_and_block(g: &ModelGraph) -> (u32, u32, u32) {
    let rows = reference(g.field("rows").unwrap().unwrap());
    let row = reference(field(g, rows, "0"));
    let block = reference(field(g, row, "0"));
    (rows, row, block)
}
fn fingerprint(g: &ModelGraph) -> String {
    serde_json::to_string(g).unwrap()
}
fn hash(svg: &str) -> String {
    format!("{:x}", Sha256::digest(svg.as_bytes()))
}

struct Counted {
    parses: Arc<AtomicUsize>,
    renders: Arc<AtomicUsize>,
}
impl DiagramModule for Counted {
    type Model = PacketModel;
    fn descriptor(&self) -> ModuleDescriptor {
        Packet.descriptor()
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Self::Model, PeleError> {
        self.parses.fetch_add(1, Ordering::SeqCst);
        Packet.parse(c)
    }
    fn render(
        &self,
        m: &Self::Model,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Packet.render(m, c, o, h)
    }
}
impl PublicDiagramModule for Counted {
    type PublicState = <Packet as PublicDiagramModule>::PublicState;
    fn create_public(&self, m: &Self::Model) -> Result<Self::PublicState, PeleError> {
        Packet.create_public(m)
    }
    fn export_public(&self, s: &Self::PublicState) -> Result<ModelGraph, PeleError> {
        Packet.export_public(s)
    }
    fn apply_public(&self, s: &mut Self::PublicState, g: ModelGraph) -> Result<(), PeleError> {
        Packet.apply_public(s, g)
    }
    fn render_public(
        &self,
        s: &mut Self::PublicState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        Packet.render_public(s, c, o, h)
    }
}
#[test]
fn packet_fields_source_equivalent_edits_and_retained_registration() {
    let parses = Arc::new(AtomicUsize::new(0));
    let renders = Arc::new(AtomicUsize::new(0));
    let mut engine = PublicEngine::empty();
    engine.register(Counted {
        parses: parses.clone(),
        renders: renders.clone(),
    });
    let empty = engine
        .parse("packet", &Default::default())
        .unwrap()
        .snapshot()
        .unwrap();
    for key in ["title", "accTitle", "accDescr"] {
        assert!(matches!(empty.field(key).unwrap(), Some(Atom::Undefined)));
    }
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let untouched = engine.parse(SOURCE, &Default::default()).unwrap();
    let g = doc.snapshot().unwrap();
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
            "bitsPerRow",
            "rows"
        ]
    );
    assert!(g.root_properties().unwrap().iter().all(|p| p.flags == 7));
    let (rows, row, b) = row_and_block(&g);
    let second = reference(field(&g, rows, "1"));
    let second_block = reference(field(&g, second, "0"));
    assert_ne!(row, second);
    assert_ne!(b, second_block);
    assert_eq!(
        g.nodes[b as usize]
            .properties()
            .iter()
            .map(|p| g.strings[p.key as usize].well_formed().unwrap())
            .collect::<Vec<_>>(),
        ["start", "end", "bits", "label"]
    );
    assert!(
        matches!((field(&g,b,"label"),field(&g,second_block,"label")),(Atom::String(a),Atom::String(b))if a==b)
    );
    let initial = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        initial,
        Engine::with_builtins()
            .render_source(SOURCE, &Default::default(), &mut MetricsResources)
            .unwrap()
    );
    let mut g = g;
    let r = root(&g);
    set_text(&mut g, r, "type", "unrelated-type");
    set_text(&mut g, r, "title", "Edited");
    set_text(&mut g, r, "accTitle", "New accessible");
    set(&mut g, r, "accDescr", Atom::Undefined);
    set_text(&mut g, b, "label", "Edited field");
    doc.apply(0, g).unwrap();
    let replacement_renders = Arc::new(AtomicUsize::new(0));
    engine.register(Counted {
        parses: parses.clone(),
        renders: replacement_renders.clone(),
    });
    drop(engine);
    let expected_source="packet\ntitle Edited\naccTitle: New accessible\n+32: \"Edited field\"\n+32: \"Shared\"\n+8: \"Tail\"";
    let mut options = RenderOptions::default();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    for max_width in [None, Some(180.)] {
        options.render.max_width = max_width;
        let edited = doc.render(&options, &mut MetricsResources).unwrap();
        let mut expected_options = RenderOptions::default();
        expected_options.render.max_width = max_width;
        assert_eq!(
            edited,
            Engine::with_builtins()
                .render_source(expected_source, &expected_options, &mut MetricsResources)
                .unwrap()
        );
        assert_eq!(edited.diagram_type, DiagramId::Builtin(DiagramType::Packet));
    }
    assert_eq!(parses.load(Ordering::SeqCst), 3);
    assert_eq!(renders.load(Ordering::SeqCst), 3);
    assert_eq!(replacement_renders.load(Ordering::SeqCst), 0);
    let untouched_graph = untouched.snapshot().unwrap();
    let Atom::String(title) = untouched_graph.field("title").unwrap().unwrap() else {
        panic!()
    };
    assert_eq!(
        untouched_graph.strings[*title as usize]
            .well_formed()
            .unwrap(),
        "Before"
    );
    assert_eq!(doc.revision(), 1);
}

#[test]
fn packet_aliases_replacements_sparse_arrays_and_ieee_mutations_match_pinned_svg() {
    // Exact probe scripts: parse(SOURCE), apply the edit in this match, render(m,
    // {maxWidth:180} for responsive cases). Hashes taken from pinned main entry.
    let probes = [
        (
            "original",
            "bae9fb277a6d10e448dfd5d677d2ac3f22b1173df8fada91882a522109724fd6",
        ),
        (
            "alias-rows",
            "6c73873a266d43a2d61a9fe8bc1a53744e0cf84b61f877410232df415efc0cf6",
        ),
        (
            "alias-block",
            "7b30dacceb707923d3a5081051af2ebe999978867171b75054cb8011d4c959af",
        ),
        (
            "outer-hole",
            "cb8e754f7b29ac94aef726ab0da77a43c9c9d1cd08d1f15941915d3a176d479e",
        ),
        (
            "outer-trailing-hole",
            "118b35ece099f4d57add5128cd4a09b75cfce2215e6d835faec9a586cb32f433",
        ),
        (
            "sparse-empty",
            "65cf8ddc5d6f96d83925a7dcbb01721c85507e9c524b6fa30db99bd975041d21",
        ),
        (
            "missing-bits-per-row",
            "01d7853509e7ce73d48e00a325076001a81e50c00bfcdccdc5f88e3bcd2a347d",
        ),
        (
            "nan-bits-per-row",
            "01d7853509e7ce73d48e00a325076001a81e50c00bfcdccdc5f88e3bcd2a347d",
        ),
        (
            "minus-zero-bits-per-row",
            "f5d74405de741e4d84c067af5f23806e3d4485b44677489f8418c87f0913edfe",
        ),
        (
            "positive-infinity-bits-per-row",
            "fce83a261414e58e58a6aead11c960cdfe4444ce0d1da3ad2fcc38fdcb2ff032",
        ),
        (
            "negative-infinity-bits-per-row",
            "92223e0797b93b464cf986f2d3320caca037422cc7ae73cbd9db261e1dff393a",
        ),
        (
            "negative-responsive",
            "7b924228e1c44c40e281c3926fa909c76b7c8f0a7e0da76a09d06b4d95c34ad4",
        ),
        (
            "alias-responsive",
            "e817f62016713f6c4ee21b97bff314034557c047f6d2b742bdf938b5eb63da92",
        ),
    ];
    let mut engine = PublicEngine::empty();
    engine.register(Packet);
    for (name, sha) in probes {
        let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
        let mut g = doc.snapshot().unwrap();
        let r = root(&g);
        let (rows, row, b) = row_and_block(&g);
        match name {
            "alias-rows" => {
                let a = array(&mut g, vec![Atom::Ref(row), Atom::Ref(row)]);
                set(&mut g, r, "rows", Atom::Ref(a));
                set_text(&mut g, b, "label", "Aliased");
            }
            "alias-block" | "alias-responsive" => {
                let a = array(&mut g, vec![Atom::Ref(b), Atom::Ref(b)]);
                let a = array(&mut g, vec![Atom::Ref(a)]);
                set(&mut g, r, "rows", Atom::Ref(a));
                set_text(&mut g, b, "label", "Alias block");
            }
            "negative-responsive" => {
                set(&mut g, b, "start", Atom::Number(-8.));
                set(&mut g, b, "end", Atom::Number(7.));
                let a = array(&mut g, vec![Atom::Ref(b)]);
                let a = array(&mut g, vec![Atom::Ref(a)]);
                set(&mut g, r, "rows", Atom::Ref(a));
            }
            "outer-hole" => delete(&mut g, rows, "0"),
            "outer-trailing-hole" => {
                let Node::Array { length, .. } = &mut g.nodes[rows as usize] else {
                    panic!()
                };
                *length = 4
            }
            "sparse-empty" => {
                let empty = array(&mut g, vec![]);
                let a = array(&mut g, vec![Atom::Undefined, Atom::Ref(empty)]);
                delete(&mut g, a, "0");
                set(&mut g, r, "rows", Atom::Ref(a));
            }
            "missing-bits-per-row" => delete(&mut g, r, "bitsPerRow"),
            "nan-bits-per-row" => set(&mut g, r, "bitsPerRow", Atom::Number(f64::NAN)),
            "positive-infinity-bits-per-row" => {
                set(&mut g, r, "bitsPerRow", Atom::Number(f64::INFINITY))
            }
            "negative-infinity-bits-per-row" => {
                set(&mut g, r, "bitsPerRow", Atom::Number(f64::NEG_INFINITY))
            }
            "minus-zero-bits-per-row" => set(&mut g, r, "bitsPerRow", Atom::Number(-0.)),
            _ => {}
        }
        let before = fingerprint(&g);
        doc.apply(0, g).unwrap();
        let mut options = RenderOptions::default();
        if name.ends_with("responsive") {
            options.render.max_width = Some(180.)
        }
        let result = doc.render(&options, &mut MetricsResources).unwrap();
        assert_eq!(hash(&result.rendered.svg), sha, "{name}");
        assert_eq!(
            fingerprint(&doc.snapshot().unwrap()),
            before,
            "render must preserve graph identity {name}"
        );
        let again = doc.render(&options, &mut MetricsResources).unwrap();
        assert_eq!(hash(&again.rendered.svg), sha, "second render {name}");
        if name.starts_with("outer-") || name == "sparse-empty" {
            assert!(result.rendered.height.is_nan());
        }
        if name == "nan-bits-per-row" || name == "missing-bits-per-row" {
            assert!(result.rendered.width.is_nan());
        }
    }
}

struct FailingHost;
impl RenderResources for FailingHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("Packet host stopped", ErrorCode::Semantic))
    }
}
#[test]
fn packet_edit_errors_host_failures_and_output_limits_keep_owned_state_and_recover() {
    let mut engine = PublicEngine::empty();
    engine.register(Packet);
    let mut doc = engine.parse(SOURCE, &Default::default()).unwrap();
    let original = doc.snapshot().unwrap();
    let expected = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    let mut g = original.clone();
    let (rows, _, _) = row_and_block(&g);
    delete(&mut g, rows, "0");
    doc.apply(0, g).unwrap();
    let before = fingerprint(&doc.snapshot().unwrap());
    let mut options = RenderOptions::default();
    options.render.max_width = Some(180.);
    assert_eq!(
        doc.render(&options, &mut MetricsResources)
            .unwrap_err()
            .message,
        "row is not iterable"
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), before);
    let mut g = original.clone();
    let (_, row, _) = row_and_block(&g);
    delete(&mut g, row, "0");
    doc.apply(1, g).unwrap();
    assert_eq!(
        doc.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .message,
        "Cannot read properties of undefined (reading 'end')"
    );
    let mut gap = engine.parse(SOURCE, &Default::default()).unwrap();
    let mut g = gap.snapshot().unwrap();
    let r = root(&g);
    let (_, _, b) = row_and_block(&g);
    set(&mut g, b, "start", Atom::Number(32.));
    set(&mut g, b, "end", Atom::Number(63.));
    let a = array(&mut g, vec![Atom::Ref(b)]);
    let a = array(&mut g, vec![Atom::Ref(a)]);
    set(&mut g, r, "rows", Atom::Ref(a));
    gap.apply(0, g).unwrap();
    assert_eq!(
        gap.render(&options, &mut MetricsResources)
            .unwrap_err()
            .message,
        "row is not iterable"
    );
    doc.apply(2, original.clone()).unwrap();
    let before = fingerprint(&doc.snapshot().unwrap());
    assert_eq!(
        doc.render(&Default::default(), &mut FailingHost)
            .unwrap_err()
            .message,
        "Packet host stopped"
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), before);
    options.render.max_width = None;
    options.render.output_limit = Some(1);
    assert_eq!(
        doc.render(&options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(fingerprint(&doc.snapshot().unwrap()), before);
    options.render.output_limit = None;
    assert_eq!(
        doc.render(&options, &mut MetricsResources).unwrap(),
        expected
    );
    assert!(doc.apply(0, original.clone()).is_err());
    assert_eq!(doc.revision(), 3);
    let mut malformed = original;
    let r = root(&malformed);
    set(&mut malformed, r, "rows", Atom::Ref(u32::MAX));
    assert!(doc.apply(3, malformed).is_err());
    assert_eq!(doc.revision(), 3);
}

#[test]
fn packet_lossless_deep_cyclic_extras_and_shared_long_labels_remain_compact() {
    let label = "Repeated label ".repeat(5000);
    let source = format!("packet\n+6400: \"{label}\"");
    let mut engine = PublicEngine::empty();
    engine.register(Packet);
    let mut doc = engine
        .parse(
            &source,
            &ParseOptions {
                preparation: PrepareOptions {
                    limit: 1_000_000,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    let mut g = doc.snapshot().unwrap();
    // 200 split pieces share one primitive string entry, never 200 expanded paths.
    assert_eq!(
        g.strings
            .iter()
            .filter(|s| s.0.len() == label.encode_utf16().count())
            .count(),
        1
    );
    assert!(serde_json::to_vec(&g).unwrap().len() < 2_000_000);
    let r = root(&g);
    let mut cursor = r;
    for _ in 0..20_000 {
        let next = object(&mut g);
        set(&mut g, cursor, "applicationData", Atom::Ref(next));
        cursor = next;
    }
    set(&mut g, cursor, "cycle", Atom::Ref(r));
    let lone = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 0x61, 0xdc00]));
    set(&mut g, cursor, "exactUtf16", Atom::String(lone));
    let nan = f64::from_bits(0x7ff8_0000_0000_0042);
    for (key, value) in [
        ("nan", nan),
        ("negativeZero", -0.),
        ("positiveInfinity", f64::INFINITY),
        ("negativeInfinity", f64::NEG_INFINITY),
    ] {
        set(&mut g, cursor, key, Atom::Number(value));
    }
    let encoded = serde_json::to_vec(&g).unwrap();
    let restored: ModelGraph = serde_json::from_slice(&encoded).unwrap();
    doc.apply(0, restored).unwrap();
    let snapshot = doc.snapshot().unwrap();
    assert_eq!(fingerprint(&snapshot), fingerprint(&g));
    assert_eq!(reference(field(&snapshot, cursor, "cycle")), r);
    assert!(matches!(field(&snapshot,cursor,"nan"),Atom::Number(n)if n.to_bits()==nan.to_bits()));
    assert!(
        matches!(field(&snapshot,cursor,"negativeZero"),Atom::Number(n)if n.to_bits()==(-0f64).to_bits())
    );
    // Render a bounded title/row edit while preserving the complete detached graph.
    let mut edited = snapshot;
    let rows = array(&mut edited, vec![]);
    set(&mut edited, r, "rows", Atom::Ref(rows));
    doc.apply(1, edited).unwrap();
    let before = fingerprint(&doc.snapshot().unwrap());
    let rendered = doc
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        (rendered.rendered.width, rendered.rendered.height),
        (16., 16.)
    );
    assert_eq!(before, fingerprint(&doc.snapshot().unwrap()));
    // A forged sparse length never authorizes allocation proportional to u32::MAX.
    let mut huge = engine.parse("packet", &Default::default()).unwrap();
    let mut huge_graph = huge.snapshot().unwrap();
    let rows = reference(huge_graph.field("rows").unwrap().unwrap());
    let Node::Array { length, .. } = &mut huge_graph.nodes[rows as usize] else {
        panic!()
    };
    *length = u32::MAX;
    huge.apply(0, huge_graph).unwrap();
    assert_eq!(
        huge.render(&Default::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert!(matches!(
        huge.snapshot().unwrap().nodes[rows as usize],
        Node::Array {
            length: u32::MAX,
            ..
        }
    ));
    // A used ill-formed label is explicitly rejected, never replacement-decoded.
    let mut bad = engine
        .parse("packet\n+8: \"A\"", &Default::default())
        .unwrap();
    let mut g = bad.snapshot().unwrap();
    let (_, _, b) = row_and_block(&g);
    let s = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    set(&mut g, b, "label", Atom::String(s));
    bad.apply(0, g).unwrap();
    assert!(bad
        .render(&Default::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
}
