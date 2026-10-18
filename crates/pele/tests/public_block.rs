//! Pinned BlockDb 11-function mutable feature slices, Pele 1.6.0 / 458322a5.
//! db.ts SHA256 2f0e6110f62875625fbbc45556bb1d5c23446fbbaa38d00374210ad8ef978aed.
//! dist/pele.js SHA256 d1ebc1889f3069d7759fe5db34blockad4b4823285edbee2b537d0ef3d505a76d8d.
//! Oracle public parse/descriptors/method traces/render(model), Node25.9.0 arm64.
use pele::compat::{diagrams::block::Block, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = "block\ncolumns 2\nblock:g[\"Group\"]:2\ncolumns 2\na[\"Alpha\"] b((\"Beta\"))\nend\nc<[\"Arrow\"]>(right) space:2 d{{\"Delta\"}}\na -- \"Flow\" --> d\nclassDef hot fill:#f96,color:blue\nclass a hot";
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
    e.register(Block);
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
                            || !["warn", "pending", "edgeCount", "ids", "colors", "spaces"]
                                .iter()
                                .any(|k| g.strings[p.key as usize].is(k))
                    })
                    .filter_map(|p| {
                        historical_json(g, &p.value)
                            .map(|v| (g.strings[p.key as usize].well_formed().unwrap(), v))
                    })
                    .collect(),
            )),
            Node::Array { length, properties } => {
                let mut values = vec![Json::Null; *length as usize];
                for p in properties {
                    if let Some(k) = g.strings[p.key as usize].well_formed() {
                        if let Ok(i) = k.parse::<u32>() {
                            if i < *length && i.to_string() == k {
                                values[i as usize] =
                                    historical_json(g, &p.value).unwrap_or(Json::Null);
                            }
                        }
                    }
                }
                Some(Json::Array(values))
            }
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
            _ => panic!("Block baseline uses only records, maps and arrays"),
        },
        _ => panic!("Block baseline has no host references"),
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
fn mutable_block_preserves_entire_existing_pinned_render_and_error_corpus() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/block.json"
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
    e.register(Block);

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
fn by_id(g: &ModelGraph, key: &str) -> u32 {
    let Node::Map { entries, .. } = &g.nodes[reference(field(g, root(g), "byId")) as usize] else {
        panic!()
    };
    reference(
        &entries
            .iter()
            .find(|(k, _)| matches!(k,Atom::String(i)if g.strings[*i as usize].is(key)))
            .unwrap()
            .1,
    )
}
fn string_arg(d: &mut PublicDocument, s: &str) -> Atom {
    let mut g = d.snapshot().unwrap();
    let a = string(&mut g, s);
    d.apply(d.revision(), g).unwrap();
    a
}
#[test]
fn actual_block_db_descriptors_private_state_own_order_and_function_shapes() {
    let d = document();
    let g = d.snapshot().unwrap();
    assert_eq!(
        keys(&g, root(&g)),
        [
            "type",
            "title",
            "root",
            "blocks",
            "edges",
            "classes",
            "byId",
            "typeStr2Type",
            "edgeStrToEdgeData",
            "edgeStrToEdgeStartData",
            "edgeStrToThickness",
            "edgeStrToPattern",
            "warn",
            "pending",
            "edgeCount",
            "ids",
            "colors",
            "spaces"
        ]
    );
    let r = reference(field(&g, root(&g), "root"));
    assert_eq!(keys(&g, r), ["id", "type", "children", "columns"]);
    assert_eq!(
        reference(field(&g, root(&g), "blocks")),
        reference(field(&g, r, "children"))
    );
    assert_eq!(
        keys(&g, by_id(&g, "a")),
        [
            "id",
            "label",
            "type",
            "directions",
            "widthInColumns",
            "children",
            "classes"
        ]
    );
    assert!(matches!(field(&g, root(&g), "ids"), Atom::Number(n) if *n == 2.));
    assert!(matches!(field(&g, root(&g), "colors"), Atom::Number(n) if *n == 1.));
    assert!(matches!(field(&g, root(&g), "spaces"), Atom::Number(n) if *n == 2.));
    assert_eq!(d.model_schema().id, "pele/block@1");
    assert_eq!(d.model_schema().methods.len(), 11);
    assert_eq!(Block.shared_functions().len(), 5);
    assert!(Block.function_shapes().iter().all(|s| s.constructible));
    for p in g.nodes[root(&g) as usize].properties() {
        assert_eq!(p.flags, 7)
    }
    let mut d = document();
    for (name, input, expected) in [
        ("typeStr2Type", "(())", "circle"),
        ("typeStr2Type", "[\\\\]", "lean_left"),
        ("typeStr2Type", "other", "na"),
        ("edgeStrToEdgeData", "  o--x  ", "arrow_cross"),
        ("edgeStrToEdgeStartData", "  o--x  ", "arrow_circle"),
        ("edgeStrToThickness", "<==>", "thick"),
        ("edgeStrToPattern", "-.->", "dotted"),
    ] {
        let a = string_arg(&mut d, input);
        let out = run(&mut d, name, vec![a]);
        let Atom::String(id) = out.result.unwrap() else {
            panic!()
        };
        assert!(out.graph.strings[id as usize].is(expected));
    }
    let out = run(&mut d, "generateId", vec![]);
    let Atom::String(id) = out.result.unwrap() else {
        panic!()
    };
    assert!(out.graph.strings[id as usize].is("id-3"));
}
fn edit(g: &mut ModelGraph, mode: &str) {
    let r = root(g);
    match mode {
        "none" => {}
        "label" => {
            let a = by_id(g, "a");
            text(g, a, "label", "Edited <b>label</b> 中文")
        }
        "repeat" => {
            let a = by_id(g, "g");
            let children = reference(field(g, reference(field(g, r, "root")), "children"));
            append(g, children, Atom::Ref(a));
        }
        "edge" => {
            let a = item(g, "edges", 0);
            let edges = reference(field(g, r, "edges"));
            append(g, edges, Atom::Ref(a));
            text(g, a, "label", "Double")
        }
        "metadata" => {
            text(g, r, "title", "Edited title");
            text(g, r, "accTitle", "Accessible & <Block>");
            text(g, r, "accDescr", "Description\nline")
        }
        "ignored" => {
            let a = array(g, vec![]);
            set(g, r, "blocks", a);
            let id = g.nodes.len() as u32;
            g.nodes.push(Node::Map {
                entries: vec![],
                properties: vec![],
            });
            set(g, r, "byId", Atom::Ref(id));
            set(g, r, "pending", Atom::Null);
            set(g, r, "ids", Atom::Number(f64::NAN));
        }
        "shape" => {
            let a = by_id(g, "a");
            let group = by_id(g, "g");
            let c = by_id(g, "c");
            text(g, a, "type", "cylinder");
            set(g, group, "columns", Atom::Number(1.));
            let values = list(g, &["x", "up"]);
            set(g, c, "directions", values)
        }
        _ => panic!(),
    }
}
#[test]
fn exact_pinned_mutated_alias_and_method_svgs() {
    for (mode, wide, narrow) in [
        (
            "none",
            "351690a5b4901c62b9ca6f6bb9b6d0d34d6ed6b11661fa49501a82944dc0de32",
            "1de0f0e886b00313cd6a20c5baeae498c014d4d52d79026ec6b785afe68590d7",
        ),
        (
            "label",
            "04647c79053b4a0cb4187e5ab1e197e4d1ea7060f0a82b91a31aa95743ec394c",
            "e8d80085543557d0a4385e1b2ab0dc631736bfbc28429064894b2dee05777c0c",
        ),
        (
            "repeat",
            "9c2fe2da128ffa746273ddc6571f1deb18ea7a42a6f7aef52a5e8d2ccdf940dd",
            "cbd24c0c839dd7248933ba986ab5c045eb6a0ad81e0df6eefc0694b927d30764",
        ),
        (
            "edge",
            "7cc31c7c6920325b90a6ce6a09570a4e5f67812fa077152815109d1831282543",
            "16fb9f66389c30b5338282d251e2f03d57788523589d5a8afd6de8e5bdcf902d",
        ),
        (
            "metadata",
            "9dd23d84bdce0245152241c49afbba6ea83737a2ea80dd3b9328720a5b9e921e",
            "d95b90e9278c2d345e9bdd686182fd38ca033b7b85cd5e971f4dabcb0714817e",
        ),
        (
            "ignored",
            "351690a5b4901c62b9ca6f6bb9b6d0d34d6ed6b11661fa49501a82944dc0de32",
            "1de0f0e886b00313cd6a20c5baeae498c014d4d52d79026ec6b785afe68590d7",
        ),
        (
            "shape",
            "f1bb017ecb20035d94f0a434c5a88e1c25cc5c01212f8d91c4632ac1860ce000",
            "53e6debd9d996f450ca9f2b82ab545f93fb13981e535c384d55796c169f0f9dd",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        edit(&mut g, mode);
        d.apply(d.revision(), g).unwrap();
        hash(&mut d, None, wide);
        hash(&mut d, Some(160.), narrow)
    }
    let mut d = document();
    for (name, a, b) in [
        (
            "addStyleClass",
            "hot",
            "fill:#00f,color:white,font-weight:bold",
        ),
        ("addStyle2Node", "b", "stroke:red,stroke-width:3"),
        ("setCssClass", "b,c", "hot"),
    ] {
        let a = string_arg(&mut d, a);
        let b = string_arg(&mut d, b);
        run(&mut d, name, vec![a, b]).result.unwrap();
    }
    hash(
        &mut d,
        None,
        "39fcbb41ee2b52096f03d7bd91d8c1dac13f867de9104c7621ec6cba1424c72f",
    );
    hash(
        &mut d,
        Some(160.),
        "071cf7a9a8728fc4a35fcd07a5cfd45d9bece582811043639ce4fe38bb05708d",
    );
}
fn builder() -> (PublicDocument, Atom, Atom) {
    let mut e = PublicEngine::empty();
    e.register(Block);
    let mut d = e
        .parse("block\ncolumns 2", &ParseOptions::default())
        .unwrap();
    for (name, a, b) in [
        ("setCssClass", "a, id-1", "hot"),
        ("addStyle2Node", "a", "stroke:blue,stroke-width:2"),
        ("addStyleClass", "hot", "fill:red,color:white"),
    ] {
        let a = string_arg(&mut d, a);
        let b = string_arg(&mut d, b);
        run(&mut d, name, vec![a, b]).result.unwrap();
    }
    let mut g = d.snapshot().unwrap();
    let mut record = |fields: &[(&str, &str)]| {
        let f = fields
            .iter()
            .map(|(k, v)| (*k, string(&mut g, v)))
            .collect();
        object(&mut g, f)
    };
    let a = record(&[("id", "a"), ("type", "square"), ("label", "Alpha")]);
    let b = record(&[("id", "b"), ("type", "circle"), ("label", "Beta")]);
    let group = record(&[("id", "g"), ("type", "composite"), ("label", "Builder")]);
    let col_type = string(&mut g, "column-setting");
    let col = object(
        &mut g,
        vec![("type", col_type), ("columns", Atom::Number(2.))],
    );
    let children = array(&mut g, vec![col.clone(), a.clone(), b]);
    set(&mut g, reference(&group), "children", children);
    let typ = string(&mut g, "space");
    let space = object(&mut g, vec![("type", typ), ("width", Atom::Number(2.))]);
    let pairs = [
        ("id", "a-b"),
        ("type", "edge"),
        ("start", "a"),
        ("end", "b"),
        ("label", "Built"),
        ("arrowTypeEnd", "arrow_point"),
    ];
    let fields = pairs.iter().map(|(k, v)| (*k, string(&mut g, v))).collect();
    let edge = object(&mut g, fields);
    let nested = array(&mut g, vec![group, space.clone()]);
    let statements = array(&mut g, vec![col, nested, edge]);
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "setHierarchy", vec![statements])
        .result
        .unwrap();
    (d, a, space)
}
#[test]
fn hierarchy_builder_preserves_pending_aliases_order_counters_and_limit_partial_state() {
    let (mut d, a, space) = builder();
    let g = d.snapshot().unwrap();
    assert_eq!(by_id(&g, "a"), reference(&a));
    let template = reference(&space);
    for id in ["id-1-0", "id-1-1"] {
        assert_eq!(
            reference(field(&g, template, "classes")),
            reference(field(&g, by_id(&g, id), "classes"))
        );
        assert_ne!(
            reference(field(&g, template, "children")),
            reference(field(&g, by_id(&g, id), "children"))
        );
    }
    assert_eq!(
        keys(&g, by_id(&g, "id-1")),
        ["type", "width", "id", "label", "classes", "styles", "children"]
    );
    assert_eq!(
        keys(&g, by_id(&g, "g")),
        ["id", "type", "label", "children", "colorIndex", "columns"]
    );
    assert_eq!(
        reference(field(&g, root(&g), "blocks")),
        reference(field(
            &g,
            reference(field(&g, root(&g), "root")),
            "children"
        ))
    );
    let Node::Set { values, .. } = &g.nodes[reference(field(&g, root(&g), "pending")) as usize]
    else {
        panic!()
    };
    assert!(values.is_empty());
    hash(
        &mut d,
        None,
        "207ac366129f602b932191eb63739809d8efee36fb0af07addfba29b605ad8f2",
    );
    hash(
        &mut d,
        Some(160.),
        "c185924136804a5a3b56eabba46488d03c013b055d511810a27cff1c9e954b48",
    );
    let mut g = d.snapshot().unwrap();
    let before = reference(field(
        &g,
        reference(field(&g, root(&g), "root")),
        "children",
    ));
    let z = string(&mut g, "z");
    let square = string(&mut g, "square");
    let label = string(&mut g, "Z");
    let first = object(&mut g, vec![("id", z), ("type", square), ("label", label)]);
    let id = string(&mut g, "too");
    let space = string(&mut g, "space");
    let second = object(
        &mut g,
        vec![
            ("id", id),
            ("type", space),
            ("width", Atom::Number(100001.)),
        ],
    );
    let args = array(&mut g, vec![first, second]);
    d.apply(d.revision(), g).unwrap();
    let out = run(&mut d, "setHierarchy", vec![args]);
    let Err(PublicError::Pele(e)) = out.result else {
        panic!()
    };
    assert_eq!(
        e.message,
        "A block diagram can hold at most 100000 space blocks."
    );
    assert_eq!(e.code, ErrorCode::Limit);
    assert_eq!(
        reference(field(
            &out.graph,
            reference(field(&out.graph, root(&out.graph), "root")),
            "children"
        )),
        before
    );
    by_id(&out.graph, "z");
    let too = by_id(&out.graph, "too");
    let Node::Array { length, .. } =
        out.graph.nodes[reference(field(&out.graph, too, "children")) as usize]
    else {
        panic!()
    };
    assert_eq!(length, 0);
    hash(
        &mut d,
        None,
        "207ac366129f602b932191eb63739809d8efee36fb0af07addfba29b605ad8f2",
    );
}
#[test]
fn methods_report_real_js_partial_errors_alias_returns_and_lossless_units() {
    let mut d = document();
    let x = string_arg(&mut d, "unknown");
    let out = run(&mut d, "named", vec![x.clone()]);
    let record = reference(&out.result.unwrap());
    let out = run(&mut d, "named", vec![x.clone()]);
    assert_eq!(reference(&out.result.unwrap()), record);
    let before = d.revision();
    let out = run(&mut d, "addStyle2Node", vec![x.clone(), Atom::Number(1.)]);
    let g = error(
        out,
        JavaScriptErrorKind::TypeError,
        "n.split is not a function",
    );
    assert!(d.revision() > before);
    assert_eq!(by_id(&g, "unknown"), record);
    let out = run(&mut d, "addStyleClass", vec![x.clone(), Atom::Number(1.)]);
    let g = error(
        out,
        JavaScriptErrorKind::TypeError,
        "n.split is not a function or its return value is not iterable",
    );
    let Node::Map { entries, .. } = &g.nodes[reference(field(&g, root(&g), "classes")) as usize]
    else {
        panic!()
    };
    assert_eq!(entries.len(), 2);
    error(
        run(&mut d, "setCssClass", vec![Atom::Number(1.)]),
        JavaScriptErrorKind::TypeError,
        "t.split is not a function or its return value is not iterable",
    );
    error(
        run(&mut d, "setHierarchy", vec![Atom::Null]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'flat')",
    );
    error(
        run(&mut d, "edgeStrToEdgeData", vec![Atom::Number(1.)]),
        JavaScriptErrorKind::TypeError,
        "e.trim is not a function",
    );
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 44, 0xdc00]));
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addStyle2Node", vec![x, Atom::String(raw)])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    let styles = reference(field(&g, record, "styles"));
    for (i, units) in [(0, vec![0xd800]), (1, vec![0xdc00])] {
        let Atom::String(s) = field(&g, styles, &i.to_string()) else {
            panic!()
        };
        assert_eq!(g.strings[*s as usize].0, units)
    }
}

