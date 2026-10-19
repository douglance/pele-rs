//! Public Info ordinary-object edits against pinned Pele 1.6.0.
// Probe entry: node_modules/pele/dist/pele.js, SHA256
// d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
// Source: src/diagrams/info/model.ts at 458322a5b8abb1ae57ddf127c32017ab9fb14a92,
// SHA256 c0bd6b6255ffc6ba2a1e889c6c71fb9ca78f0a55409157407d554983ef373abc.
// Exact public JS probe: const m=parse('info'); Object.getPrototypeOf(m)===Object.prototype
// => true; Reflect.ownKeys(m) => ['type','title','accTitle','accDescr']; each descriptor
// has enumerable/writable/configurable=true. All metadata values are undefined.
use pele::compat::{
    diagrams::info::{Info, InfoModel},
    public::{Atom, JsString, ModelGraph, Node, Property, PublicDocument, PublicEngine},
    text::MetricsResources,
    *,
};
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
fn value(graph: &ModelGraph, atom: &Atom) -> Json {
    match atom {
        Atom::String(id) => json!(graph.strings[*id as usize].well_formed().unwrap()),
        Atom::Undefined => json!({"kind":"undefined"}),
        _ => panic!("string or undefined probe"),
    }
}
fn properties(graph: &ModelGraph) -> Json {
    Json::Array(graph.root_properties().unwrap().iter().map(|p|json!({"key":graph.strings[p.key as usize].well_formed().unwrap(),"value":value(graph,&p.value),"flags":p.flags})).collect())
}
fn set(graph: &mut ModelGraph, key: &str, value: Atom) {
    let Atom::Ref(root) = graph.root else {
        panic!("root object")
    };
    let Node::Object { properties } = &mut graph.nodes[root as usize] else {
        panic!("root object")
    };
    if let Some(field) = properties
        .iter_mut()
        .find(|p| graph.strings[p.key as usize].is(key))
    {
        field.value = value;
        return;
    }
    let id = graph.strings.len() as u32;
    graph.strings.push(key.into());
    properties.push(Property {
        key: id,
        value,
        flags: 7,
    });
}
fn string(graph: &mut ModelGraph, text: &str) -> Atom {
    let id = graph.strings.len() as u32;
    graph.strings.push(text.into());
    Atom::String(id)
}
fn set_text(graph: &mut ModelGraph, key: &str, text: &str) {
    let atom = string(graph, text);
    set(graph, key, atom)
}
fn flags(graph: &mut ModelGraph, key: &str, flags: u8) {
    let Atom::Ref(root) = graph.root else {
        panic!("root object")
    };
    let Node::Object { properties } = &mut graph.nodes[root as usize] else {
        panic!("root object")
    };
    properties
        .iter_mut()
        .find(|p| graph.strings[p.key as usize].is(key))
        .unwrap()
        .flags = flags;
}
struct FailingHost;
impl RenderResources for FailingHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("Info host stopped", ErrorCode::Semantic))
    }
}
#[test]
fn plain_info_fields_and_edits_match_pinned_order_descriptors_and_svg() {
    let mut registry = PublicEngine::empty();
    registry.register(Info);
    let empty = registry
        .parse("info", &ParseOptions::default())
        .unwrap()
        .snapshot()
        .unwrap();
    assert!(matches!(empty.nodes[0], Node::Object { .. }));
    assert_eq!(
        properties(&empty),
        json!([
            {"key":"type","value":"info","flags":7},
            {"key":"title","value":{"kind":"undefined"},"flags":7},
            {"key":"accTitle","value":{"kind":"undefined"},"flags":7},
            {"key":"accDescr","value":{"kind":"undefined"},"flags":7}
        ])
    );
    let mut document = registry
        .parse(
            "info title Before\naccTitle: Accessible\naccDescr: Details",
            &ParseOptions::default(),
        )
        .unwrap();
    let mut graph = document.snapshot().unwrap();
    set_text(&mut graph, "title", "Edited");
    set_text(&mut graph, "type", "changed-type");
    set_text(&mut graph, "accTitle", "New accessible");
    set(&mut graph, "accDescr", Atom::Undefined);
    flags(&mut graph, "title", 1);
    flags(&mut graph, "accTitle", 6);
    set_text(&mut graph, "applicationData", "extra");
    flags(&mut graph, "applicationData", 3);
    document.apply(0, graph).unwrap();
    // Exact JS operation: assign title='Edited', type='changed-type'; delete/reinsert
    // accDescr=undefined; defineProperty title flags1, accTitle='New accessible' flags6,
    // applicationData='extra' flags3. render(m) => type info, width97, height72, hash below.
    let expected = json!([
        {"key":"type","value":"changed-type","flags":7},
        {"key":"title","value":"Edited","flags":1},
        {"key":"accTitle","value":"New accessible","flags":6},
        {"key":"accDescr","value":{"kind":"undefined"},"flags":7},
        {"key":"applicationData","value":"extra","flags":3}
    ]);
    assert_eq!(properties(&document.snapshot().unwrap()), expected);
    let options = RenderOptions::default();
    let rendered = document.render(&options, &mut MetricsResources).unwrap();
    assert_eq!(rendered.diagram_type, DiagramType::Info.into());
    assert_eq!(
        (rendered.rendered.width, rendered.rendered.height),
        (97., 72.)
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(rendered.rendered.svg.as_bytes())),
        "68acc2027816805134bb753fe2b3ea1d68656bde77eaa6d4487450bfd479a8b8"
    );
    assert_eq!(
        document.render(&options, &mut MetricsResources).unwrap(),
        rendered
    );
    assert_eq!(properties(&document.snapshot().unwrap()), expected);
    registry.register(Info);
    drop(registry);
    assert_eq!(
        document.render(&options, &mut MetricsResources).unwrap(),
        rendered
    );
    // Pinned JS: delete m.accTitle; m.accTitle='New accessible' =>
    // ['type','title','accDescr','applicationData','accTitle'], accTitle flags7.
    let mut graph = document.snapshot().unwrap();
    let Atom::Ref(root) = graph.root else {
        panic!("root")
    };
    let Node::Object { properties } = &mut graph.nodes[root as usize] else {
        panic!("root")
    };
    properties.retain(|p| !graph.strings[p.key as usize].is("accTitle"));
    set_text(&mut graph, "accTitle", "New accessible");
    document.apply(1, graph).unwrap();
    let kept = document.snapshot().unwrap();
    let keys: Vec<_> = kept
        .root_properties()
        .unwrap()
        .iter()
        .map(|p| kept.strings[p.key as usize].well_formed().unwrap())
        .collect();
    assert_eq!(
        keys,
        ["type", "title", "accDescr", "applicationData", "accTitle"]
    );
    assert_eq!(kept.root_properties().unwrap().last().unwrap().flags, 7);
    assert_eq!(
        document.render(&options, &mut MetricsResources).unwrap(),
        rendered
    );
}
#[test]
fn info_metadata_edits_keep_native_ownership_absence_extras_and_recovery() {
    let engine = Engine::with_builtins();
    let options = RenderOptions::default();
    let parsed = engine
        .parse("---\ntitle: Retained\n---\ninfo", &options.parse)
        .unwrap();
    let mut document = PublicDocument::from_module(
        Info,
        parsed.model::<InfoModel>().unwrap(),
        parsed.config().clone(),
    )
    .unwrap();
    drop(parsed);
    drop(engine);
    let mut graph = document.snapshot().unwrap();
    set_text(&mut graph, "title", "Native title");
    set_text(&mut graph, "accTitle", "Native accessibility");
    set_text(&mut graph, "accDescr", "Native details");
    let id = graph.strings.len() as u32;
    graph.strings.push(JsString(vec![0xd800, 0x41, 0xdc00]));
    set(&mut graph, "applicationPayload", Atom::String(id));
    document.apply(0, graph).unwrap();
    let expected = Engine::with_builtins()
        .render_source(
            "info title Native title\naccTitle: Native accessibility\naccDescr: Native details",
            &options,
            &mut MetricsResources,
        )
        .unwrap();
    assert_eq!(
        document.render(&options, &mut MetricsResources).unwrap(),
        expected
    );
    let kept = document.snapshot().unwrap();
    assert_eq!(kept.strings[id as usize].0, vec![0xd800, 0x41, 0xdc00]);
    assert_eq!(
        document
            .render(&options, &mut FailingHost)
            .unwrap_err()
            .message,
        "Info host stopped"
    );
    assert_eq!(
        serde_json::to_value(document.snapshot().unwrap()).unwrap(),
        serde_json::to_value(&kept).unwrap()
    );
    assert_eq!(
        document.render(&options, &mut MetricsResources).unwrap(),
        expected
    );
    let mut native = options.clone();
    native.render.native_theme = Some(NativeTheme::default());
    let output = document.render(&native, &mut MetricsResources).unwrap();
    let xml = roxmltree::Document::parse(&output.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants()
            .find(|n| n.has_tag_name("title"))
            .unwrap()
            .text(),
        Some("Native accessibility")
    );
    assert_eq!(
        xml.descendants()
            .find(|n| n.has_tag_name("desc"))
            .unwrap()
            .text(),
        Some("Native details")
    );
    let mut raster = resvg::usvg::Options::default();
    raster
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec());
    raster.font_family = "Noto Sans".into();
    raster.fontdb_mut().set_sans_serif_family("Noto Sans");
    let tree = resvg::usvg::Tree::from_str(&output.rendered.svg, &raster).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(output.rendered.width as u32, output.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 100).count() > 100);
    native.render.output_limit = Some(1);
    assert_eq!(
        document
            .render(&native, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    native.render.output_limit = None;
    assert_eq!(
        document.render(&options, &mut MetricsResources).unwrap(),
        expected
    );
    let mut graph = document.snapshot().unwrap();
    let Atom::Ref(root) = graph.root else {
        panic!("root")
    };
    let Node::Object { properties } = &mut graph.nodes[root as usize] else {
        panic!("root")
    };
    properties.retain(|p| !graph.strings[p.key as usize].is("title"));
    set(&mut graph, "accTitle", Atom::Undefined);
    set(&mut graph, "accDescr", Atom::Null);
    document.apply(1, graph).unwrap();
    assert!(document
        .snapshot()
        .unwrap()
        .field("title")
        .unwrap()
        .is_none());
    let plain = Engine::with_builtins()
        .render_source("info", &options, &mut MetricsResources)
        .unwrap();
    assert_eq!(
        document.render(&options, &mut MetricsResources).unwrap(),
        plain
    );
}
#[test]
fn info_keeps_lone_utf16_values_but_reports_the_rendering_gap_and_recovers() {
    let mut engine = PublicEngine::empty();
    engine.register(Info);
    let mut document = engine.parse("info", &ParseOptions::default()).unwrap();
    let mut graph = document.snapshot().unwrap();
    let id = graph.strings.len() as u32;
    graph.strings.push(JsString(vec![0xd800]));
    set(&mut graph, "title", Atom::String(id));
    document.apply(0, graph).unwrap();
    let error = document
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err();
    assert_eq!(
        error.message,
        "UTF-16 Info text rendering is not implemented"
    );
    assert_eq!(
        document.snapshot().unwrap().strings[id as usize].0,
        vec![0xd800]
    );
    let mut graph = document.snapshot().unwrap();
    set(&mut graph, "title", Atom::Null);
    document.apply(1, graph).unwrap();
    assert_eq!(
        document
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Semantic
    );
    let mut graph = document.snapshot().unwrap();
    set_text(&mut graph, "title", "Recovered");
    document.apply(2, graph).unwrap();
    assert!(document
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap()
        .rendered
        .svg
        .contains("Recovered"));
}
#[test]
fn info_mutable_text_escaping_matches_exact_pinned_public_render_probes() {
    // Pinned JS, same artifact/source above: for each label, m=parse('info');
    // m.title=m.accTitle=m.accDescr=label; r=render(m); record dimensions/SHA256(svg).
    let probes = [
        (
            "",
            97.,
            40.,
            "f6b04549a630180e349818c994c6737f64aa163a0d03e7c8a9596f3ea3ed700e",
        ),
        (
            "A & B <script>alert(1)</script>",
            118.,
            72.,
            "eb8fee593d1ea1ace84375fddd7cdd424605a249805a6389b544127cc59f1ec7",
        ),
        (
            "literal #35; &#35; &amp;",
            125.,
            72.,
            "91fb8f3710567597e830f7c234c13d15d3973bb70b69b123e9cbc23e1756e25c",
        ),
        (
            "中文 🙂 é",
            97.,
            72.,
            "86b1623c3fcc651820a90178249fd8a3e5177eb9d84bf2b1d10955c891a4c56e",
        ),
        (
            "Line\\nTwo",
            97.,
            96.,
            "0f3ca6a0aa152b5d82de7345994c881bbc5e79153f3da051bf81d94cc9d95537",
        ),
        (
            "Line<br/>Two",
            97.,
            96.,
            "d2e16b3919c53aaddd0d2879a8145c488df5c6e24fc741cd2f31f6786e7b82a7",
        ),
        (
            "fa:fa-user Text",
            97.,
            72.,
            "07058e20a6a70b4389a86c1a4bec2cde10ddd0841b7e25e06efd413948af45b1",
        ),
        (
            "__proto__",
            97.,
            72.,
            "b05cf28cba745dd94b9afd2cbb2108c27bd25e4c67a26ed33527a75c2920fadf",
        ),
        (
            "Line\nTwo",
            97.,
            96.,
            "eb7521fab6619d80e2b2d99d5de177463d34a83f3ff2a34eda8cfc0f6b6a4a7c",
        ),
        (
            "\"quoted\" 'single'",
            143.,
            72.,
            "80f11d2816e8ae9f242e1c4bd2ce76ad77fb7fa49704e253fb8a8b83521b1050",
        ),
    ];
    let mut engine = PublicEngine::empty();
    engine.register(Info);
    for (label, width, height, sha256) in probes {
        let mut document = engine.parse("info", &ParseOptions::default()).unwrap();
        let mut graph = document.snapshot().unwrap();
        for key in ["title", "accTitle", "accDescr"] {
            set_text(&mut graph, key, label)
        }
        document.apply(0, graph).unwrap();
        let result = document
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap();
        assert_eq!(
            (result.rendered.width, result.rendered.height),
            (width, height),
            "{label:?}"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(result.rendered.svg.as_bytes())),
            sha256,
            "{label:?}"
        );
    }
}
