//! Pinned C4Db 19-method mutable feature slices, Pele 1.6.0 / 458322a5.
//! db.ts SHA256 01f1f59de8e806297383f555091ba8f331d75c79d330656cf33363773f37f8f1.
//! dist/pele.js SHA256 d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! Oracle public parse/descriptors/method traces/render(model), Node25.9.0 arm64.
use pele::compat::{diagrams::c4::C4, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = r#"C4Context
Person(user,"Customer")
System_Boundary(bank,"Bank") {
System(app,"Banking")
ContainerDb(store,"Ledger","SQL")
}
Rel(user,app,"Uses","HTTPS")
Rel(app,store,"Reads")"#;
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
    e.register(C4);
    e.parse(SOURCE, &ParseOptions::default()).unwrap()
}
fn array(g: &mut ModelGraph, values: Vec<Atom>) -> Atom {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: values.len() as u32,
        properties: vec![],
    });
    for (i, v) in values.into_iter().enumerate() {
        set(g, id, &i.to_string(), v)
    }
    Atom::Ref(id)
}
fn list(g: &mut ModelGraph, values: &[&str]) -> Atom {
    let v = values.iter().map(|s| string(g, s)).collect();
    array(g, v)
}
fn object(g: &mut ModelGraph, fields: Vec<(&str, Atom)>) -> Atom {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (k, v) in fields {
        set(g, id, k, v)
    }
    Atom::Ref(id)
}
fn append(g: &mut ModelGraph, id: u32, v: Atom) {
    let Node::Array { length, .. } = g.nodes[id as usize] else {
        panic!()
    };
    set(g, id, &length.to_string(), v);
    if let Node::Array { length, .. } = &mut g.nodes[id as usize] {
        *length += 1
    }
}
fn item(g: &ModelGraph, key: &str, i: usize) -> u32 {
    reference(field(g, reference(field(g, root(g), key)), &i.to_string()))
}
fn map_set(g: &mut ModelGraph, id: u32, k: &str, v: Atom) {
    let found = if let Node::Map { entries, .. } = &g.nodes[id as usize] {
        entries
            .iter()
            .position(|(key, _)| matches!(key,Atom::String(i)if g.strings[*i as usize].is(k)))
    } else {
        panic!()
    };
    let key = string(g, k);
    if let Node::Map { entries, .. } = &mut g.nodes[id as usize] {
        if let Some(i) = found {
            entries[i].1 = v
        } else {
            entries.push((key, v))
        }
    }
}
fn call(
    d: &mut PublicDocument,
    name: &str,
    receiver: Atom,
    arguments: Vec<Atom>,
) -> InvocationOutcome {
    let method = d
        .model_schema()
        .methods
        .iter()
        .find(|m| m.name == name)
        .unwrap()
        .id;
    d.invoke(
        d.revision(),
        MethodCall {
            method,
            receiver,
            arguments,
        },
        &mut MetricsResources,
    )
    .unwrap()
}
fn run(d: &mut PublicDocument, name: &str, arguments: Vec<Atom>) -> InvocationOutcome {
    let r = d.snapshot().unwrap().root;
    call(d, name, r, arguments)
}
fn error(o: InvocationOutcome, kind: JavaScriptErrorKind, message: &str) -> ModelGraph {
    match o.result {
        Err(PublicError::JavaScript { name, message: m }) => {
            assert_eq!(name, kind);
            assert!(m.is(message));
        }
        a => panic!("wrong error: {a:?}"),
    };
    o.graph
}
fn hash(d: &mut PublicDocument, max_width: Option<f64>, expected: &str) {
    let mut options = RenderOptions::default();
    options.render.max_width = max_width;
    let r = d.render(&options, &mut MetricsResources).unwrap();
    roxmltree::Document::parse(&r.rendered.svg).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(r.rendered.svg.as_bytes())),
        expected
    );
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
                    .filter(|p| {
                        *id != root(g)
                            || ![
                                "shapeIndex",
                                "boundaryIndex",
                                "relIndex",
                                "current",
                                "stack",
                            ]
                            .iter()
                            .any(|k| g.strings[p.key as usize].is(k))
                    })
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
            _ => panic!("C4 baseline uses only records, maps and arrays"),
        },
        _ => panic!("C4 baseline has no host references"),
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
fn mutable_c4_preserves_entire_existing_pinned_render_and_error_corpus() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/c4.json"
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
    e.register(C4);

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
fn actual_c4_db_descriptors_bookkeeping_and_named_property_order() {
    let mut d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    assert_eq!(d.model_schema().id, "pele/c4@1");
    assert_eq!(
        keys(&g, r),
        [
            "type",
            "c4Type",
            "title",
            "accTitle",
            "accDescr",
            "shapes",
            "boundaries",
            "rels",
            "shapeInRow",
            "boundaryInRow",
            "shapeIndex",
            "boundaryIndex",
            "relIndex",
            "current",
            "stack"
        ]
    );
    assert_eq!(
        d.model_schema()
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        [
            ("setC4Type", 1),
            ("setTitle", 1),
            ("setAccDescription", 1),
            ("element", 4),
            ("addPersonOrSystem", 2),
            ("addContainer", 2),
            ("addComponent", 2),
            ("finishShape", 4),
            ("boundary", 2),
            ("open", 1),
            ("addPersonOrSystemBoundary", 1),
            ("addContainerBoundary", 1),
            ("addDeploymentNode", 2),
            ("popBoundaryParseStack", 0),
            ("addRel", 2),
            ("getC4Shape", 1),
            ("updateElStyle", 2),
            ("updateRelStyle", 2),
            ("updateLayoutConfig", 2)
        ]
    );
    assert!(d.model_schema().prototypes[0]
        .members
        .iter()
        .all(|m| m.flags == 6));
    assert!(g
        .nodes
        .iter()
        .flat_map(|n| n.properties())
        .all(|p| p.flags == 7));
    assert_eq!(
        keys(&g, item(&g, "boundaries", 0)),
        ["alias", "label", "type", "parent"]
    );
    assert_eq!(
        keys(&g, item(&g, "boundaries", 1)),
        ["alias", "label", "parent", "labelAttr", "type"]
    );
    assert_eq!(
        keys(&g, item(&g, "shapes", 0)),
        ["alias", "label", "parent", "labelAttr", "descr", "kind"]
    );
    assert_eq!(
        keys(&g, item(&g, "rels", 0)),
        ["type", "from", "to", "label", "labelAttr", "techn", "descr"]
    );
    assert!(matches!(
        field(&g, item(&g, "shapes", 0), "labelAttr"),
        Atom::Undefined
    ));
    assert_eq!(
        reference(field(&g, r, "current")),
        item(&g, "boundaries", 0)
    );
    let arg = field(&g, item(&g, "shapes", 0), "alias").clone();
    assert_eq!(
        reference(&run(&mut d, "getC4Shape", vec![arg]).result.unwrap()),
        item(&g, "shapes", 0)
    );
    let source="C4Context\nSystem($bgColor=\"red\",$label=\"Named\",$techn=\"Rust\",$custom=\"one\",$tags=\"tag\",$bgColor=\"blue\")";
    let mut e = PublicEngine::empty();
    e.register(C4);
    let d = e.parse(source, &ParseOptions::default()).unwrap();
    let g = d.snapshot().unwrap();
    assert_eq!(
        keys(&g, item(&g, "shapes", 0)),
        [
            "alias",
            "label",
            "parent",
            "aliasAttr",
            "bgColor",
            "labelAttr",
            "techn",
            "extra",
            "tags",
            "kind"
        ]
    );
}
fn edit(g: &mut ModelGraph, mode: &str) {
    let r = root(g);
    let user = item(g, "shapes", 0);
    let app = item(g, "shapes", 1);
    let store = item(g, "shapes", 2);
    let bank = item(g, "boundaries", 1);
    let rel = item(g, "rels", 0);
    let rel2 = item(g, "rels", 1);
    match mode {
        "baseline" => {}
        "metadata" => {
            for (k, v) in [
                ("c4Type", "C4Dynamic"),
                ("title", "Edited title"),
                ("accTitle", "Accessible"),
                ("accDescr", "Description"),
            ] {
                text(g, r, k, v)
            }
        }
        "records" => {
            for (id, k, v) in [
                (user, "label", "Edited customer"),
                (app, "descr", "Changed description"),
                (store, "techn", "Postgres"),
                (bank, "label", "Changed bank"),
                (bank, "type", "Enterprise"),
                (rel, "label", "Edited relation"),
            ] {
                text(g, id, k, v)
            }
            set(g, rel2, "offsetX", Atom::Number(22.))
        }
        "styles" => {
            for (id, k, v) in [
                (app, "bgColor", "gold"),
                (app, "fontColor", "purple"),
                (app, "borderColor", "red"),
                (app, "shape", "queue"),
                (bank, "bgColor", "pink"),
                (bank, "fontColor", "blue"),
                (bank, "borderColor", "green"),
                (rel, "lineColor", "red"),
                (rel, "textColor", "purple"),
            ] {
                text(g, id, k, v)
            }
            set(g, rel, "offsetX", Atom::Number(14.));
            set(g, rel, "offsetY", Atom::Number(-9.))
        }
        "index-shape" => {
            let map = reference(field(g, r, "shapeIndex"));
            map_set(g, map, "user", Atom::Ref(app))
        }
        "index-boundary" => {
            text(g, rel, "to", "bankAlias");
            let map = reference(field(g, r, "boundaryIndex"));
            map_set(g, map, "bankAlias", Atom::Ref(bank))
        }
        "index-nullish" => {
            let map = reference(field(g, r, "shapeIndex"));
            map_set(g, map, "user", Atom::Null);
            let map = reference(field(g, r, "boundaryIndex"));
            map_set(g, map, "user", Atom::Ref(bank))
        }
        "alias-shape" | "alias-boundary" | "alias-rel" => {
            let (k, v) = if mode == "alias-shape" {
                ("shapes", user)
            } else if mode == "alias-boundary" {
                ("boundaries", bank)
            } else {
                ("rels", rel)
            };
            let list = reference(field(g, r, k));
            append(g, list, Atom::Ref(v))
        }
        "cross-kind" => {
            text(g, bank, "kind", "system");
            let list = reference(field(g, r, "shapes"));
            append(g, list, Atom::Ref(bank));
            text(g, rel, "to", "bank")
        }
        "inactive" => {
            set(g, user, "parent", Atom::Null);
            set(g, user, "label", Atom::Null);
            set(g, user, "kind", Atom::Number(1.));
            let rels = array(g, vec![]);
            set(g, r, "rels", rels);
            text(g, r, "type", "pie");
            set(g, r, "shapeIndex", Atom::Null);
            set(g, r, "boundaryIndex", Atom::Bool(false))
        }
        "foreign-parent" => {
            let foreign = object(g, vec![("label", Atom::Null)]);
            set(g, bank, "parent", foreign);
            let rels = array(g, vec![]);
            set(g, r, "rels", rels)
        }
        _ => panic!("{mode}"),
    }
}
fn string_arg(d: &mut PublicDocument, s: &str) -> Atom {
    let mut g = d.snapshot().unwrap();
    let v = string(&mut g, s);
    d.apply(d.revision(), g).unwrap();
    v
}
fn attrs(d: &mut PublicDocument, values: &[Option<&str>]) -> Atom {
    let mut g = d.snapshot().unwrap();
    let values = values
        .iter()
        .map(|s| s.map_or(Atom::Undefined, |s| string(&mut g, s)))
        .collect();
    let a = array(&mut g, values);
    d.apply(d.revision(), g).unwrap();
    a
}
fn add(d: &mut PublicDocument, method: &str, kind: Option<&str>, values: &[Option<&str>]) -> Atom {
    let a = attrs(d, values);
    let arguments = if let Some(kind) = kind {
        vec![string_arg(d, kind), a]
    } else {
        vec![a]
    };
    run(d, method, arguments).result.unwrap()
}
fn apply_methods(d: &mut PublicDocument) {
    for (method, s) in [
        ("setC4Type", "C4Dynamic"),
        ("setTitle", "Method diagram"),
        ("setAccDescription", "A\n   description\n\n end"),
    ] {
        let a = string_arg(d, s);
        run(d, method, vec![a]).result.unwrap();
    }
    add(
        d,
        "addPersonOrSystem",
        Some("external_person"),
        &[Some("guest"), Some("Guest"), Some("Outside")],
    );
    add(
        d,
        "addPersonOrSystem",
        Some("person"),
        &[Some("user"), Some("Edited user")],
    );
    add(
        d,
        "addContainer",
        Some("container_queue"),
        &[Some("queue"), Some("Queue"), Some("AMQP"), Some("Jobs")],
    );
    add(
        d,
        "addComponent",
        Some("component"),
        &[
            Some("service"),
            Some("Service"),
            Some("Rust"),
            Some("Worker"),
        ],
    );
    add(
        d,
        "addPersonOrSystemBoundary",
        None,
        &[Some("team"), Some("Team")],
    );
    add(
        d,
        "addPersonOrSystem",
        Some("system"),
        &[Some("inside"), Some("Inside")],
    );
    run(d, "popBoundaryParseStack", vec![]).result.unwrap();
    add(
        d,
        "addContainerBoundary",
        None,
        &[Some("container"), Some("Container")],
    );
    add(
        d,
        "addComponent",
        Some("component"),
        &[Some("child"), Some("Child")],
    );
    run(d, "popBoundaryParseStack", vec![]).result.unwrap();
    add(
        d,
        "addDeploymentNode",
        Some("nodeL"),
        &[Some("node"), Some("Node"), None, Some("Runs")],
    );
    add(
        d,
        "addContainer",
        Some("container"),
        &[Some("runtime"), Some("Runtime"), Some("Rust")],
    );
    run(d, "popBoundaryParseStack", vec![]).result.unwrap();
    add(
        d,
        "addRel",
        Some("birel"),
        &[Some("guest"), Some("queue"), Some("Sends"), Some("TLS")],
    );
    add(
        d,
        "updateElStyle",
        Some("ignored"),
        &[
            Some("queue"),
            Some("gold"),
            Some("blue"),
            Some("red"),
            None,
            Some("queue"),
        ],
    );
    add(
        d,
        "updateRelStyle",
        Some("ignored"),
        &[
            Some("guest"),
            Some("queue"),
            Some("purple"),
            Some("green"),
            Some("12.5px"),
            Some("-3"),
        ],
    );
    add(
        d,
        "updateLayoutConfig",
        Some("ignored"),
        &[Some("3"), Some("2")],
    );
}
fn private_methods(d: &mut PublicDocument) {
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let list = field(&g, r, "shapes").clone();
    let index = field(&g, r, "shapeIndex").clone();
    let alias = string_arg(d, "extra");
    let title = string_arg(d, "Extra");
    let el = run(d, "element", vec![list, index, alias, title])
        .result
        .unwrap();
    let kind = string_arg(d, "system");
    let args = attrs(d, &[Some("extra"), Some("Extra"), None, Some("sprite")]);
    run(d, "finishShape", vec![el, kind, args, Atom::Number(3.)])
        .result
        .unwrap();
    let args = attrs(d, &[Some("helper"), Some("Helper")]);
    let fallback = string_arg(d, "custom");
    let boundary = run(d, "boundary", vec![args, fallback]).result.unwrap();
    run(d, "open", vec![boundary]).result.unwrap();
    add(
        d,
        "addPersonOrSystem",
        Some("system"),
        &[Some("nested"), Some("Nested")],
    );
    run(d, "popBoundaryParseStack", vec![]).result.unwrap();
    add(
        d,
        "addRel",
        Some("rel"),
        &[Some("extra"), Some("nested"), Some("Connects")],
    );
}
fn named_methods(d: &mut PublicDocument) {
    let mut g = d.snapshot().unwrap();
    let mut values = vec![];
    for (k, v) in [
        ("bgColor", "gold"),
        ("label", "Named title"),
        ("techn", "Rust"),
        ("custom", "one"),
        ("custom", "two"),
    ] {
        let key = string(&mut g, k);
        let value = string(&mut g, v);
        values.push(object(&mut g, vec![("key", key), ("value", value)]));
    }
    let values = array(&mut g, values);
    let kind = string(&mut g, "container");
    d.apply(d.revision(), g).unwrap();
    run(d, "addContainer", vec![kind, values]).result.unwrap();
}
#[test]
fn exact_pinned_method_and_alias_mutation_svg() {
    for (mode, wide, narrow) in [
        (
            "baseline",
            "01c214c51b6fc6942a6a06d26c9a9f5f5c2cead63fb786892ac2415c91165078",
            "ae3af64ea8bb94f5ac6d82ffcc0953bf0f26802980cfd561842bb3265e35cc73",
        ),
        (
            "metadata",
            "0c60f947bbffd194c5091174f6bf1b88c072c66919c7e33ee3c34cccba2e8e3e",
            "9d1991335ad098e5d2de155423bef9094880fc0f981ba4ee34e308a66ba6c8ff",
        ),
        (
            "records",
            "6b2b5351035a73d7d5b3d6e83c721d8091ae828ed3de33c3073fd1bda4e3999b",
            "04c44974e21a6f137a1b83c32e4b6c980d67b5248ca0f3f018f4a746d7f5cf47",
        ),
        (
            "styles",
            "837a0c65b452102cd60bc84d9836f7106e3e6310eecea5c58dba3918df2ab13e",
            "91f1e7bb5d5a46a439c8e5f25c26f1e2693e19cf44efb921275eef7181225141",
        ),
        (
            "index-shape",
            "f91f8f4f1d2e6422a8426d7d167157d9a0a59c7a3826169ce73ae875f850f303",
            "15c392d4c05ea414df0e27f4dc59835c4e9c3d8c816add967ed25c88ca638a51",
        ),
        (
            "index-boundary",
            "a34dbcdb10bc30643c18dea3eecf650807ac52338d294c723401d1e1087fc07d",
            "152211bd8514dbff13a7637eb7d56f38fdbe2ca378ba8b047754a70b8a815d43",
        ),
        (
            "index-nullish",
            "3e9d029a0ff724bbe1d604156c00e9e8d605a73c25f8bb535d0e00d578261adc",
            "62353cebb204713840c425b922294786722496600df048a752f7dcfe69528aae",
        ),
        (
            "alias-shape",
            "7fd9bb655caf17bd0dfe19449b3bccb0e3d20378960477ddc4564a96746edd3c",
            "0b79853628a94f7d4c8876f16c45c4f51fd5a38e34f630f65cea3974d0b34ac1",
        ),
        (
            "alias-boundary",
            "01c214c51b6fc6942a6a06d26c9a9f5f5c2cead63fb786892ac2415c91165078",
            "ae3af64ea8bb94f5ac6d82ffcc0953bf0f26802980cfd561842bb3265e35cc73",
        ),
        (
            "alias-rel",
            "f7cffafc3f777b96a88ed825e97844da21e1bca7ab4a298c4d6449d0f7ecf856",
            "52fdfc31e00222e5b6e827ec1dcff8ed046bb2ef8c29561c804e9bf23b34f940",
        ),
        (
            "cross-kind",
            "5c77a6d38770886ae53a873459be56bde5188e9db161601664f2887fb67eb833",
            "98c3535231456810e38dfd3e8274bf5c98ce906116a3b5d579660edae21ddbbd",
        ),
        (
            "inactive",
            "5b78870d3ae6da82849f63f2d84ac19261381821272fdf45a308dae8384b8c25",
            "3d0eb4fb8b54f7e53d2a62324a29ba17cf84d943ef77418ded82c42aabe2eaba",
        ),
        (
            "foreign-parent",
            "e9bea2dd6cb97284cecc4bfc07900545c8b186e3c146998716c06932b3ab6e73",
            "e9bea2dd6cb97284cecc4bfc07900545c8b186e3c146998716c06932b3ab6e73",
        ),
        (
            "methods",
            "2752d3233612c3a348e43e8f762edee08f96a99872915c94263c031cb949616d",
            "be3821088ee1d79024c4819630c87222f5b376645e5d13e066d39413aabde426",
        ),
        (
            "private-helpers",
            "d3800cf5938150bb6152f60fec9ea1abb244b1d7eb10ee29f1015280f6021f0b",
            "475c3a81554d51a25b88f9c094fdb69586ab4fa6a2305acdc51d0d8f20836692",
        ),
        (
            "named",
            "5c9bdf33d1e60186f5c1795b4882db319cabaa9549a857530d0f6a6cd89dd176",
            "1fc09b46b78f688b4dd52de6c3aeaa8b64279ad024b5cb8a601bfe5509e8382a",
        ),
    ] {
        let mut d = document();
        match mode {
            "methods" => apply_methods(&mut d),
            "private-helpers" => private_methods(&mut d),
            "named" => named_methods(&mut d),
            _ => {
                let mut g = d.snapshot().unwrap();
                edit(&mut g, mode);
                d.apply(0, g).unwrap();
            }
        }
        hash(&mut d, None, wide);
        hash(&mut d, Some(280.), narrow);
    }
}
#[test]
fn actual_method_partial_mutations_alias_returns_and_error_recovery() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let user = item(&g, "shapes", 0);
    let args = list(&mut g, &["user"]);
    let args_id = reference(&args);
    append(&mut g, args_id, Atom::Null);
    let kind = string(&mut g, "system");
    d.apply(0, g).unwrap();
    let rev = d.revision();
    let o = run(&mut d, "addPersonOrSystem", vec![kind, args]);
    assert_eq!(o.revision, rev + 1);
    let g = error(
        o,
        JavaScriptErrorKind::TypeError,
        "Cannot destructure property 'key' of 't' as it is null.",
    );
    assert!(matches!(field(&g,user,"label"),Atom::String(i)if g.strings[*i as usize].is("")));
    assert!(matches!(field(&g, user, "labelAttr"), Atom::Null));
    hash(
        &mut d,
        None,
        "e3edebefaae136e60bae58e8a30239d9d86b2cd410f91ce33998935b4a5feca5",
    );
    hash(
        &mut d,
        Some(280.),
        "5812359ec55a3160711ccfcb1a635f82c32f05148a86210a39804c29f6c9c6ee",
    );
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let empty = array(&mut g, vec![]);
    let kind = string(&mut g, "system");
    d.apply(0, g).unwrap();
    let o = run(&mut d, "addPersonOrSystem", vec![kind, empty]);
    let g = error(
        o,
        JavaScriptErrorKind::TypeError,
        "Cannot destructure property 'key' of 't' as it is undefined.",
    );
    assert_eq!(
        keys(&g, item(&g, "shapes", 3)),
        ["alias", "label", "parent", "aliasAttr"]
    );
    for (value, message) in [
        (Atom::Number(1.), "t.replace is not a function"),
        (
            Atom::Null,
            "Cannot read properties of null (reading 'replace')",
        ),
    ] {
        error(
            run(&mut d, "setAccDescription", vec![value]),
            JavaScriptErrorKind::TypeError,
            message,
        );
    }
    let mut d = document();
    let g = d.snapshot().unwrap();
    let user = item(&g, "shapes", 0);
    let arg = field(&g, user, "alias").clone();
    assert_eq!(
        reference(&run(&mut d, "getC4Shape", vec![arg]).result.unwrap()),
        user
    );
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let shapes = field(&g, r, "shapes").clone();
    let index = field(&g, r, "shapeIndex").clone();
    let alias = string_arg(&mut d, "user");
    let title = string_arg(&mut d, "Direct element");
    let returned = run(&mut d, "element", vec![shapes, index, alias, title])
        .result
        .unwrap();
    assert_eq!(reference(&returned), user);
    // Nested method receiver errors publish earlier writes and remain repairable.
    let mut g = d.snapshot().unwrap();
    let root = root(&g);
    set(&mut g, root, "element", Atom::Number(1.));
    d.apply(d.revision(), g).unwrap();
    let args = attrs(&mut d, &[Some("x"), Some("X")]);
    let kind = string_arg(&mut d, "system");
    error(
        run(&mut d, "addPersonOrSystem", vec![kind, args]),
        JavaScriptErrorKind::TypeError,
        "this.element is not a function",
    );
}
struct Trace {
    calls: Vec<String>,
}
impl RenderResources for Trace {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        self.calls.push(format!(
            "{}|{}|{}",
            serde_json::to_string(r.text).unwrap(),
            ryu_js::Buffer::new().format(r.size),
            r.style
        ));
        Ok(r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 })
    }
}
#[test]
fn duplicate_boundary_identity_retains_exact_pinned_host_measurements() {
    for (duplicate, count, expected) in [
        (
            false,
            12,
            "5a784256977ced04831ef51fcfee63079940e15495e080cdc464152bfe0998ff",
        ),
        (
            true,
            14,
            "f6c1c0285e2744e59aacbef67a7589df1b729f713de5c96d1ed076cea1eaa236",
        ),
    ] {
        let mut d = document();
        if duplicate {
            let mut g = d.snapshot().unwrap();
            edit(&mut g, "alias-boundary");
            d.apply(0, g).unwrap();
        }
        let mut trace = Trace { calls: vec![] };
        let rendered = d.render(&RenderOptions::default(), &mut trace).unwrap();
        assert_eq!(trace.calls.len(), count);
        assert_eq!(
            format!("{:x}", Sha256::digest(trace.calls.join("\n").as_bytes())),
            expected
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(rendered.rendered.svg.as_bytes())),
            "6b1a31f152d4db1cb162b479c195c04bd250df67a8673ef4cbfbb45a57a0b0bf"
        );
    }
}
struct FailingText;
impl RenderResources for FailingText {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("C4 text stopped", ErrorCode::Semantic))
    }
}
#[test]
fn native_source_equivalence_lifetime_lossless_flat_graph_and_recovery() {
    let mut e = PublicEngine::empty();
    e.register(C4);
    let mut d = e.parse(SOURCE, &ParseOptions::default()).unwrap();
    e.register(C4);
    drop(e);
    let mut g = d.snapshot().unwrap();
    let user = item(&g, "shapes", 0);
    text(&mut g, user, "label", "Native customer");
    d.apply(0, g).unwrap();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let expected = Engine::with_builtins()
        .render_source(
            &SOURCE.replace("Customer", "Native customer"),
            &options,
            &mut MetricsResources,
        )
        .unwrap();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), expected);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    text(&mut g, r, "type", "pie");
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    set(&mut g, r, "applicationRaw", Atom::String(raw));
    set(&mut g, r, "current", Atom::String(raw));
    set(&mut g, r, "stack", Atom::Null);
    set(
        &mut g,
        r,
        "nan",
        Atom::Number(f64::from_bits(0x7ff8000000000042)),
    );
    set(&mut g, r, "negativeZero", Atom::Number(-0.));
    let mut at = r;
    for _ in 0..5000 {
        let id = g.nodes.len() as u32;
        g.nodes.push(Node::Object { properties: vec![] });
        set(&mut g, at, "next", Atom::Ref(id));
        at = id
    }
    set(&mut g, at, "next", Atom::Ref(r));
    g.nodes[r as usize].properties_mut()[0].flags = 0;
    let bytes = serde_json::to_vec(&g).unwrap();
    d.apply(d.revision(), serde_json::from_slice(&bytes).unwrap())
        .unwrap();
    let saved = serde_json::to_value(d.snapshot().unwrap()).unwrap();
    let native = d.render(&options, &mut MetricsResources).unwrap();
    assert_eq!(native.rendered.svg, expected.rendered.svg);
    assert_eq!(native.diagram_type.as_str(), "c4");
    let mut svg_options = resvg::usvg::Options::default();
    svg_options
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec());
    svg_options.fontdb_mut().set_sans_serif_family("Noto Sans");
    svg_options.font_family = "Noto Sans".into();
    let tree = resvg::usvg::Tree::from_str(&native.rendered.svg, &svg_options).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(
        tree.size().width().ceil() as u32,
        tree.size().height().ceil() as u32,
    )
    .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    assert!(pixmap.data().chunks_exact(4).any(|p| p[3] != 0));
    assert_eq!(
        d.render(&options, &mut FailingText).unwrap_err().message,
        "C4 text stopped"
    );
    let mut small = options.clone();
    small.render.output_limit = Some(8);
    assert_eq!(
        d.render(&small, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), native);
    let rev = d.revision();
    assert!(d.apply(rev - 1, d.snapshot().unwrap()).is_err());
    assert_eq!(d.revision(), rev);
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
}
#[test]
fn cyclic_boundary_work_guard_raw_utf16_and_repair() {
    let mut d = document();
    let original = d.snapshot().unwrap();
    let mut g = original.clone();
    let global = item(&g, "boundaries", 0);
    let bank = item(&g, "boundaries", 1);
    set(&mut g, global, "parent", Atom::Ref(bank));
    d.apply(0, g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("cyclic reachable"));
    d.apply(d.revision(), original.clone()).unwrap();
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings
        .push(JsString(vec![65, 10, 32, 0xd800, 10, 32, 32, 66]));
    let raw = Atom::String(raw);
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "setAccDescription", vec![raw]).result.unwrap();
    let g = d.snapshot().unwrap();
    let Atom::String(i) = field(&g, root(&g), "accDescr") else {
        panic!()
    };
    assert_eq!(g.strings[*i as usize].0, [65, 10, 0xd800, 10, 66]);
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    d.apply(d.revision(), original.clone()).unwrap();
    let mut g = d.snapshot().unwrap();
    let shapes = reference(field(&g, root(&g), "shapes"));
    if let Node::Array { length, .. } = &mut g.nodes[shapes as usize] {
        *length = 1_000_001
    }
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    d.apply(d.revision(), original).unwrap();
    hash(
        &mut d,
        None,
        "01c214c51b6fc6942a6a06d26c9a9f5f5c2cead63fb786892ac2415c91165078",
    );
}
#[test]
fn readonly_pop_keeps_deleted_index_partial_state_and_repairs() {
    for full in [false, true] {
        let mut d = document();
        if full {
            let g = d.snapshot().unwrap();
            let bank = item(&g, "boundaries", 1);
            run(&mut d, "open", vec![Atom::Ref(bank)]).result.unwrap();
        }
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let current = field(&g, r, "current").clone();
        let stack = reference(field(&g, r, "stack"));
        let key = if let Atom::String(i) = string(&mut g, "length") {
            i
        } else {
            unreachable!()
        };
        g.nodes[stack as usize].properties_mut().push(Property {
            key,
            value: Atom::Number(if full { 1. } else { 0. }),
            flags: 0,
        });
        d.apply(d.revision(), g).unwrap();
        let g = error(
            run(&mut d, "popBoundaryParseStack", vec![]),
            JavaScriptErrorKind::TypeError,
            "Cannot assign to read only property 'length' of object '[object Array]'",
        );
        assert_eq!(
            serde_json::to_value(field(&g, r, "current")).unwrap(),
            serde_json::to_value(current).unwrap()
        );
        assert!(matches!(g.nodes[stack as usize],Node::Array{length,..}if length==u32::from(full)));
        assert!(!g.nodes[stack as usize]
            .properties()
            .iter()
            .any(|p| g.strings[p.key as usize].is("0")));
        let mut g = g;
        g.nodes[stack as usize]
            .properties_mut()
            .retain(|p| !g.strings[p.key as usize].is("length"));
        d.apply(d.revision(), g).unwrap();
        run(&mut d, "popBoundaryParseStack", vec![]).result.unwrap();
        let g = d.snapshot().unwrap();
        assert_eq!(
            reference(field(&g, r, "current")),
            item(&g, "boundaries", 0)
        );
    }
}

#[test]
fn renderer_callbacks_are_explicit_and_retained_builtin_lookup_remains_callable() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let properties = std::mem::take(g.nodes[r as usize].properties_mut());
    g.nodes[r as usize] = Node::Object { properties };
    d.apply(0, g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("getC4Shape render callbacks"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "getC4Shape", Atom::BuiltinFunction(15));
    d.apply(d.revision(), g).unwrap();
    hash(
        &mut d,
        None,
        "01c214c51b6fc6942a6a06d26c9a9f5f5c2cead63fb786892ac2415c91165078",
    );
    let mut g = d.snapshot().unwrap();
    let rels = reference(field(&g, r, "rels"));
    set(&mut g, rels, "map", Atom::Number(1.));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("relation Array render callbacks"));
    let mut g = d.snapshot().unwrap();
    g.nodes[rels as usize]
        .properties_mut()
        .retain(|p| !g.strings[p.key as usize].is("map"));
    set(&mut g, r, "getC4Shape", Atom::Number(1.));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("getC4Shape render callbacks"));
}