struct FailingText;
impl RenderResources for FailingText {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("Block text stopped", ErrorCode::Semantic))
    }
}
#[test]
fn native_source_equivalence_lifetime_lossless_flat_graph_and_recovery() {
    let mut e = PublicEngine::empty();
    e.register(Block);
    let mut d = e.parse(SOURCE, &ParseOptions::default()).unwrap();
    e.register(Block);
    drop(e);
    let mut g = d.snapshot().unwrap();
    let user = by_id(&g, "a");
    text(&mut g, user, "label", "Native alpha");
    d.apply(0, g).unwrap();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let expected = Engine::with_builtins()
        .render_source(
            &SOURCE.replace("Alpha", "Native alpha"),
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
    assert_eq!(native.diagram_type.as_str(), "block");
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
        "Block text stopped"
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
fn active_cycle_surrogate_and_replaced_method_gaps_reject_then_repair() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let saved = g.clone();
    let group = by_id(&g, "g");
    let child = reference(field(&g, group, "children"));
    append(&mut g, child, Atom::Ref(group));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("cyclic"));
    d.apply(d.revision(), saved.clone()).unwrap();
    let mut g = saved.clone();
    let label = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    set(&mut g, by_id(&saved, "a"), "label", Atom::String(label));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("UTF-16"));
    d.apply(d.revision(), saved.clone()).unwrap();
    let mut g = saved;
    let r = root(&g);
    set(&mut g, r, "named", Atom::Null);
    let x = string(&mut g, "new");
    let styles = string(&mut g, "fill:red");
    d.apply(d.revision(), g).unwrap();
    error(
        run(&mut d, "addStyle2Node", vec![x.clone(), styles.clone()]),
        JavaScriptErrorKind::TypeError,
        "this.named is not a function",
    );
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "named", Atom::BuiltinFunction(9));
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addStyle2Node", vec![x, styles])
        .result
        .unwrap();
    d.render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
}
#[test]
fn parser_edge_direction_aliases_and_typed_factory_edits_remain_observable() {
    let mut e = PublicEngine::empty();
    e.register(Block);
    let d = e
        .parse("block\na-->b<[\"B\"]>(right)", &ParseOptions::default())
        .unwrap();
    let g = d.snapshot().unwrap();
    let edge = item(&g, "edges", 0);
    let b = by_id(&g, "b");
    assert_eq!(
        reference(field(&g, edge, "directions")),
        reference(field(&g, b, "directions"))
    );
    let config = Object::default();
    let mut model = Block
        .parse(ParseContext {
            source: "block\na-->b<[\"B\"]>(right)",
            config: &config,
            title: None,
            line_offset: 0,
            max_edges: 1000,
        })
        .unwrap();
    let b = model.by_id.iter().find(|(id, _)| id == "b").unwrap().1;
    model.nodes[b].directions = Some(vec!["x".into()]);
    model.nodes[b].label = Some("Typed".into());
    let mut d = PublicDocument::from_module(Block, &model, config.clone()).unwrap();
    let g = d.snapshot().unwrap();
    let edge = item(&g, "edges", 0);
    let b = by_id(&g, "b");
    assert_ne!(
        reference(field(&g, edge, "directions")),
        reference(field(&g, b, "directions"))
    );
    let direct = Block
        .render(
            &model,
            &config,
            &RenderSettings::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap()
            .rendered,
        direct
    );
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let map = reference(field(&g, r, "byId"));
    map_set(&mut g, map, "alias", Atom::Ref(b));
    d.apply(d.revision(), g).unwrap();
    let a = string_arg(&mut d, "alias");
    assert_eq!(reference(&run(&mut d, "named", vec![a]).result.unwrap()), b);
}
#[test]
fn inactive_array_properties_remain_data_while_native_flat_ignores_nested_flat() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let edges = reference(field(&g, r, "edges"));
    set(&mut g, edges, "map", Atom::Number(1.));
    let children = reference(field(&g, reference(field(&g, r, "root")), "children"));
    set(&mut g, children, "flat", Atom::Number(2.));
    let classes = reference(field(&g, by_id(&g, "a"), "classes"));
    set(&mut g, classes, "map", Atom::Number(3.));
    d.apply(d.revision(), g).unwrap();
    hash(
        &mut d,
        None,
        "351690a5b4901c62b9ca6f6bb9b6d0d34d6ed6b11661fa49501a82944dc0de32",
    );
    let mut e = PublicEngine::empty();
    e.register(Block);
    let mut d = e
        .parse("block\ncolumns 2", &ParseOptions::default())
        .unwrap();
    let mut g = d.snapshot().unwrap();
    let id = string(&mut g, "native");
    let typ = string(&mut g, "square");
    let b = object(&mut g, vec![("id", id), ("type", typ)]);
    let nested = array(&mut g, vec![b.clone()]);
    set(&mut g, reference(&nested), "flat", Atom::Null);
    let outer = array(&mut g, vec![nested]);
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "setHierarchy", vec![outer]).result.unwrap();
    assert_eq!(by_id(&d.snapshot().unwrap(), "native"), reference(&b));
    d.render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
}
#[test]
fn class_builder_preserves_writable_and_readonly_array_length_descriptors() {
    let mut d = document();
    let id = string_arg(&mut d, "a");
    let hot = string_arg(&mut d, "hot");
    let mut g = d.snapshot().unwrap();
    let classes = reference(field(&g, by_id(&g, "a"), "classes"));
    set(&mut g, classes, "length", Atom::Number(1.));
    g.nodes[classes as usize]
        .properties_mut()
        .last_mut()
        .unwrap()
        .flags = 2;
    d.apply(d.revision(), g).unwrap();
    let out = run(&mut d, "setCssClass", vec![id.clone(), hot.clone()]);
    out.result.unwrap();
    assert!(matches!(field(&out.graph,classes,"length"),Atom::Number(n)if *n==2.));
    let mut g = out.graph;
    let prop = g.nodes[classes as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is("length"))
        .unwrap();
    g.nodes[classes as usize].properties_mut()[prop].flags = 0;
    d.apply(d.revision(), g).unwrap();
    let out = run(&mut d, "setCssClass", vec![id.clone(), hot.clone()]);
    let g = error(
        out,
        JavaScriptErrorKind::TypeError,
        "Cannot assign to read only property 'length' of object '[object Array]'",
    );
    let Node::Array { length, .. } = g.nodes[classes as usize] else {
        panic!()
    };
    assert_eq!(length, 2);
    let mut g = g;
    g.nodes[classes as usize].properties_mut()[prop].flags = 2;
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "setCssClass", vec![id, hot]).result.unwrap();
    let g = d.snapshot().unwrap();
    assert!(matches!(field(&g,classes,"length"),Atom::Number(n)if *n==3.));
    d.render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
}
