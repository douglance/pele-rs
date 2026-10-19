//! Venn mutable feature slices: pinned Pele1.6.0 / 458322a5 (MIT kepano).
//! db.ts SHA256 c5d43a0f6edd46f0d70b37b19a973fc840d78e2820259116f6b3bff6f405610e.
//! dist/pele.js SHA256 d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! Public parse/Reflect.ownKeys/prototype descriptors, method operation traces,
//! mutated render(model) whole-SVG SHA256; Node25.9.0 arm64 numeric oracle.
use pele::compat::{diagrams::venn::Venn, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = r#"venn-beta
set A["Alpha"]:20
  text a["A text"]
set B["Beta"]:14
union A,B["Both"]:3
style A fill:gold,color:blue"#;
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
    e.register(Venn);
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
                            || !["known", "current", "indent"]
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
            _ => panic!("Venn baseline uses only records, maps and arrays"),
        },
        _ => panic!("Venn baseline has no host references"),
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
fn mutable_venn_preserves_entire_existing_pinned_render_and_error_corpus() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/venn.json"
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
    e.register(Venn);

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
fn actual_venn_db_fields_descriptors_set_order_and_current_alias() {
    let mut d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    assert_eq!(d.model_schema().id, "pele/venn@1");
    assert_eq!(
        keys(&g, r),
        [
            "type",
            "title",
            "subsets",
            "textNodes",
            "styles",
            "known",
            "current",
            "indent"
        ]
    );
    assert!(matches!(
        g.nodes[r as usize],
        Node::Instance {
            prototype: PrototypeId(1),
            ..
        }
    ));
    assert_eq!(
        d.model_schema()
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        [
            ("setDiagramTitle", 1),
            ("addSubsetData", 3),
            ("addTextData", 3),
            ("addStyleData", 2),
            ("validateUnionIdentifiers", 1),
            ("getCurrentSets", 0),
            ("getIndentMode", 0),
            ("setIndentMode", 1)
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
    assert_eq!(keys(&g, item(&g, "subsets", 0)), ["sets", "size", "label"]);
    assert_eq!(keys(&g, item(&g, "textNodes", 0)), ["sets", "id", "label"]);
    assert_eq!(keys(&g, item(&g, "styles", 0)), ["targets", "styles"]);
    let known = reference(field(&g, r, "known"));
    let Node::Set { values, .. } = &g.nodes[known as usize] else {
        panic!()
    };
    assert_eq!(
        values
            .iter()
            .map(|a| historical_json(&g, a).unwrap())
            .collect::<Vec<_>>(),
        [Json::String("A".into()), Json::String("B".into())]
    );
    let current = reference(field(&g, r, "current"));
    assert_eq!(
        current,
        reference(field(&g, item(&g, "subsets", 2), "sets"))
    );
    let o = run(&mut d, "getCurrentSets", vec![]);
    assert_eq!(reference(&o.result.unwrap()), current);
    assert!(matches!(
        run(&mut d, "getIndentMode", vec![]).result,
        Ok(Atom::Bool(false))
    ));
    hash(
        &mut d,
        None,
        "7bc6102dabcb098ced9655a4f0738a957f38fde7acd379f1ec9de38455b8f168",
    );
}
fn edit(g: &mut ModelGraph, mode: &str) {
    let r = root(g);
    let a = item(g, "subsets", 0);
    let b = item(g, "subsets", 1);
    let union = item(g, "subsets", 2);
    let t = item(g, "textNodes", 0);
    let style = item(g, "styles", 0);
    match mode {
        "metadata" => {
            text(g, r, "title", "  \"Edited title\"  ");
            text(g, r, "accTitle", "Ignored");
            text(g, r, "accDescr", "Ignored")
        }
        "records" => {
            text(g, a, "label", "Edited Alpha");
            set(g, b, "size", Atom::Number(30.));
            text(g, union, "label", "Edited union");
            text(g, t, "id", "new");
            text(g, t, "label", "Edited text")
        }
        "alias" => {
            for (k, v) in [("subsets", a), ("textNodes", t), ("styles", style)] {
                let id = reference(field(g, r, k));
                append(g, id, Atom::Ref(v))
            }
        }
        "order" => {
            let id = reference(field(g, r, "subsets"));
            set(g, id, "0", Atom::Ref(union));
            set(g, id, "2", Atom::Ref(a));
            let sets = reference(field(g, union, "sets"));
            let a = field(g, sets, "0").clone();
            let b = field(g, sets, "1").clone();
            set(g, sets, "0", b);
            set(g, sets, "1", a)
        }
        "null-labels" => {
            set(g, a, "label", Atom::Null);
            set(g, b, "label", Atom::Bool(false));
            set(g, union, "label", Atom::Number(0.));
            set(g, t, "label", Atom::Null)
        }
        "styles" => {
            let map = reference(field(g, style, "styles"));
            for (k, v) in [("fill", "tomato"), ("stroke", "red"), ("color", "purple")] {
                let v = string(g, v);
                map_set(g, map, k, v)
            }
            map_set(g, map, "stroke-width", Atom::Number(2.))
        }
        "inactive" => {
            text(g, r, "type", "pie");
            set(g, r, "known", Atom::Undefined);
            set(g, r, "current", Atom::Null);
            set(g, r, "indent", Atom::Number(f64::NAN));
            text(g, r, "accTitle", "Ignored");
            text(g, r, "accDescr", "Ignored");
            let sets = list(g, &["X"]);
            let text = object(
                g,
                vec![
                    ("sets", sets),
                    ("id", Atom::Null),
                    ("label", Atom::Number(1.)),
                ],
            );
            let texts = reference(field(g, r, "textNodes"));
            append(g, texts, text)
        }
        "duplicate" => {
            let sets = list(g, &["A"]);
            let label = string(g, "Replacement");
            let subset = object(
                g,
                vec![("sets", sets), ("size", Atom::Number(9.)), ("label", label)],
            );
            let subsets = reference(field(g, r, "subsets"));
            append(g, subsets, subset)
        }
        "detached" => {
            let targets = list(g, &["missing"]);
            let key = string(g, "fill");
            let value = string(g, "red");
            let map = g.nodes.len() as u32;
            g.nodes.push(Node::Map {
                entries: vec![(key, value)],
                properties: vec![],
            });
            let style = object(g, vec![("targets", targets), ("styles", Atom::Ref(map))]);
            let styles = reference(field(g, r, "styles"));
            append(g, styles, style)
        }
        "string-sizes" => {
            text(g, a, "size", "30");
            set(g, b, "size", Atom::Null);
            text(g, union, "size", "2")
        }
        "baseline" => {}
        _ => panic!("{mode}"),
    }
}
fn method_arguments(d: &mut PublicDocument, ids: &[&str], rest: &[Atom]) -> Vec<Atom> {
    let mut g = d.snapshot().unwrap();
    let ids = list(&mut g, ids);
    let mut values = vec![ids];
    values.extend_from_slice(rest);
    d.apply(d.revision(), g).unwrap();
    values
}
fn method_string_arguments(d: &mut PublicDocument, ids: &[&str], rest: &[&str]) -> Vec<Atom> {
    let mut g = d.snapshot().unwrap();
    let ids = list(&mut g, ids);
    let mut values = vec![ids];
    values.extend(rest.iter().map(|s| string(&mut g, s)));
    d.apply(d.revision(), g).unwrap();
    values
}
fn style_arguments(d: &mut PublicDocument, ids: &[&str], pairs: &[(&str, Atom)]) -> Vec<Atom> {
    let mut g = d.snapshot().unwrap();
    let ids = list(&mut g, ids);
    let mut data = vec![];
    for (k, v) in pairs {
        let k = string(&mut g, k);
        data.push(array(&mut g, vec![k, v.clone()]))
    }
    let data = array(&mut g, data);
    d.apply(d.revision(), g).unwrap();
    vec![ids, data]
}
fn string_arg(d: &mut PublicDocument, s: &str) -> Atom {
    let mut g = d.snapshot().unwrap();
    let v = string(&mut g, s);
    d.apply(d.revision(), g).unwrap();
    v
}
fn apply_methods(d: &mut PublicDocument, defaults: bool) {
    if !defaults {
        let arg = string_arg(d, " Methods ");
        run(d, "setDiagramTitle", vec![arg]).result.unwrap();
    }
    let label = if defaults {
        Atom::Undefined
    } else {
        string_arg(d, " \"Gamma\" ")
    };
    let args = method_arguments(
        d,
        if defaults { &["C"] } else { &[" \"C\" "] },
        &[
            label,
            if defaults {
                Atom::Undefined
            } else {
                Atom::Number(12.)
            },
        ],
    );
    run(d, "addSubsetData", args).result.unwrap();
    let label = if defaults {
        Atom::Undefined
    } else {
        string_arg(d, " \"AC\" ")
    };
    let args = method_arguments(
        d,
        &["C", " A "],
        &[
            label,
            if defaults {
                Atom::Null
            } else {
                Atom::Number(2.)
            },
        ],
    );
    run(d, "addSubsetData", args).result.unwrap();
    let args = if defaults {
        let id = string_arg(d, "text");
        method_arguments(d, &["C"], &[id, Atom::Undefined])
    } else {
        method_string_arguments(d, &["C", "A"], &[" text ", " \"Method content\" "])
    };
    run(d, "addTextData", args).result.unwrap();
    let pairs = if defaults {
        vec![("fill", string_arg(d, "")), ("opacity", Atom::Number(0.))]
    } else {
        vec![
            ("fill", string_arg(d, " \"tomato\" ")),
            ("color", string_arg(d, "purple")),
            ("fill", string_arg(d, "gold")),
        ]
    };
    let args = style_arguments(d, if defaults { &["C"] } else { &["C", "A"] }, &pairs);
    run(d, "addStyleData", args).result.unwrap();
    if !defaults {
        let args = method_arguments(d, &["A", " B ", "\"C\""], &[]);
        run(d, "validateUnionIdentifiers", args).result.unwrap();
        run(d, "setIndentMode", vec![Atom::Bool(true)])
            .result
            .unwrap();
    }
}
#[test]
fn exact_pinned_edited_and_method_generated_svg() {
    for (mode, expected) in [
        (
            "baseline",
            "7bc6102dabcb098ced9655a4f0738a957f38fde7acd379f1ec9de38455b8f168",
        ),
        (
            "metadata",
            "e1099d6744ab8a4b2f845db2e39a110a0d1eb512afb8660bc6334e4e7689d973",
        ),
        (
            "records",
            "daec9732d11236dddc9cb291da7ac0b4e26b56206d4cc6b360f572783b8663c1",
        ),
        (
            "alias",
            "a047786b530f1756c85b7fce9e21b919c124338d2ad097129150dd133a36298e",
        ),
        (
            "order",
            "156c2e00ad7f195ed7f54df61b25657457a4d865442b5a87b988c2300fca2c76",
        ),
        (
            "null-labels",
            "d189356db899462db9542e53557e5db6f0f697312e75747fea0e68f49fb803ac",
        ),
        (
            "styles",
            "0943f5e3891a1a53e70b40ce543f4d9d7a32b8fb2e5697726440ed5c3ff2a115",
        ),
        (
            "inactive",
            "7bc6102dabcb098ced9655a4f0738a957f38fde7acd379f1ec9de38455b8f168",
        ),
        (
            "duplicate",
            "ce49920e9a3044f98126415e2e8fed3ef49eeb72192f68aa0a4c448806f5b188",
        ),
        (
            "detached",
            "7bc6102dabcb098ced9655a4f0738a957f38fde7acd379f1ec9de38455b8f168",
        ),
        (
            "methods",
            "0535c363836c06035b3d157919e4024d1b5f350de7dadbf15d18a32c0b394b1d",
        ),
        (
            "empty-default",
            "1f3db6449674a0b5829ffda041edc313b39333f4fb90f96eb209cadb529cbefc",
        ),
        (
            "string-sizes",
            "30141dd36ebd912c616e683ef6ce0a5094b5a3c46b8ef433ee23104b5f802f21",
        ),
    ] {
        let mut d = document();
        if mode == "methods" || mode == "empty-default" {
            apply_methods(&mut d, mode == "empty-default")
        } else {
            let mut g = d.snapshot().unwrap();
            edit(&mut g, mode);
            d.apply(0, g).unwrap();
        }
        for width in [None, Some(160.)] {
            hash(&mut d, width, expected)
        }
    }
}
#[test]
fn actual_method_partial_errors_return_aliases_and_semantic_validation() {
    let mut d = document();
    let before = d.snapshot().unwrap();
    let count = reference(field(&before, root(&before), "subsets"));
    let args = method_arguments(&mut d, &[" \"C\" "], &[Atom::Number(1.)]);
    let rev = d.revision();
    let o = run(&mut d, "addSubsetData", args);
    assert_eq!(o.revision, rev + 1);
    let g = error(
        o,
        JavaScriptErrorKind::TypeError,
        "e.trim is not a function",
    );
    let r = root(&g);
    assert!(matches!(
        g.nodes[count as usize],
        Node::Array { length: 3, .. }
    ));
    let current = reference(field(&g, r, "current"));
    assert!(matches!(field(&g,current,"0"),Atom::String(i)if g.strings[*i as usize].is("C")));
    let known = reference(field(&g, r, "known"));
    let Node::Set { values, .. } = &g.nodes[known as usize] else {
        panic!()
    };
    assert_eq!(values.len(), 3);
    let args = method_arguments(&mut d, &["C"], &[]);
    run(&mut d, "validateUnionIdentifiers", args)
        .result
        .unwrap();
    // Failure mutated current/known but did not add a renderable subset.
    hash(
        &mut d,
        None,
        "7bc6102dabcb098ced9655a4f0738a957f38fde7acd379f1ec9de38455b8f168",
    );
    let args = method_arguments(&mut d, &["X", "Y"], &[]);
    match run(&mut d, "validateUnionIdentifiers", args).result {
        Err(PublicError::Pele(e)) => {
            assert_eq!(e.message, "unknown set identifier: X, Y");
            assert_eq!(e.code, ErrorCode::Semantic);
            assert_eq!(e.diagram_type.as_deref(), Some("venn"));
        }
        x => panic!("{x:?}"),
    }
    for (name, args, message) in [
        (
            "addSubsetData",
            vec![Atom::Null],
            "Cannot read properties of null (reading 'map')",
        ),
        (
            "addSubsetData",
            vec![Atom::Number(1.)],
            "e.map is not a function",
        ),
        (
            "addStyleData",
            vec![Atom::Undefined, Atom::Null],
            "n is not iterable",
        ),
    ] {
        error(
            run(&mut d, name, args),
            JavaScriptErrorKind::TypeError,
            message,
        );
    }
    let args = method_arguments(&mut d, &["A"], &[Atom::Undefined]);
    error(
        run(&mut d, "addTextData", args),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of undefined (reading 'trim')",
    );
    let args = method_arguments(&mut d, &["A"], &[Atom::Number(1.)]);
    error(
        run(&mut d, "addTextData", args),
        JavaScriptErrorKind::TypeError,
        "e.trim is not a function",
    );
    let raw = string_arg(&mut d, "raw mode");
    run(&mut d, "setIndentMode", vec![raw.clone()])
        .result
        .unwrap();
    let o = run(&mut d, "getIndentMode", vec![]);
    assert_eq!(
        serde_json::to_value(o.result.unwrap()).unwrap(),
        serde_json::to_value(raw).unwrap()
    );
    // Real prototype method callable on an independent plain object.
    let mut g = d.snapshot().unwrap();
    let receiver = object(&mut g, vec![]);
    let title = string(&mut g, "Borrowed");
    d.apply(d.revision(), g).unwrap();
    call(&mut d, "setDiagramTitle", receiver.clone(), vec![title])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    assert!(
        matches!(field(&g,reference(&receiver),"title"),Atom::String(i)if g.strings[*i as usize].is("Borrowed"))
    );
}
#[test]
fn raw_utf16_normalization_sparse_map_alias_and_partial_render_recovery() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![32, 34, 0xd800, 34, 32]));
    let ids = array(&mut g, vec![Atom::String(raw)]);
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addSubsetData", vec![ids, Atom::Undefined])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    let current = reference(field(&g, root(&g), "current"));
    let Atom::String(i) = field(&g, current, "0") else {
        panic!()
    };
    assert_eq!(g.strings[*i as usize].0, [0xd800]);
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let mut g = d.snapshot().unwrap();
    let current = reference(field(&g, root(&g), "current"));
    text(&mut g, current, "0", "C");
    d.apply(d.revision(), g).unwrap();
    d.render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    let ids = array(&mut g, vec![Atom::String(raw)]);
    d.apply(d.revision(), g).unwrap();
    match run(&mut d, "validateUnionIdentifiers", vec![ids]).result {
        Err(PublicError::PeleUtf16(e)) => {
            assert_eq!(e.message.0.last(), Some(&0xd800));
            assert_eq!(e.code, ErrorCode::Semantic)
        }
        x => panic!("{x:?}"),
    }
    let mut g = d.snapshot().unwrap();
    let sparse = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: 1,
        properties: vec![],
    });
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addSubsetData", vec![Atom::Ref(sparse)])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    let current = reference(field(&g, root(&g), "current"));
    assert!(g.nodes[current as usize].properties().is_empty());
    let known = reference(field(&g, root(&g), "known"));
    assert!(
        matches!(&g.nodes[known as usize],Node::Set{values,..}if matches!(values.last(),Some(Atom::Undefined)))
    );
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("sparse"));
}
struct FailingText;
impl RenderResources for FailingText {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("Venn text stopped", ErrorCode::Semantic))
    }
}
#[test]
fn native_render_source_equivalence_lifetime_lossless_extras_and_recovery() {
    let mut e = PublicEngine::empty();
    e.register(Venn);
    let mut d = e.parse(SOURCE, &ParseOptions::default()).unwrap();
    e.register(Venn);
    drop(e);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let first = item(&g, "subsets", 0);
    text(&mut g, first, "label", "Native Alpha");
    d.apply(0, g).unwrap();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let expected = Engine::with_builtins()
        .render_source(
            &SOURCE.replace("Alpha", "Native Alpha"),
            &options,
            &mut MetricsResources,
        )
        .unwrap();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), expected);
    let mut g = d.snapshot().unwrap();
    text(&mut g, r, "type", "pie");
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    set(&mut g, r, "applicationRaw", Atom::String(raw));
    set(&mut g, r, "known", Atom::String(raw));
    for (k, n) in [
        ("nan", f64::from_bits(0x7ff8000000000042)),
        ("negativeZero", -0.),
        ("infinity", f64::INFINITY),
    ] {
        set(&mut g, r, k, Atom::Number(n))
    }
    let mut previous = r;
    for _ in 0..5000 {
        let id = g.nodes.len() as u32;
        g.nodes.push(Node::Object { properties: vec![] });
        set(&mut g, previous, "next", Atom::Ref(id));
        previous = id
    }
    set(&mut g, previous, "next", Atom::Ref(r));
    g.nodes[r as usize].properties_mut()[0].flags = 0;
    let bytes = serde_json::to_vec(&g).unwrap();
    d.apply(d.revision(), serde_json::from_slice(&bytes).unwrap())
        .unwrap();
    let saved = serde_json::to_value(d.snapshot().unwrap()).unwrap();
    let native = d.render(&options, &mut MetricsResources).unwrap();
    assert_eq!(native.diagram_type.as_str(), "venn");
    assert_eq!(native.rendered.svg, expected.rendered.svg);
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
        "Venn text stopped"
    );
    let mut small = options.clone();
    small.render.output_limit = Some(8);
    assert_eq!(
        d.render(&small, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), native);
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
    let rev = d.revision();
    assert!(d.apply(rev - 1, d.snapshot().unwrap()).is_err());
    assert_eq!(d.revision(), rev);
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
}
#[test]
fn bounded_alias_collection_import_and_method_recovery() {
    let mut d = document();
    let original = d.snapshot().unwrap();
    let mut g = original.clone();
    let subsets = reference(field(&g, root(&g), "subsets"));
    if let Node::Array { length, .. } = &mut g.nodes[subsets as usize] {
        *length = 1_000_001
    }
    d.apply(0, g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    d.apply(d.revision(), original.clone()).unwrap();
    let mut g = d.snapshot().unwrap();
    let huge = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: 1_000_001,
        properties: vec![],
    });
    d.apply(d.revision(), g).unwrap();
    assert!(
        matches!(run(&mut d,"addSubsetData",vec![Atom::Ref(huge)]).result,Err(PublicError::Pele(e))if e.code==ErrorCode::Limit)
    );
    d.apply(d.revision(), original).unwrap();
    hash(
        &mut d,
        None,
        "7bc6102dabcb098ced9655a4f0738a957f38fde7acd379f1ec9de38455b8f168",
    );
    // Layout materializes undeclared edited identifiers with the upstream
    // default size, even though validateUnionIdentifiers would reject them.
    let mut g = d.snapshot().unwrap();
    let sets = reference(field(&g, item(&g, "subsets", 2), "sets"));
    text(&mut g, sets, "0", "undeclared");
    d.apply(d.revision(), g).unwrap();
    hash(
        &mut d,
        None,
        "59f676ba274e09a6ca10a5099637785c6e735ad5ffe041b31642331b1c740e7f",
    );
}

