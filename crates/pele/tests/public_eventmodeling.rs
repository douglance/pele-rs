//! EventModeling public native feature slices, Pele 1.6.0 / 458322a5 (MIT kepano).
//! model.ts SHA256 e77eef0dcf372ff8b5394c958c1c6a352d0e6924e169c86905729ffc780fac72.
//! Oracle node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! Public parse(), descriptors, object/array/Set edits and render(model) probes.
//! Both known UTF16 hard-wrap output exclusions are proved separately as gaps.
use pele::compat::{diagrams::event_modeling::EventModeling, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str =
    "eventmodeling\ntf 01 ui CartUI\ntf 02 cmd AddItem { productId: 7 }\ntf 03 evt ItemAdded\n";
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
    e.register(EventModeling);
    e.parse(SOURCE, &ParseOptions::default()).unwrap()
}
fn item(g: &ModelGraph, key: &str, index: u32) -> u32 {
    let id = reference(field(g, root(g), key));
    reference(field(g, id, &index.to_string()))
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
fn object(g: &mut ModelGraph, fields: Vec<(&str, Atom)>) -> u32 {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (k, v) in fields {
        set(g, id, k, v)
    }
    id
}
fn strings(g: &mut ModelGraph, values: &[&str]) -> u32 {
    let a = values.iter().map(|s| string(g, s)).collect();
    array(g, a)
}
fn append(g: &mut ModelGraph, id: u32, a: Atom) {
    let Node::Array { length, .. } = g.nodes[id as usize] else {
        panic!()
    };
    set(g, id, &length.to_string(), a);
    if let Node::Array { length, .. } = &mut g.nodes[id as usize] {
        *length += 1;
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
fn event_modeling_plain_records_own_undefined_flags_and_shared_lanes_boxes() {
    let mut d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    assert_eq!(d.model_schema().id, "pele/eventmodeling@1");
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
            "boxes",
            "lanes",
            "relations",
            "notes",
            "specifications",
            "entities"
        ]
    );
    for k in ["title", "accTitle", "accDescr"] {
        assert!(matches!(field(&g, r, k), Atom::Undefined));
    }
    let b = item(&g, "boxes", 0);
    let lane = reference(field(&g, b, "lane"));
    assert_eq!(
        keys(&g, b),
        [
            "id",
            "kind",
            "entity",
            "name",
            "namespace",
            "reset",
            "lane",
            "data",
            "dataType"
        ]
    );
    assert_eq!(keys(&g, lane), ["index", "label", "group", "namespace"]);
    assert!(matches!(field(&g, b, "namespace"), Atom::Undefined));
    assert!(matches!(field(&g, b, "data"), Atom::Undefined));
    assert!(matches!(field(&g, b, "dataType"), Atom::Undefined));
    assert_eq!(lane, item(&g, "lanes", 0));
    let relation = item(&g, "relations", 0);
    assert_eq!(keys(&g, relation), ["source", "target"]);
    assert_eq!(reference(field(&g, relation, "source")), b);
    assert_eq!(
        reference(field(&g, relation, "target")),
        item(&g, "boxes", 1)
    );
    assert!(g
        .nodes
        .iter()
        .flat_map(|n| n.properties())
        .all(|p| p.flags == 7));
    hash(
        &mut d,
        None,
        "d58c9a36cc045f33c88d6caf113493695d5e89d3bd77d49cd1de22d53fd44a37",
    );
    hash(
        &mut d,
        Some(200.),
        "de71df7fe95b77ced28b45242943edb9f49bc2aac58cf2f591bf94810420a822",
    );
}
#[test]
fn event_modeling_pinned_mutations_preserve_aliases_and_real_native_layout() {
    for (mode, wide, narrow) in [
        (
            "metadata",
            "e51252e254930092d4d958711ceb4cb5ce8162291c98b46657b4f508a58eea9b",
            "884a5402174e462abe5283c15d702c057182b44816611f4f19d5c0942c2c57ab",
        ),
        (
            "records",
            "6a845802f93d0a211a1e43011e0b10f3722c9412651ddfb0679cc034ce5189c7",
            "5799ead68d6f5a3a8e0d3cb09354fa5dbd86382048e831d118c89e53ce247a1c",
        ),
        (
            "alias",
            "425f3a90f6efbc0da79436570908194962cbd5ee4429ca3205305c14016778cc",
            "a79a36162cdca4f355e90299d6846837853a36d9a1a9d6cb3f57522cdb832827",
        ),
        (
            "reverse",
            "1cedaae692dcfdc728ad874d917f93e9f2b9f5e55057e879ceca3a0479678d10",
            "3d4f88e22f77d0073c3d25285f1a9786545e066d923423840437f001be133bd2",
        ),
        (
            "detached",
            "d34d5f95d9d101de29e0dbc2482a4f53b19ef6f4e096ce9a705faef7f5c33462",
            "b877cfe2ffdb467fcd8af8d2f45131c65d9bf0ed3c2dc5dc266f35f4dabe15f1",
        ),
        (
            "ignored",
            "d58c9a36cc045f33c88d6caf113493695d5e89d3bd77d49cd1de22d53fd44a37",
            "de71df7fe95b77ced28b45242943edb9f49bc2aac58cf2f591bf94810420a822",
        ),
        (
            "duplicateId",
            "e00b646799d61147f758488d8243f2672f84f840c88b0ef0c55ecc844aceeb31",
            "7443112cdc920d0616064b3c4c2f4eaf312a57e816fda62c4ee7981093fa3601",
        ),
        (
            "setCollections",
            "3d11c441258be6dc105541ae99c871b5acdb37b9d582634bc65a2e9fd2d7af75",
            "0014b0ada3bdb140301a8079a44eea2fbf0e62b2466ef87286f836695bae0914",
        ),
        (
            "sparseSpec",
            "6638c5abd0998ad3fa9a6f130de712ec43e365133bb56c84a9a2505c580b580b",
            "bf81527b677656b63170dc5b6272e1765bcd76b95cb80dc4d14ae8579e7dfaa3",
        ),
        (
            "laneAlias",
            "3f10a8616ac2ffa2c88c8abb058f48129b46eebf387375a9b19baadeca3e7201",
            "5b3f34f214da5251bcd9b57cd2d1e71e4de225fc274d17384724640a0a303f4a",
        ),
        (
            "relationAlias",
            "058b1c4c43a474e93d04d7776d25bd736fb30a3dd9d18ab0ddb88094b8a63755",
            "484182153a1f55c3f5ec5b753a2b1d9fc06e72097c218ea639d6f6a334752aed",
        ),
        (
            "dataFalsy",
            "b8450532c9a9d7daa03afbad9ebfaf7052d4ffa2e297b11c02758e22d2dc7f53",
            "e4a2bb43ccb07482127bafa0df580b4e1e7408eb64699551c946be43d91912c8",
        ),
        (
            "newNote",
            "7692bebe33ab8fff46f06a92dd7be08c7caccafb02897c085b25996b1b36e343",
            "716cafbd1fce430de97b40a954bcd8cca0f25ad98818894872d0226239818a3a",
        ),
        (
            "reusedRelation",
            "53be7f1d47c8bfbfea2beab038c51cc097e7ba9005b0afda6eaccbe39fa4bbf6",
            "b047e2d99bfbde76c8f0c72e827b0700b0b2af6bab4c9887dd1bb1e68fbba333",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let a = item(&g, "boxes", 0);
        let b = item(&g, "boxes", 1);
        let c = item(&g, "boxes", 2);
        let lane = reference(field(&g, a, "lane"));
        let boxes = reference(field(&g, r, "boxes"));
        let lanes = reference(field(&g, r, "lanes"));
        let relations = reference(field(&g, r, "relations"));
        let notes = reference(field(&g, r, "notes"));
        let specifications = reference(field(&g, r, "specifications"));
        match mode {
            "metadata" => {
                for (k, v) in [
                    ("title", "Edited"),
                    ("accTitle", "Event history"),
                    ("accDescr", "Changes"),
                ] {
                    text(&mut g, r, k, v)
                }
            }
            "records" => {
                text(&mut g, a, "name", "<b>Edited UI</b>");
                text(&mut g, b, "data", "first\nsecond");
                text(&mut g, lane, "label", "Edited lane");
                set(&mut g, c, "reset", Atom::Bool(true));
            }
            "alias" => {
                append(&mut g, boxes, Atom::Ref(a));
                append(&mut g, lanes, Atom::Ref(lane));
            }
            "reverse" => {
                for id in [boxes, lanes] {
                    let values = (0..3)
                        .rev()
                        .map(|i| field(&g, id, &i.to_string()).clone())
                        .collect::<Vec<_>>();
                    for (i, v) in values.into_iter().enumerate() {
                        set(&mut g, id, &i.to_string(), v)
                    }
                }
            }
            "detached" => {
                let detached = object(&mut g, vec![("bad", Atom::Bool(true))]);
                let rel = object(
                    &mut g,
                    vec![("source", Atom::Ref(detached)), ("target", Atom::Ref(a))],
                );
                append(&mut g, relations, Atom::Ref(rel));
                let rel = object(
                    &mut g,
                    vec![("source", Atom::Ref(b)), ("target", Atom::Ref(a))],
                );
                append(&mut g, relations, Atom::Ref(rel));
            }
            "ignored" => {
                set(&mut g, r, "entities", Atom::Null);
                for id in [a, b, c] {
                    set(&mut g, id, "entity", Atom::Null);
                    set(&mut g, id, "namespace", Atom::Bool(true));
                    set(&mut g, id, "dataType", Atom::Number(123.));
                }
                for i in 0..3 {
                    let l = item(&g, "lanes", i);
                    set(&mut g, l, "index", Atom::Number(f64::NAN));
                    text(&mut g, l, "group", "wrong");
                    set(&mut g, l, "namespace", Atom::Null);
                }
            }
            "duplicateId" => {
                text(&mut g, b, "id", "01");
                let frame = string(&mut g, "01");
                let value = string(&mut g, "Note");
                let n = object(
                    &mut g,
                    vec![
                        ("frame", frame.clone()),
                        ("text", value),
                        ("dataType", Atom::Undefined),
                    ],
                );
                append(&mut g, notes, Atom::Ref(n));
                let given = strings(&mut g, &["a"]);
                let when = strings(&mut g, &["b"]);
                let then = strings(&mut g, &["c"]);
                let spec = object(
                    &mut g,
                    vec![
                        ("frame", frame),
                        ("given", Atom::Ref(given)),
                        ("when", Atom::Ref(when)),
                        ("then", Atom::Ref(then)),
                    ],
                );
                append(&mut g, specifications, Atom::Ref(spec));
            }
            "setCollections" => {
                for id in [lanes, relations] {
                    let Node::Array { length, .. } = g.nodes[id as usize] else {
                        panic!()
                    };
                    let values = (0..length)
                        .map(|i| field(&g, id, &i.to_string()).clone())
                        .collect();
                    g.nodes[id as usize] = Node::Set {
                        values,
                        properties: vec![],
                    };
                }
                let frame = string(&mut g, "01");
                let value = string(&mut g, "Set note");
                let note = object(&mut g, vec![("frame", frame), ("text", value)]);
                g.nodes[notes as usize] = Node::Set {
                    values: vec![Atom::Ref(note)],
                    properties: vec![],
                };
                g.nodes[specifications as usize] = Node::Set {
                    values: vec![],
                    properties: vec![],
                };
            }
            "sparseSpec" => {
                let frame = string(&mut g, "01");
                let x = string(&mut g, "x");
                let given = array(
                    &mut g,
                    vec![
                        Atom::Undefined,
                        Atom::Null,
                        Atom::Number(3.),
                        Atom::Bool(true),
                        Atom::Undefined,
                        x,
                    ],
                );
                g.nodes[given as usize].properties_mut().remove(4);
                let when = array(&mut g, vec![]);
                let then = array(
                    &mut g,
                    vec![
                        Atom::Bool(false),
                        Atom::Number(f64::NAN),
                        Atom::Number(f64::INFINITY),
                    ],
                );
                let spec = object(
                    &mut g,
                    vec![
                        ("frame", frame),
                        ("given", Atom::Ref(given)),
                        ("when", Atom::Ref(when)),
                        ("then", Atom::Ref(then)),
                    ],
                );
                append(&mut g, specifications, Atom::Ref(spec));
            }
            "laneAlias" => {
                set(&mut g, b, "lane", Atom::Ref(lane));
                text(&mut g, lane, "label", "Shared lane");
            }
            "relationAlias" => {
                let rel = item(&g, "relations", 0);
                set(&mut g, rel, "source", Atom::Ref(c));
                set(&mut g, rel, "target", Atom::Ref(a));
            }
            "dataFalsy" => {
                set(&mut g, b, "data", Atom::Null);
                set(&mut g, a, "data", Atom::Bool(false));
                set(&mut g, r, "accTitle", Atom::Number(0.));
                set(&mut g, r, "accDescr", Atom::Number(f64::NAN));
            }
            "newNote" => {
                let frame = string(&mut g, "03");
                let value = string(&mut g, "<script>not active</script>\n    kept");
                let n = object(&mut g, vec![("frame", frame), ("text", value)]);
                append(&mut g, notes, Atom::Ref(n));
            }
            "reusedRelation" => {
                let rel = item(&g, "relations", 0);
                append(&mut g, relations, Atom::Ref(rel));
            }
            _ => unreachable!(),
        };
        d.apply(0, g).unwrap();
        let saved = serde_json::to_value(d.snapshot().unwrap()).unwrap();
        hash(&mut d, None, wide);
        hash(&mut d, Some(200.), narrow);
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
            _ => panic!("EventModeling baseline uses only records, maps and arrays"),
        },
        _ => panic!("EventModeling baseline has no host references"),
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
fn mutable_eventmodeling_preserves_entire_existing_pinned_render_and_error_corpus() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/eventmodeling.json"
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
    e.register(EventModeling);

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

struct FailingHost;
impl RenderResources for FailingHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new(
            "EventModeling host stopped",
            ErrorCode::Semantic,
        ))
    }
}
#[test]
fn event_modeling_native_edits_metadata_lifetime_flat_extras_and_resource_recovery() {
    let mut e = PublicEngine::empty();
    e.register(EventModeling);
    let mut d = e.parse(SOURCE, &ParseOptions::default()).unwrap();
    e.register(EventModeling);
    drop(e);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let a = item(&g, "boxes", 0);
    let b = item(&g, "boxes", 1);
    text(&mut g, a, "name", "EditedUI");
    text(&mut g, b, "data", "new: 9");
    d.apply(0, g).unwrap();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let expected = Engine::with_builtins()
        .render_source(
            &SOURCE
                .replace("CartUI", "EditedUI")
                .replace("productId: 7", "new: 9"),
            &options,
            &mut MetricsResources,
        )
        .unwrap();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), expected);
    let mut g = d.snapshot().unwrap();
    text(&mut g, r, "accTitle", "Native event model");
    text(&mut g, r, "accDescr", "Edited history");
    text(&mut g, r, "type", "pie");
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    set(&mut g, r, "applicationRaw", Atom::String(raw));
    set(&mut g, a, "entity", Atom::String(raw));
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
    let xml = roxmltree::Document::parse(&native.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Native event model")));
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("desc") && n.text() == Some("Edited history")));
    assert_eq!(native.diagram_type.as_str(), "eventmodeling");
    let tree = resvg::usvg::Tree::from_str(&native.rendered.svg, &Default::default()).unwrap();
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
        d.render(&options, &mut FailingHost).unwrap_err().message,
        "EventModeling host stopped"
    );
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
fn event_modeling_runtime_shape_and_finite_work_errors_retain_graph_and_recover() {
    let mut d = document();
    let original = d.snapshot().unwrap();
    let r = root(&original);
    let a = item(&original, "boxes", 0);
    let boxes = reference(field(&original, r, "boxes"));
    for (mode, message) in [
        ("kind", "unknown runtime entity kind"),
        ("nullLane", "requires an object"),
        ("detachedLane", "must be present"),
        ("sparse", "sparse active"),
        ("dataNumber", "active text coercion"),
        ("loneText", "lone UTF-16"),
        ("join", "Array.join"),
        ("layoutWork", "layout exceeds"),
        ("arrayWork", "collections exceed"),
        ("repeatedText", "imported text exceeds"),
    ] {
        let mut g = original.clone();
        match mode {
            "kind" => text(&mut g, a, "kind", "futureKind"),
            "nullLane" => set(&mut g, a, "lane", Atom::Null),
            "detachedLane" => {
                let label = string(&mut g, "Detached");
                let lane = object(&mut g, vec![("label", label)]);
                set(&mut g, a, "lane", Atom::Ref(lane));
            }
            "sparse" => {
                g.nodes[boxes as usize].properties_mut().remove(0);
            }
            "dataNumber" => set(&mut g, a, "data", Atom::Number(1.)),
            "loneText" => {
                let raw = g.strings.len() as u32;
                g.strings.push(JsString(vec![0xd800]));
                set(&mut g, a, "name", Atom::String(raw));
            }
            "join" => {
                let frame = string(&mut g, "01");
                let given = array(&mut g, vec![]);
                set(&mut g, given, "join", Atom::Bool(false));
                let when = array(&mut g, vec![]);
                let then = array(&mut g, vec![]);
                let spec = object(
                    &mut g,
                    vec![
                        ("frame", frame),
                        ("given", Atom::Ref(given)),
                        ("when", Atom::Ref(when)),
                        ("then", Atom::Ref(then)),
                    ],
                );
                let specs = reference(field(&g, r, "specifications"));
                append(&mut g, specs, Atom::Ref(spec));
            }
            "layoutWork" => {
                let rel = item(&g, "relations", 0);
                let id = reference(field(&g, r, "relations"));
                let properties = (0..12000)
                    .map(|i| {
                        let key = if let Atom::String(k) = string(&mut g, &i.to_string()) {
                            k
                        } else {
                            unreachable!()
                        };
                        Property {
                            key,
                            value: Atom::Ref(rel),
                            flags: 7,
                        }
                    })
                    .collect();
                g.nodes[id as usize] = Node::Array {
                    length: 12000,
                    properties,
                };
            }
            "arrayWork" => {
                g.nodes[boxes as usize] = Node::Array {
                    length: 1_000_001,
                    properties: vec![],
                }
            }
            "repeatedText" => {
                text(&mut g, a, "data", &"x".repeat(60000));
                let properties = (0..300)
                    .map(|i| {
                        let key = if let Atom::String(k) = string(&mut g, &i.to_string()) {
                            k
                        } else {
                            unreachable!()
                        };
                        Property {
                            key,
                            value: Atom::Ref(a),
                            flags: 7,
                        }
                    })
                    .collect();
                g.nodes[boxes as usize] = Node::Array {
                    length: 300,
                    properties,
                };
            }
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
            if mode.ends_with("Work") || mode == "repeatedText" {
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
            "d58c9a36cc045f33c88d6caf113493695d5e89d3bd77d49cd1de22d53fd44a37",
        );
    }
}
#[test]
fn event_modeling_existing_utf16_hard_wrap_output_exclusions_remain_explicit_gaps() {
    use std::io::Read;
    let mut bytes = vec![];
    flate2::read::GzDecoder::new(
        include_bytes!("../../../fixtures/pele/eventmodeling.json.gz").as_slice(),
    )
    .read_to_end(&mut bytes)
    .unwrap();
    let f: Json = serde_json::from_slice(&bytes).unwrap();
    let excluded = f["extraction"]["utf16OutputExclusions"].as_array().unwrap();
    assert_eq!(excluded.len(), 2);
    assert_eq!(f["extraction"]["surrogateExcluded"], 0);
    let mut e = PublicEngine::empty();
    e.register(EventModeling);
    for c in excluded {
        let options: RenderOptions = serde_json::from_value(c["options"].clone()).unwrap();
        let mut d = e
            .parse(c["source"].as_str().unwrap(), &options.parse)
            .unwrap();
        let graph = d.snapshot().unwrap();
        assert!(graph.strings.iter().all(|s| s.well_formed().is_some()));
        let r = d.render(&options, &mut MetricsResources).unwrap();
        let actual = r
            .rendered
            .svg
            .encode_utf16()
            .map(|u| u as u64)
            .collect::<Vec<_>>();
        let raw = c["svgUtf16"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect::<Vec<_>>();
        assert_ne!(
            actual, raw,
            "excluded output must not be represented as passing parity"
        );
        assert!(r.rendered.svg.contains('\u{fffd}'));
        assert_eq!(
            serde_json::to_value(d.snapshot().unwrap()).unwrap(),
            serde_json::to_value(graph).unwrap()
        );
    }
}
