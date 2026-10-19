//! TreeView mutable native feature slices, Pele 1.6.0 / 458322a5 (MIT kepano).
//! model.ts SHA256 f9a3eb84483ab9c0e658be283f412944406a958bfb12b79f02b19c514a6cc30d.
//! Oracle node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! Public parse(), own descriptors, tree/DAG edits and render(model) probes.
//! Config probes parse(source,{config}); render(model) ignores parse options.
use pele::compat::{diagrams::tree_view::TreeView, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = "treeView-beta\nsrc/\n  main.rs\n  lib.rs :::highlight ## library\nREADME.md";
fn reference(a: &Atom) -> u32 {
    if let Atom::Ref(id) = a {
        *id
    } else {
        panic!("reference")
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
fn string(g: &mut ModelGraph, s: &str) -> Atom {
    let id = g.strings.len() as u32;
    g.strings.push(s.into());
    Atom::String(id)
}
fn set(g: &mut ModelGraph, id: u32, key: &str, value: Atom) {
    let i = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key));
    if let Some(i) = i {
        g.nodes[id as usize].properties_mut()[i].value = value
    } else {
        let key = if let Atom::String(id) = string(g, key) {
            id
        } else {
            unreachable!()
        };
        g.nodes[id as usize].properties_mut().push(Property {
            key,
            value,
            flags: 7,
        })
    }
}
fn keys(g: &ModelGraph, id: u32) -> Vec<String> {
    g.nodes[id as usize]
        .properties()
        .iter()
        .map(|p| g.strings[p.key as usize].well_formed().unwrap())
        .collect()
}
fn text(g: &mut ModelGraph, id: u32, key: &str, value: &str) {
    let a = string(g, value);
    set(g, id, key, a)
}

fn document() -> PublicDocument {
    let mut e = PublicEngine::empty();
    e.register(TreeView);
    e.parse(SOURCE, &ParseOptions::default()).unwrap()
}
fn child(g: &ModelGraph, id: u32, index: u32) -> u32 {
    let children = reference(field(g, id, "children"));
    reference(field(g, children, &index.to_string()))
}
fn virtual_root(g: &ModelGraph) -> u32 {
    reference(field(g, root(g), "root"))
}
fn append(g: &mut ModelGraph, id: u32, a: Atom) {
    let Node::Array { length, .. } = g.nodes[id as usize] else {
        panic!()
    };
    set(g, id, &length.to_string(), a);
    if let Node::Array { length, .. } = &mut g.nodes[id as usize] {
        *length += 1
    }
}
fn hash(d: &mut PublicDocument, width: Option<f64>, expected: &str) {
    let mut o = RenderOptions::default();
    o.render.max_width = width;
    let r = d.render(&o, &mut MetricsResources).unwrap();
    roxmltree::Document::parse(&r.rendered.svg).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(r.rendered.svg.as_bytes())),
        expected
    );
}
#[test]
fn tree_view_plain_wrapper_virtual_root_and_regular_node_descriptors() {
    let mut d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let virtual_root = virtual_root(&g);
    let src = child(&g, virtual_root, 0);
    let main = child(&g, src, 0);
    assert_eq!(d.model_schema().id, "pele/treeView@1");
    assert!(d.model_schema().methods.is_empty());
    assert!(d.model_schema().prototypes.is_empty());
    assert!(matches!(g.nodes[r as usize], Node::Object { .. }));
    assert_eq!(
        keys(&g, r),
        ["type", "title", "accTitle", "accDescr", "root", "count"]
    );
    assert_eq!(
        keys(&g, virtual_root),
        ["id", "level", "name", "nodeType", "children"]
    );
    assert_eq!(
        keys(&g, src),
        [
            "id",
            "level",
            "name",
            "nodeType",
            "icon",
            "cssClass",
            "description",
            "children"
        ]
    );
    assert_eq!(keys(&g, main), keys(&g, src));
    for k in ["icon", "cssClass", "description"] {
        assert!(matches!(field(&g, main, k), Atom::Undefined));
    }
    assert!(g
        .nodes
        .iter()
        .flat_map(|n| n.properties())
        .all(|p| p.flags == 7));
    assert!(matches!(field(&g,r,"count"),Atom::Number(n)if *n==5.));
    assert!(matches!(field(&g,virtual_root,"level"),Atom::Number(n)if *n== -1.));
    hash(
        &mut d,
        None,
        "aa9aa8871ec0c016dfb8ef9e21081072b8bf58c1556fcfd6379d2d5c3c993823",
    );
    hash(
        &mut d,
        Some(140.),
        "42c1fd4eb5e0f84c1d9c4a8aec8bf25f1cd9cc7610ac9366ca7f9f805f08b68a",
    );
}
fn edit(g: &mut ModelGraph, mode: &str) {
    let r = root(g);
    let virtual_root = virtual_root(g);
    let src = child(g, virtual_root, 0);
    let main = child(g, src, 0);
    let lib = child(g, src, 1);
    let readme = child(g, virtual_root, 1);
    match mode {
        "metadata" => {
            for (k, v) in [
                ("title", "Edited"),
                ("accTitle", "Files"),
                ("accDescr", "Directory"),
            ] {
                text(g, r, k, v)
            }
        }
        "records" => {
            text(g, src, "name", "source");
            text(g, main, "name", "changed.rs");
            text(g, lib, "description", "Updated");
            text(g, readme, "cssClass", "highlight extra");
        }
        "alias" => {
            let children = reference(field(g, virtual_root, "children"));
            append(g, children, Atom::Ref(src));
            let children = reference(field(g, src, "children"));
            append(g, children, Atom::Ref(readme));
        }
        "reverse" => {
            for id in [virtual_root, src] {
                let children = reference(field(g, id, "children"));
                let a = field(g, children, "0").clone();
                let b = field(g, children, "1").clone();
                set(g, children, "0", b);
                set(g, children, "1", a)
            }
        }
        "icons" => {
            text(g, src, "icon", "none");
            text(g, main, "icon", "custom:leaf");
            text(g, readme, "icon", "file");
        }
        "ignored" => {
            set(g, r, "count", Atom::Null);
            set(g, virtual_root, "name", Atom::Null);
            set(g, virtual_root, "nodeType", Atom::Bool(true));
            set(g, virtual_root, "id", Atom::Number(f64::NAN));
            set(g, virtual_root, "level", Atom::Undefined);
            set(g, virtual_root, "icon", Atom::Bool(true));
            set(g, virtual_root, "cssClass", Atom::Null);
            set(g, virtual_root, "description", Atom::Bool(true));
            for id in [src, main, lib, readme] {
                set(g, id, "id", Atom::Null);
                set(g, id, "level", Atom::Number(f64::NAN));
            }
        }
        "unknown" => {
            text(g, main, "nodeType", "future");
            set(g, lib, "nodeType", Atom::Null);
        }
        "falsy" => {
            for id in [src, main, lib] {
                set(g, id, "icon", Atom::Null);
                set(g, id, "description", Atom::Bool(false));
            }
        }
        "rootAlias" => {
            let children = reference(field(g, src, "children"));
            append(g, children, Atom::Ref(readme));
            text(g, readme, "name", "shared.md");
        }
        _ => unreachable!(),
    }
}
#[test]
fn tree_view_pinned_edits_alias_guides_annotations_and_ignored_bookkeeping() {
    for (mode, wide, narrow) in [
        (
            "metadata",
            "664b915760f44a9d0df3ca8c2abe550b2e7f6a51fbb712692725d31e9f368f56",
            "f56741b725e834e215ca78521626b7be826b9e60cd0cfd2f85499f39177b36c1",
        ),
        (
            "records",
            "f2a88df0dd465ff207886830cf8fbf672d93d2aeb1fa0523bf23bd08fa26ffc5",
            "d42bfcfbb633dacef302155572ff5136f484e73faf7b7cc4214d6c6959e64d75",
        ),
        (
            "alias",
            "7fb7c8c3f6bdc75ecda05fcb8474b03fc9b33683fd9abb397364d7dcbccd53fd",
            "f0a3bbf00ef652022b2392ced833f33cf5944e89d5224b67765e846e9845ebe3",
        ),
        (
            "reverse",
            "de33a06e783377ee61326261e4385955471a5696291fc26856476ffa32a1dc7f",
            "a5c657f80f690f83fd3dde94d25979bec26216fd58144e097ef055f7d39e56ec",
        ),
        (
            "icons",
            "9b95cd6800741507f6eb0075ab4a76ec6009f9f9bf5da366331edad1edc00694",
            "1886a7895c758708d08f50f025ec411cdcb11c3e67641e10ae21ab3493ec537c",
        ),
        (
            "ignored",
            "aa9aa8871ec0c016dfb8ef9e21081072b8bf58c1556fcfd6379d2d5c3c993823",
            "42c1fd4eb5e0f84c1d9c4a8aec8bf25f1cd9cc7610ac9366ca7f9f805f08b68a",
        ),
        (
            "unknown",
            "aa9aa8871ec0c016dfb8ef9e21081072b8bf58c1556fcfd6379d2d5c3c993823",
            "42c1fd4eb5e0f84c1d9c4a8aec8bf25f1cd9cc7610ac9366ca7f9f805f08b68a",
        ),
        (
            "falsy",
            "9e6ff50ea243d4f8bd21762801c9e9cd687d32aba2544820da2fe79336a66198",
            "9e6ff50ea243d4f8bd21762801c9e9cd687d32aba2544820da2fe79336a66198",
        ),
        (
            "rootAlias",
            "5c5b89716a189b1d0c0d305c8acfca8b611f04f817eb0b1b83592f24c2fc18bf",
            "532bbd448e7b5f40c3c6ee4537d6086f5feb3a6eb802d10e3e6303c16940b7a8",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        edit(&mut g, mode);
        d.apply(0, g).unwrap();
        let saved = serde_json::to_value(d.snapshot().unwrap()).unwrap();
        hash(&mut d, None, wide);
        hash(&mut d, Some(140.), narrow);
        assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
    }
}
fn icon_parse_options() -> ParseOptions {
    let mut options = ParseOptions::default();
    options.preparation.config=serde_json::from_value(serde_json::json!({"treeView":{"showIcons":true,"extensionIcons":{".rs":"rust"},"defaultIconPack":"custom"}})).unwrap();
    options
}
struct Icons {
    calls: Vec<String>,
    fail: bool,
}
impl RenderResources for Icons {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        MetricsResources.text_width(r)
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        self.calls.push(name.into());
        if self.fail {
            Err(PeleError::new("TreeView icon stopped", ErrorCode::Semantic))
        } else {
            Ok(None)
        }
    }
}
#[test]
fn tree_view_retained_icon_config_unknown_types_and_repeated_direct_host_calls_match_pinned() {
    for (mode, wide, narrow, calls) in [
        (
            "baseline",
            "dcd58f080657d1c64028e10630d828506905cc750732af4f9b1e883cdd43dc72",
            "56d7f65a3653482446c6a250cf58f1aeface7c02768b93a76d9b8f4b4f287037",
            vec![
                "mermaid-treeview:folder",
                "custom:rust",
                "custom:rust",
                "mermaid-treeview:file",
            ],
        ),
        (
            "unknown",
            "5074fe64671da5c28d63404054ab954e8d959921591a233bdc7e9fbdf7a61e84",
            "4a38d908697c154608102f2ba8bf7c78d95042f25e6eddc4232e752c2fb98d77",
            vec![
                "mermaid-treeview:folder",
                "mermaid-treeview:file",
                "mermaid-treeview:file",
                "mermaid-treeview:file",
            ],
        ),
        (
            "icons",
            "429647352ad5b8af546ab9246849a7e2349f76580f7c8207fc96011a83f2af5f",
            "ff52876c12fbadd19be2d4c47f59c6f4250ac66717f05ee60213cfc60d5b6023",
            vec!["custom:leaf", "custom:rust", "mermaid-treeview:file"],
        ),
        (
            "alias",
            "073ef01454894b91d3b28900700bea5b0695a922bba69e6897df8448b202cce1",
            "2fa8e697bf0a96a981ddbcecf3a71003353da77f564592e85f3a0b4dd638320b",
            vec![
                "mermaid-treeview:folder",
                "custom:rust",
                "custom:rust",
                "mermaid-treeview:file",
                "mermaid-treeview:file",
                "mermaid-treeview:folder",
                "custom:rust",
                "custom:rust",
                "mermaid-treeview:file",
            ],
        ),
    ] {
        let mut e = PublicEngine::empty();
        e.register(TreeView);
        let mut d = e.parse(SOURCE, &icon_parse_options()).unwrap();
        if mode != "baseline" {
            let mut g = d.snapshot().unwrap();
            edit(&mut g, mode);
            d.apply(0, g).unwrap();
        }
        for (width, expected, multiplier) in [(None, wide, 1), (Some(140.), narrow, 2)] {
            let mut host = Icons {
                calls: vec![],
                fail: false,
            };
            let mut o = RenderOptions::default();
            o.render.max_width = width;
            o.parse.preparation.limit = 0;
            o.parse.preparation.config = Object::default();
            let r = d.render(&o, &mut host).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(r.rendered.svg.as_bytes())),
                expected
            );
            assert_eq!(host.calls, calls.repeat(multiplier));
        }
    }
}
// Historical JSON projection is only a syntax/render regression check; identity,
// functions, undefined, flags and raw UTF-16 are proved separately above.
fn historical_json(g: &ModelGraph, a: &Atom) -> Option<Json> {
    match a {
        Atom::Undefined | Atom::BuiltinFunction(_) => None,
        Atom::Null => Some(Json::Null),
        Atom::Bool(v) => Some(Json::Bool(*v)),
        Atom::Number(v) => Some(if v.is_finite() {
            // JSON.stringify numbers have JavaScript spelling: 4 rather than 4.0.
            serde_json::from_str(ryu_js::Buffer::new().format(*v)).unwrap()
        } else {
            Json::Null
        }),
        Atom::String(id) => Some(Json::String(g.strings[*id as usize].well_formed().unwrap())),
        Atom::Ref(id) => match &g.nodes[*id as usize] {
            Node::Object { properties } | Node::Instance { properties, .. } => Some(Json::Object(
                properties
                    .iter()
                    .filter(|p| p.flags & 1 != 0)
                    .filter_map(|p| {
                        historical_json(g, &p.value)
                            .map(|v| (g.strings[p.key as usize].well_formed().unwrap(), v))
                    })
                    .collect(),
            )),
            Node::Array { length, .. } => Some(Json::Array(
                (0..*length)
                    .map(|i| {
                        historical_json(g, field(g, *id, &i.to_string())).unwrap_or(Json::Null)
                    })
                    .collect(),
            )),
            Node::Map { entries, .. } => Some(Json::Array(
                entries
                    .iter()
                    .map(|(k, v)| {
                        Json::Array(vec![
                            historical_json(g, k).unwrap_or(Json::Null),
                            historical_json(g, v).unwrap_or(Json::Null),
                        ])
                    })
                    .collect(),
            )),
            _ => panic!("TreeView baseline uses only records, maps and arrays"),
        },
        _ => panic!("TreeView baseline has no host references"),
    }
}
struct FixtureResources {
    host: Json,
}
impl RenderResources for FixtureResources {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        if self.host["measure"] == "fixed" {
            Ok(r.text.encode_utf16().count() as f64
                * r.size
                * if r.style & 1 != 0 { 0.71 } else { 0.6 })
        } else {
            MetricsResources.text_width(r)
        }
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        Ok(self.host["icons"][name]
            .as_str()
            .map(|s| TrustedSvg::from_trusted_markup(s.into())))
    }
}
#[test]
fn mutable_treeview_preserves_entire_existing_pinned_render_and_error_corpus() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/treeview.json"
    ));
    let bytes = if path.exists() {
        std::fs::read(path).unwrap()
    } else {
        use std::io::Read;
        let mut bytes = vec![];
        flate2::read::GzDecoder::new(std::fs::File::open(path.with_extension("json.gz")).unwrap())
            .read_to_end(&mut bytes)
            .unwrap();
        bytes
    };
    let fixture: Json = serde_json::from_slice(&bytes).unwrap();
    let mut e = PublicEngine::empty();
    e.register(TreeView);

    for c in fixture["cases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(c["options"].clone()).unwrap();
        let actual = e
            .parse(c["source"].as_str().unwrap(), &options.parse)
            .and_then(|mut d| {
                if c["expected"].get("model").is_some() {
                    let g = d.snapshot().unwrap();
                    assert_eq!(
                        historical_json(&g, &g.root).unwrap(),
                        c["expected"]["model"],
                        "{}",
                        c["id"]
                    );
                }
                d.render(
                    &options,
                    &mut FixtureResources {
                        host: c["host"].clone(),
                    },
                )
            });
        match actual {
            Ok(result) => {
                let mut result = serde_json::to_value(result).unwrap();
                if c["expected"]["result"].get("svgSha256").is_some() {
                    result["svgSha256"] = Json::String(format!(
                        "{:x}",
                        Sha256::digest(result["svg"].as_str().unwrap().as_bytes())
                    ));
                    result.as_object_mut().unwrap().remove("svg");
                }
                assert_eq!(result, c["expected"]["result"], "{}", c["id"]);
            }
            Err(error) => assert_eq!(
                serde_json::to_value(error).unwrap(),
                c["expected"]["error"],
                "{}",
                c["id"]
            ),
        }
    }
}