#[test]
fn readonly_collection_length_preserves_earlier_real_db_mutations() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let subsets = reference(field(&g, r, "subsets"));
    let key = if let Atom::String(i) = string(&mut g, "length") {
        i
    } else {
        unreachable!()
    };
    g.nodes[subsets as usize].properties_mut().push(Property {
        key,
        value: Atom::Number(3.),
        flags: 0,
    });
    d.apply(0, g).unwrap();
    let arguments = method_arguments(&mut d, &["C"], &[]);
    let o = run(&mut d, "addSubsetData", arguments);
    let g = error(
        o,
        JavaScriptErrorKind::TypeError,
        "Cannot assign to read only property 'length' of object '[object Array]'",
    );
    assert!(matches!(
        g.nodes[subsets as usize],
        Node::Array { length: 3, .. }
    ));
    let current = reference(field(&g, r, "current"));
    assert!(matches!(field(&g,current,"0"),Atom::String(i)if g.strings[*i as usize].is("C")));
    let known = reference(field(&g, r, "known"));
    assert!(matches!(&g.nodes[known as usize],Node::Set{values,..}if values.len()==3));
    hash(
        &mut d,
        None,
        "7bc6102dabcb098ced9655a4f0738a957f38fde7acd379f1ec9de38455b8f168",
    );
}

