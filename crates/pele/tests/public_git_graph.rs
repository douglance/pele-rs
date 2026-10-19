//! Public mutable GitGraph, pinned Pele 1.6.0 / 458322a5 (MIT kepano).
//! db.ts SHA256 5f031f66aa3b36ab47ab2747d69392db615cc3550702544a6b70977a579e452c.
//! node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! Public parse/own-descriptor and render(model) mutation probes, default measurer.
//! No actual GitGraph prototype/builder methods are returned by the JS library.
use pele::compat::{diagrams::git_graph::GitGraph, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE:&str="gitGraph\ncommit id:\"a\" msg:\"Alpha\" tag:\"v1\"\nbranch feature\ncommit id:\"b\" type:HIGHLIGHT\ncheckout main\ncommit id:\"c\"\nmerge feature id:\"m\"\nbranch fix\ncommit id:\"f\"\ncheckout main\ncherry-pick id:\"f\"";
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
fn entries(g: &ModelGraph, id: u32) -> &Vec<(Atom, Atom)> {
    if let Node::Map { entries, .. } = &g.nodes[id as usize] {
        entries
    } else {
        panic!("map")
    }
}
fn map<'a>(g: &'a ModelGraph, id: u32, key: &str) -> &'a Atom {
    &entries(g, id)
        .iter()
        .find(|(k, _)| g.text(k).is_ok_and(|s| s.is(key)))
        .unwrap()
        .1
}