struct FailingText;
impl RenderResources for FailingText {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("TreeView text stopped", ErrorCode::Semantic))
    }
}
#[test]
fn tree_view_native_edits_lifetime_raw_graph_and_text_icon_output_recovery() {
    let mut e = PublicEngine::empty();
    e.register(TreeView);
    let parse = icon_parse_options();
    let mut d = e.parse(SOURCE, &parse).unwrap();
    e.register(TreeView);
    drop(e);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let virtual_root = virtual_root(&g);
    let src = child(&g, virtual_root, 0);
    let main = child(&g, src, 0);
    let lib = child(&g, src, 1);
    text(&mut g, main, "name", "changed.rs");
    text(&mut g, lib, "description", "Changed description");
    d.apply(0, g).unwrap();
    let mut options = RenderOptions::default();
    options.parse = parse;
    options.render.native_theme = Some(NativeTheme::default());
    let expected = Engine::with_builtins()
        .render_source(
            &SOURCE
                .replace("main.rs", "changed.rs")
                .replace("## library", "## Changed description"),
            &options,
            &mut MetricsResources,
        )
        .unwrap();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), expected);
    let mut g = d.snapshot().unwrap();
    text(&mut g, r, "accTitle", "Native files");
    text(&mut g, r, "accDescr", "Edited directory");
    text(&mut g, r, "type", "pie");
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    set(&mut g, r, "applicationRaw", Atom::String(raw));
    set(&mut g, virtual_root, "name", Atom::String(raw));
    set(&mut g, r, "count", Atom::Number(f64::NAN));
    for (k, n) in [
        ("nan", f64::from_bits(0x7ff8000000000042)),
        ("negativeZero", -0.),
        ("infinity", f64::INFINITY),
    ] {
        set(&mut g, r, k, Atom::Number(n));
    }
    let mut previous = r;
    for _ in 0..5000 {
        let id = g.nodes.len() as u32;
        g.nodes.push(Node::Object { properties: vec![] });
        set(&mut g, previous, "next", Atom::Ref(id));
        previous = id;
    }
    set(&mut g, previous, "next", Atom::Ref(r));
    g.nodes[r as usize].properties_mut()[0].flags = 0;
    let bytes = serde_json::to_vec(&g).unwrap();
    d.apply(d.revision(), serde_json::from_slice(&bytes).unwrap())
        .unwrap();
    let saved = serde_json::to_value(d.snapshot().unwrap()).unwrap();
    let native = d.render(&options, &mut MetricsResources).unwrap();
    assert_eq!(native.diagram_type.as_str(), "treeView");
    let xml = roxmltree::Document::parse(&native.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Native files")));
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("desc") && n.text() == Some("Edited directory")));
    let mut svg_options = resvg::usvg::Options::default();
    svg_options
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec());
    svg_options.fontdb_mut().set_sans_serif_family("Noto Sans");
    svg_options.font_family = "Noto Sans".into();
    let tree = resvg::usvg::Tree::from_str(&native.rendered.svg, &svg_options).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(native.rendered.width as u32, native.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 200).count() > 500);
    assert_eq!(
        d.render(&options, &mut FailingText).unwrap_err().message,
        "TreeView text stopped"
    );
    let mut host = Icons {
        calls: vec![],
        fail: true,
    };
    assert_eq!(
        d.render(&options, &mut host).unwrap_err().message,
        "TreeView icon stopped"
    );
    assert_eq!(host.calls, ["mermaid-treeview:folder"]);
    options.render.output_limit = Some(1);
    assert_eq!(
        d.render(&options, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    options.render.output_limit = None;
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), native);
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
    let g = d.snapshot().unwrap();
    assert_eq!(g.strings[raw as usize].0, [0xd800, 65, 0xdc00]);
    assert!(matches!(field(&g,r,"nan"),Atom::Number(n)if n.to_bits()==0x7ff8000000000042));
    assert!(
        matches!(field(&g,r,"negativeZero"),Atom::Number(n)if n.to_bits()==(-0.0f64).to_bits())
    );
    assert!(d.apply(d.revision() - 1, g).is_err());
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
}
#[test]
fn tree_view_flat_deep_dag_cycle_sparse_and_expansion_limits_retain_graph() {
    let mut source = "treeView-beta\n".to_string();
    for i in 0..1000 {
        source.push_str(&" ".repeat(i));
        source.push_str(&format!("node{i}/\n"));
    }
    let mut e = PublicEngine::empty();
    e.register(TreeView);
    let mut parse = ParseOptions::default();
    parse.preparation.limit = 1_000_000;
    let mut d = e.parse(&source, &parse).unwrap();
    let g = d.snapshot().unwrap();
    let bytes = serde_json::to_vec(&g).unwrap();
    d.apply(0, serde_json::from_slice(&bytes).unwrap()).unwrap();
    let mut options = RenderOptions::default();
    options.parse = parse;
    assert_eq!(
        d.render(&options, &mut MetricsResources).unwrap(),
        Engine::with_builtins()
            .render_source(&source, &options, &mut MetricsResources)
            .unwrap()
    );
    let original = document().snapshot().unwrap();
    d.apply(d.revision(), original.clone()).unwrap();
    let root = virtual_root(&original);
    let children = reference(field(&original, root, "children"));
    let src = child(&original, root, 0);
    let main = child(&original, src, 0);
    for (mode, message) in [
        ("cycle", "cyclic"),
        ("expanded", "work budget"),
        ("sparse", "sparse"),
        ("arrayWork", "work budget"),
        ("childrenSet", "must be Arrays"),
        ("loneText", "lone UTF-16"),
        ("cssNull", "text coercion"),
        ("truthyIcon", "text coercion"),
    ] {
        let mut g = original.clone();
        match mode {
            "cycle" => set(&mut g, children, "0", Atom::Ref(root)),
            "expanded" => {
                let mut current = main;
                for _ in 0..25 {
                    let children = g.nodes.len() as u32;
                    g.nodes.push(Node::Array {
                        length: 2,
                        properties: vec![],
                    });
                    set(&mut g, children, "0", Atom::Ref(current));
                    set(&mut g, children, "1", Atom::Ref(current));
                    let node = g.nodes.len() as u32;
                    g.nodes.push(Node::Object { properties: vec![] });
                    text(&mut g, node, "name", "Repeated");
                    text(&mut g, node, "nodeType", "file");
                    set(&mut g, node, "children", Atom::Ref(children));
                    current = node;
                }
                set(&mut g, children, "0", Atom::Ref(current));
            }
            "sparse" => {
                g.nodes[children as usize].properties_mut().remove(0);
            }
            "arrayWork" => {
                g.nodes[children as usize] = Node::Array {
                    length: 1_000_001,
                    properties: vec![],
                }
            }
            "childrenSet" => {
                g.nodes[children as usize] = Node::Set {
                    values: vec![Atom::Ref(src)],
                    properties: vec![],
                }
            }
            "loneText" => {
                let raw = g.strings.len() as u32;
                g.strings.push(JsString(vec![0xd800]));
                set(&mut g, main, "name", Atom::String(raw));
            }
            "cssNull" => set(&mut g, main, "cssClass", Atom::Null),
            "truthyIcon" => set(&mut g, main, "icon", Atom::Bool(true)),
            _ => unreachable!(),
        };
        let saved = serde_json::to_value(&g).unwrap();
        d.apply(d.revision(), g).unwrap();
        let error = d
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap_err();
        assert!(error.message.contains(message), "{mode}: {}", error.message);
        assert_eq!(
            error.code,
            if mode == "expanded" || mode == "arrayWork" {
                ErrorCode::Limit
            } else {
                ErrorCode::Semantic
            }
        );
        assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
        d.apply(d.revision(), original.clone()).unwrap();
        hash(
            &mut d,
            None,
            "aa9aa8871ec0c016dfb8ef9e21081072b8bf58c1556fcfd6379d2d5c3c993823",
        );
    }
}

#[test]
fn tree_view_typed_factory_rejects_invalid_references_and_retains_valid_models() {
    let engine = Engine::with_builtins();
    let parsed = engine.parse(SOURCE, &ParseOptions::default()).unwrap();
    let model = parsed
        .model::<pele::compat::diagrams::tree_view::TreeViewModel>()
        .unwrap();
    let mut d = PublicDocument::from_module(TreeView, model, Object::default()).unwrap();
    let mut invalid = model.clone();
    invalid.root = usize::MAX;
    assert!(PublicDocument::from_module(TreeView, &invalid, Object::default()).is_err());
    let mut invalid = model.clone();
    invalid.nodes[invalid.root].children.push(usize::MAX);
    assert!(PublicDocument::from_module(TreeView, &invalid, Object::default()).is_err());
    drop(parsed);
    drop(engine);
    hash(
        &mut d,
        None,
        "aa9aa8871ec0c016dfb8ef9e21081072b8bf58c1556fcfd6379d2d5c3c993823",
    );
}