#[test]
fn typed_factory_retains_actual_bookkeeping_after_public_typed_collection_edits() {
    let mut model = pele::compat::diagrams::venn::parse(SOURCE, &ParseOptions::default()).unwrap();
    model.subsets[2].sets = vec!["A".into()];
    model
        .subsets
        .push(pele::compat::diagrams::venn::VennSubset {
            sets: vec!["C".into()],
            size: 9.,
            label: Some("Typed C".into()),
        });
    model.subsets.reverse();
    let mut d = PublicDocument::from_module(Venn, &model, Object::default()).unwrap();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let known = reference(field(&g, r, "known"));
    let Node::Set { values, .. } = &g.nodes[known as usize] else {
        panic!()
    };
    assert_eq!(
        values
            .iter()
            .map(|v| historical_json(&g, v).unwrap())
            .collect::<Vec<_>>(),
        [Json::String("A".into()), Json::String("B".into())]
    );
    let current = reference(field(&g, r, "current"));
    assert_eq!(
        historical_json(&g, &Atom::Ref(current)).unwrap(),
        serde_json::json!(["A", "B"])
    );
    for i in 0..model.subsets.len() {
        assert_ne!(
            current,
            reference(field(&g, item(&g, "subsets", i), "sets"))
        )
    }
    let args = method_arguments(&mut d, &["C"], &[]);
    assert!(
        matches!(run(&mut d,"validateUnionIdentifiers",args).result,Err(PublicError::Pele(e))if e.message=="unknown set identifier: C")
    );
    let expected = Venn
        .render(
            &model,
            &Object::default(),
            &RenderSettings::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        expected
    );
    // Scalar-model current aliases cannot be reconstructed by current array index
    // after reordering. Match its last equal value; canonical parser output always
    // aliases exactly the final inserted subset's sets.
    let mut original =
        pele::compat::diagrams::venn::parse(SOURCE, &ParseOptions::default()).unwrap();
    original.subsets.reverse();
    let d = PublicDocument::from_module(Venn, &original, Object::default()).unwrap();
    let g = d.snapshot().unwrap();
    assert_eq!(
        reference(field(&g, root(&g), "current")),
        reference(field(&g, item(&g, "subsets", 0), "sets"))
    );
}