fn document() -> PublicDocument {
    let mut e = PublicEngine::empty();
    e.register(GitGraph);
    e.parse(SOURCE, &ParseOptions::default()).unwrap()
}
fn commit(g: &ModelGraph, key: &str) -> u32 {
    reference(map(g, reference(field(g, root(g), "commits")), key))
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
#[test]
fn git_graph_actual_plain_objects_property_order_undefined_and_head_identity() {
    let mut d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    assert_eq!(d.model_schema().id, "pele/gitGraph@1");
    assert!(d.model_schema().methods.is_empty());
    assert!(d.model_schema().prototypes.is_empty());
    assert!(matches!(g.nodes[r as usize], Node::Object { .. }));
    assert_eq!(
        keys(&g, r),
        [
            "type",
            "title",
            "accTitle",
            "accDescr",
            "direction",
            "commits",
            "branches",
            "lanes",
            "currentBranch",
            "head",
            "warnings"
        ]
    );
    for k in ["title", "accTitle", "accDescr"] {
        assert!(matches!(field(&g, r, k), Atom::Undefined));
    }
    assert_eq!(
        keys(&g, commit(&g, "a")),
        ["id", "message", "seq", "type", "tags", "parents", "branch"]
    );
    assert_eq!(
        keys(&g, commit(&g, "m")),
        [
            "id",
            "message",
            "seq",
            "parents",
            "branch",
            "type",
            "customType",
            "customId",
            "tags"
        ]
    );
    assert!(matches!(
        field(&g, commit(&g, "m"), "customType"),
        Atom::Undefined
    ));
    let head = reference(field(&g, r, "head"));
    assert_eq!(
        keys(&g, head),
        ["id", "message", "seq", "parents", "branch", "type", "tags"]
    );
    assert_eq!(
        head,
        reference(
            &entries(&g, reference(field(&g, r, "commits")))
                .last()
                .unwrap()
                .1
        )
    );
    assert!(g
        .nodes
        .iter()
        .flat_map(|n| n.properties())
        .all(|p| p.flags == 7));
    hash(
        &mut d,
        None,
        "29f973e9385987e3aa52e2f7f4f77f333f26f1ca161ea1c44dcc42520ddf4a60",
    );
    hash(
        &mut d,
        Some(180.),
        "4b683f6b1b9bd673c8d95112a3369582fcec756ce7f6b9c216f1013914c17523",
    );
    // Duplicate ID replaces Map value in its original slot; checkout head aliases it.
    let mut e = PublicEngine::empty();
    e.register(GitGraph);
    let d=e.parse("gitGraph\ncommit id:\"a\"\ncommit id:\"b\"\ncommit id:\"a\"\nbranch other\ncheckout main",&ParseOptions::default()).unwrap();
    let g = d.snapshot().unwrap();
    assert_eq!(reference(field(&g, root(&g), "head")), commit(&g, "a"));
    assert_eq!(
        entries(&g, reference(field(&g, root(&g), "commits"))).len(),
        2
    );
    let w = reference(field(&g, root(&g), "warnings"));
    assert!(g
        .text(field(&g, w, "0"))
        .unwrap()
        .is("Commit ID a already exists"));
}
#[test]
fn git_graph_pinned_mutations_reuse_real_renderer_and_retain_graph_aliases() {
    for (mode, wide, narrow) in [
        (
            "records",
            "1dc59363115f4e45e4bfd2025667dd0ca111960fa6b9304f108ded4cd013bd8f",
            "99cf0005a6099ace3bed2f573e6146f249f017609f56c6071d08505c34f57bbd",
        ),
        (
            "alias",
            "cc54e8020eabece76909ef7933e4f00fb11a49c5833ef137205b1b93613f05db",
            "37cb53f7d01b221d7a217ac8926b8e818bc1054edc2a7d550768d6df509f26ba",
        ),
        (
            "reorder",
            "804ff2a8a391c29d4d6d3cbd22908a7cdb0f3a456eb41fa579ffcd30d52ed4f6",
            "1d1bc812247d12faf24e6b275584a9eb7da3bb0ac7aa47ff64af1ac10ccd860d",
        ),
        (
            "opaque",
            "29f973e9385987e3aa52e2f7f4f77f333f26f1ca161ea1c44dcc42520ddf4a60",
            "4b683f6b1b9bd673c8d95112a3369582fcec756ce7f6b9c216f1013914c17523",
        ),
        (
            "types",
            "01692fbe103668f9e1553b0acd775953f3ab29a35b10490d16fff0e45aae60fc",
            "f6f801db42a05e9e5aa2709c1a41778adcaa4158986945f5520974ab4ed3f97e",
        ),
        (
            "parents",
            "15465650bc88f58b05d0abf6f55ce32c9b80b0b83aff304decda0d2229064add",
            "5701f4fe42ac8559d146a69568cbd948da2c96a297188dfe47cb18b16afc68d9",
        ),
        (
            "setTags",
            "658905d13cc3fb2a434c5e6f0f090f9190272306780438acde14e03d1956c751",
            "c5edf41d11613896a0049a2f17c012d9cf2e803c5261af3d5a13169d5282411c",
        ),
        (
            "direction",
            "4b683f6b1b9bd673c8d95112a3369582fcec756ce7f6b9c216f1013914c17523",
            "4b683f6b1b9bd673c8d95112a3369582fcec756ce7f6b9c216f1013914c17523",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let a = commit(&g, "a");
        let b = commit(&g, "b");
        let c = commit(&g, "c");
        let m = commit(&g, "m");
        match mode {
            "records" => {
                for (k, v) in [
                    ("title", "Edited history"),
                    ("accTitle", "Accessible"),
                    ("accDescr", "Details"),
                ] {
                    text(&mut g, r, k, v)
                }
                text(&mut g, a, "message", "<b>Edited</b>");
                let tags = reference(field(&g, a, "tags"));
                text(&mut g, tags, "1", "v2");
                if let Node::Array { length, .. } = &mut g.nodes[tags as usize] {
                    *length = 2;
                }
                set(&mut g, b, "type", Atom::Number(1.));
            }
            "alias" => {
                let commits = reference(field(&g, r, "commits"));
                let key = string(&mut g, "alias");
                if let Node::Map { entries, .. } = &mut g.nodes[commits as usize] {
                    entries.push((key, Atom::Ref(a)))
                }
                let h = reference(field(&g, r, "head"));
                text(&mut g, h, "message", "Changed head");
            }
            "reorder" => {
                set(&mut g, a, "seq", Atom::Number(99.5));
                set(&mut g, c, "seq", Atom::Number(-2.5));
                let lanes = reference(field(&g, r, "lanes"));
                let vs = (0..3)
                    .rev()
                    .map(|i| field(&g, lanes, &i.to_string()).clone())
                    .collect::<Vec<_>>();
                for (i, v) in vs.into_iter().enumerate() {
                    set(&mut g, lanes, &i.to_string(), v)
                }
            }
            "opaque" => {
                let key = g.nodes.len() as u32;
                g.nodes.push(Node::Object { properties: vec![] });
                let commits = reference(field(&g, r, "commits"));
                if let Node::Map { entries, .. } = &mut g.nodes[commits as usize] {
                    let (_, value) = entries.remove(0);
                    entries.push((Atom::Ref(key), value));
                }
                for k in ["head", "currentBranch", "branches", "warnings"] {
                    set(&mut g, r, k, Atom::Null)
                }
                text(&mut g, r, "type", "pie");
            }
            "types" => {
                set(&mut g, a, "type", Atom::Number(-3.));
                set(&mut g, b, "type", Atom::Number(9.));
                set(&mut g, m, "customType", Atom::Number(2.));
                set(&mut g, m, "customId", Atom::Bool(false));
            }
            "parents" => {
                let vs = ["f", "unknown", "b"]
                    .into_iter()
                    .map(|v| string(&mut g, v))
                    .collect();
                let parents = array(&mut g, vs);
                set(&mut g, b, "parents", Atom::Ref(parents));
                let parents = reference(field(&g, m, "parents"));
                text(&mut g, parents, "2", "a");
                if let Node::Array { length, .. } = &mut g.nodes[parents as usize] {
                    *length = 3;
                }
            }
            "setTags" => {
                let values = ["one", "two"]
                    .into_iter()
                    .map(|v| string(&mut g, v))
                    .collect();
                let tags = g.nodes.len() as u32;
                g.nodes.push(Node::Set {
                    values,
                    properties: vec![],
                });
                set(&mut g, a, "tags", Atom::Ref(tags));
            }
            "direction" => text(&mut g, r, "direction", "weird"),
            _ => unreachable!(),
        };
        d.apply(0, g).unwrap();
        let saved = serde_json::to_value(d.snapshot().unwrap()).unwrap();
        hash(&mut d, None, wide);
        hash(&mut d, Some(180.), narrow);
        assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
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
            _ => panic!("GitGraph baseline uses only records, maps and arrays"),
        },
        _ => panic!("GitGraph baseline has no host references"),
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
fn mutable_git_graph_preserves_entire_existing_pinned_render_and_error_corpus() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/git-graph.json"
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
    e.register(GitGraph);
    e.register(pele::compat::diagrams::flowchart::Flowchart);
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

#[test]
fn git_graph_default_iterators_numeric_sorting_and_inactive_state_match_pinned() {
    for (mode, expected) in [
        (
            "tagsString",
            "435cb4274861cdd3ba88cac4d6ae359f4744d174702719b82682cb91e6409ef9",
        ),
        (
            "numericTypes",
            "65df23348b896d33f4cb81b792291a0f082219528b30a00ba924612c5de55417",
        ),
        (
            "seqInfinite",
            "07184419567c4f90f1038d5614c4900dc0c12b9a419647a63c60d50256750176",
        ),
        (
            "unused",
            "29f973e9385987e3aa52e2f7f4f77f333f26f1ca161ea1c44dcc42520ddf4a60",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let a = commit(&g, "a");
        let b = commit(&g, "b");
        let m = commit(&g, "m");
        match mode {
            "tagsString" => text(&mut g, a, "tags", "a🙂"),
            "numericTypes" => {
                set(&mut g, a, "type", Atom::Number(2.5));
                set(&mut g, b, "type", Atom::Null);
                set(&mut g, m, "customType", Atom::Number(f64::NAN));
            }
            "seqInfinite" => {
                set(&mut g, a, "seq", Atom::Number(f64::INFINITY));
                set(&mut g, b, "seq", Atom::Number(f64::NEG_INFINITY));
            }
            "unused" => {
                for k in ["head", "branches", "currentBranch", "warnings"] {
                    set(&mut g, r, k, Atom::Null)
                }
                set(&mut g, r, "accTitle", Atom::Bool(false));
                set(&mut g, r, "accDescr", Atom::Number(f64::NAN));
            }
            _ => unreachable!(),
        }
        d.apply(0, g).unwrap();
        hash(&mut d, None, expected);
    }
}
struct FailingHost;
impl RenderResources for FailingHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("GitGraph host stopped", ErrorCode::Semantic))
    }
}
#[test]
fn git_graph_native_edit_lifetime_flat_utf16_ieee_flags_and_resource_recovery() {
    let mut e = PublicEngine::empty();
    e.register(GitGraph);
    let immutable = Engine::with_builtins()
        .parse(SOURCE, &ParseOptions::default())
        .unwrap();
    let mut d = PublicDocument::from_module(
        GitGraph,
        immutable
            .model::<pele::compat::diagrams::git_graph::GitModel>()
            .unwrap(),
        Object::default(),
    )
    .unwrap();
    e.register(GitGraph);
    drop(e);
    drop(immutable);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let a = commit(&g, "a");
    text(&mut g, r, "title", "Native edited history");
    text(&mut g, a, "message", "<script>never active</script>");
    let tags = reference(field(&g, a, "tags"));
    text(&mut g, tags, "0", "Edited tag");
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    set(&mut g, r, "applicationUTF16", Atom::String(raw));
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
    let commits = reference(field(&g, r, "commits"));
    if let Node::Map { entries, .. } = &mut g.nodes[commits as usize] {
        entries[0].0 = Atom::String(raw)
    }
    g.nodes[r as usize].properties_mut()[0].flags = 0;
    let bytes = serde_json::to_vec(&g).unwrap();
    d.apply(0, serde_json::from_slice(&bytes).unwrap()).unwrap();
    let saved = serde_json::to_value(d.snapshot().unwrap()).unwrap();
    let mut options = RenderOptions::default();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    options.render.native_theme = Some(NativeTheme::default());
    let expected = d.render(&options, &mut MetricsResources).unwrap();
    assert!(expected.rendered.svg.contains("Native edited history"));
    assert!(expected.rendered.svg.contains("Edited tag"));
    assert!(!expected.rendered.svg.contains("<script>"));
    let tree = resvg::usvg::Tree::from_str(&expected.rendered.svg, &Default::default()).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(
        expected.rendered.width as u32,
        expected.rendered.height as u32,
    )
    .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 200).count() > 500);
    assert_eq!(
        d.render(&options, &mut FailingHost).unwrap_err().message,
        "GitGraph host stopped"
    );
    options.render.output_limit = Some(1);
    assert_eq!(
        d.render(&options, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
    options.render.output_limit = None;
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), expected);
    let g = d.snapshot().unwrap();
    assert_eq!(g.strings[raw as usize].0, [0xd800, 65, 0xdc00]);
    assert!(matches!(field(&g,r,"nan"),Atom::Number(n)if n.to_bits()==0x7ff8000000000042));
    assert!(
        matches!(field(&g,r,"negativeZero"),Atom::Number(n)if n.to_bits()==(-0.0f64).to_bits())
    );
    assert!(d.apply(0, g.clone()).is_err());
    assert_eq!(d.revision(), 1);
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
}
#[test]
fn git_graph_explicit_runtime_gaps_and_work_guards_retain_graph_and_recover() {
    let mut d = document();
    let original = d.snapshot().unwrap();
    let r = root(&original);
    let a = commit(&original, "a");
    let lanes = reference(field(&original, r, "lanes"));
    // Pinned JS: null title throws TypeError reading includes; empty lanes throws
    // TypeError reading push; null tags throws TypeError "B.tags is not iterable".
    // Native render still transports Semantic, so exact JS error kind is a gap.
    for (mode, expected) in [
        ("nullTitle", "active text coercion"),
        ("emptyLanes", "without a lane"),
        ("nullTags", "requires an object"),
        ("undefinedMessage", "active text coercion"),
        ("loneText", "lone UTF-16"),
        ("sparse", "sparse"),
        ("nanSeq", "NaN sequence"),
        ("mapValues", "Map.values"),
        ("work", "work budget"),
    ] {
        let mut g = original.clone();
        match mode {
            "nullTitle" => set(&mut g, r, "title", Atom::Null),
            "emptyLanes" => {
                g.nodes[lanes as usize] = Node::Array {
                    length: 0,
                    properties: vec![],
                }
            }
            "nullTags" => set(&mut g, a, "tags", Atom::Null),
            "undefinedMessage" => set(&mut g, a, "message", Atom::Undefined),
            "loneText" => {
                let id = g.strings.len() as u32;
                g.strings.push(JsString(vec![0xd800]));
                set(&mut g, a, "message", Atom::String(id));
            }
            "sparse" => {
                g.nodes[lanes as usize].properties_mut().remove(0);
            }
            "nanSeq" => set(&mut g, a, "seq", Atom::Number(f64::NAN)),
            "mapValues" => {
                let c = reference(field(&g, r, "commits"));
                set(&mut g, c, "values", Atom::Bool(false));
            }
            "work" => {
                g.nodes[lanes as usize] = Node::Array {
                    length: 1_000_001,
                    properties: vec![],
                }
            }
            _ => unreachable!(),
        }
        let saved = serde_json::to_value(&g).unwrap();
        d.apply(d.revision(), g).unwrap();
        let error = d
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap_err();
        assert!(
            error.message.contains(expected),
            "{mode}: {}",
            error.message
        );
        assert_eq!(
            error.code,
            if mode == "work" {
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
            "29f973e9385987e3aa52e2f7f4f77f333f26f1ca161ea1c44dcc42520ddf4a60",
        );
    }
}

#[test]
fn git_graph_repeated_record_alias_work_is_bounded_without_losing_edits() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let a = commit(&g, "a");
    for i in 0..5000 {
        set(
            &mut g,
            a,
            &format!("application{i}"),
            Atom::Number(i as f64),
        );
    }
    let commits = reference(field(&g, r, "commits"));
    if let Node::Map { entries, .. } = &mut g.nodes[commits as usize] {
        *entries = (0..500)
            .map(|i| (Atom::Number(i as f64), Atom::Ref(a)))
            .collect();
    }
    d.apply(0, g).unwrap();
    let error = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Limit);
    assert!(error.message.contains("property reads"));
    let g = d.snapshot().unwrap();
    assert_eq!(entries(&g, commits).len(), 500);
    assert_eq!(g.nodes[a as usize].properties().len(), 5007);
}

#[test]
fn git_graph_inactive_lone_utf16_lookup_values_do_not_block_render() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let a = commit(&g, "a");
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    set(&mut g, a, "branch", Atom::String(raw));
    let parents = array(&mut g, vec![Atom::String(raw), Atom::Bool(false)]);
    set(&mut g, a, "parents", Atom::Ref(parents));
    d.apply(0, g).unwrap();
    // Pinned a.branch='\\uD800'; a.parents=['\\uD800',false]: unmatched lookups.
    hash(
        &mut d,
        None,
        "29f973e9385987e3aa52e2f7f4f77f333f26f1ca161ea1c44dcc42520ddf4a60",
    );
    let g = d.snapshot().unwrap();
    assert_eq!(g.text(field(&g, a, "branch")).unwrap().0, [0xd800]);
}
