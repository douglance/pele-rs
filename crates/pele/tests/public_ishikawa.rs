//! Ishikawa public native feature slices, Pele 1.6.0 / 458322a5 (MIT kepano).
//! db.ts SHA256 bb75051c583c0fed54fa7bdc3d80b8a78b93e194fdb9cf1fff5268df866b0b1a.
//! Oracle node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
//! Probes: parse(SOURCE), descriptors, real prototype addNode.call, render(model).
//! Undefined leaf text is a known wide/narrow timing gap, never a fake passing SVG.
use pele::compat::{diagrams::ishikawa::Ishikawa, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = "ishikawa-beta\nEffect\n  People\n    Training\n  Process\n    Review";
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
    e.register(Ishikawa);
    e.parse(SOURCE, &ParseOptions::default()).unwrap()
}
fn child(g: &ModelGraph, parent: u32, index: u32) -> u32 {
    let children = reference(field(g, parent, "children"));
    reference(field(g, children, &index.to_string()))
}
fn effect(g: &ModelGraph) -> u32 {
    reference(field(g, root(g), "root"))
}
fn empty_array(g: &mut ModelGraph) -> u32 {
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: 0,
        properties: vec![],
    });
    id
}
fn args(d: &mut PublicDocument, values: &[&str]) -> Vec<Atom> {
    let mut g = d.snapshot().unwrap();
    let a = values.iter().map(|s| string(&mut g, s)).collect();
    d.apply(d.revision(), g).unwrap();
    a
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

#[test]
fn ishikawa_actual_prototype_fields_stack_and_tree_aliases() {
    let mut d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    assert_eq!(d.model_schema().id, "pele/ishikawa@1");
    assert_eq!(d.model_schema().methods.len(), 1);
    assert_eq!(d.model_schema().methods[0].arity, 2);
    assert_eq!(d.model_schema().prototypes[0].members[0].flags, 6);
    assert!(matches!(
        g.nodes[r as usize],
        Node::Instance {
            prototype: PrototypeId(1),
            ..
        }
    ));
    assert_eq!(keys(&g, r), ["type", "title", "root", "stack", "baseLevel"]);
    assert_eq!(keys(&g, effect(&g)), ["text", "children"]);
    assert!(matches!(field(&g, r, "title"), Atom::Undefined));
    assert!(matches!(field(&g,r,"baseLevel"),Atom::Number(n)if *n==2.));
    let stack = reference(field(&g, r, "stack"));
    let ancestors = [
        effect(&g),
        child(&g, effect(&g), 1),
        child(&g, child(&g, effect(&g), 1), 0),
    ];
    for (i, expected) in ancestors.into_iter().enumerate() {
        let row = reference(field(&g, stack, &i.to_string()));
        assert_eq!(keys(&g, row), ["level", "node"]);
        assert_eq!(reference(field(&g, row, "node")), expected);
    }
    assert!(g
        .nodes
        .iter()
        .flat_map(|n| n.properties())
        .all(|p| p.flags == 7));
    hash(
        &mut d,
        None,
        "7d873a578cb4254201bbf40f5e9a395bec6df9c169924c36977d6c1b8011781d",
    );
    hash(
        &mut d,
        Some(150.),
        "bfca20d24f6b8f446336628860ffa86cd38d7f94da8aa3232f898cc5e6b63b64",
    );
    let mut e = PublicEngine::empty();
    e.register(Ishikawa);
    let d = e
        .parse("ishikawa-beta\n\n", &ParseOptions::default())
        .unwrap();
    let g = d.snapshot().unwrap();
    assert!(matches!(field(&g, root(&g), "root"), Atom::Undefined));
    assert!(matches!(field(&g, root(&g), "baseLevel"), Atom::Undefined));
}
#[test]
fn ishikawa_add_node_alias_metadata_and_numeric_calls_match_pinned_svgs() {
    for (mode, wide, narrow) in [
        (
            "added",
            "28f1151189cd152e03bb3cdd2a6a30f511e4ee56b5dd6fd3a48ad711a0364ff9",
            "cacf2f41d14904039c32a12cf5b8e4f8e4d7ce399fcacdf5ee2bebd3e7f77256",
        ),
        (
            "alias",
            "1029f6a563990b03e211cd44b5b5f796b30c44a434d310a5e635975d7528c04f",
            "36112d4c12b8dd18ec5c9d490919912c1186f0a322164f89e46c8ed0ee069d78",
        ),
        (
            "metadata",
            "150415f757c25bf5fb977f114fcd8ecf712b91113214dc91d85903b765819d68",
            "1cf327083c3a26791d336f79b0637d8fbd9a49e9b4f5b9bd7a7cca870070053e",
        ),
        (
            "rawString",
            "5e7da1f5f7e62939773c1f54c0e00f5a4a17ab78e8325385dc68935efd416277",
            "3bd211cec202815a669cd2beb9933fb55e9bb08d32cbaa51b37503708170ae1f",
        ),
        (
            "rawUndefined",
            "c1f17b8210ba161f464b8ec97268e9d9be05bd1cf54bde773c625b64dea8da69",
            "2cabf0bb3664f08bb65601267778ce2018ff7aa4c0870a461f65c012da38660e",
        ),
        (
            "baseNull",
            "5d02034579e919e94e103bba118e8153f35da62611e4fe00b80ec12d7d88d5df",
            "0b18c8845af6c06c9ed9a90abab48322d57309712a3d93700b2b57b995015640",
        ),
        (
            "baseNaN",
            "0dcd1bef29011689c5f92543164ccbe070e9543852e5e96113fd956229cb009a",
            "4ac159a6ac05752e3d31f6c90d36b1de9d3fd5bdf006bdb5b389b344f8c574ab",
        ),
    ] {
        let mut d = document();
        match mode {
            "added" => {
                for (level, label) in [(4., "Tools"), (8., "Maintenance")] {
                    let a = args(&mut d, &[label]);
                    assert!(matches!(
                        run(&mut d, "addNode", vec![Atom::Number(level), a[0].clone()])
                            .result
                            .unwrap(),
                        Atom::Undefined
                    ));
                }
            }
            "alias" => {
                let mut g = d.snapshot().unwrap();
                let root = effect(&g);
                let a = child(&g, root, 0);
                text(&mut g, a, "text", "Shared");
                let children = reference(field(&g, root, "children"));
                set(&mut g, children, "2", Atom::Ref(a));
                if let Node::Array { length, .. } = &mut g.nodes[children as usize] {
                    *length = 3;
                }
                d.apply(d.revision(), g).unwrap();
            }
            "metadata" => {
                let mut g = d.snapshot().unwrap();
                let r = root(&g);
                for (k, v) in [
                    ("title", "Edited"),
                    ("accTitle", "Fishbone"),
                    ("accDescr", "Causes"),
                ] {
                    text(&mut g, r, k, v)
                }
                d.apply(d.revision(), g).unwrap();
            }
            _ => {
                let a = args(&mut d, &[if mode == "baseNaN" { "new" } else { "x" }]);
                let level = match mode {
                    "rawString" => args(&mut d, &["4"])[0].clone(),
                    "rawUndefined" => Atom::Undefined,
                    "baseNull" => {
                        let mut g = d.snapshot().unwrap();
                        let r = root(&g);
                        set(&mut g, r, "baseLevel", Atom::Null);
                        d.apply(d.revision(), g).unwrap();
                        Atom::Number(7.)
                    }
                    "baseNaN" => {
                        let mut g = d.snapshot().unwrap();
                        let r = root(&g);
                        set(&mut g, r, "baseLevel", Atom::Number(f64::NAN));
                        d.apply(d.revision(), g).unwrap();
                        Atom::Number(1.)
                    }
                    _ => unreachable!(),
                };
                run(&mut d, "addNode", vec![level, a[0].clone()])
                    .result
                    .unwrap();
            }
        }
        let graph = serde_json::to_value(d.snapshot().unwrap()).unwrap();
        hash(&mut d, None, wide);
        hash(&mut d, Some(150.), narrow);
        assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), graph);
    }
}
#[test]
fn ishikawa_partial_errors_publish_real_stack_mutations_and_repair() {
    for mode in [
        "nullThis",
        "emptyStack",
        "nullStack",
        "popFalse",
        "pushFalse",
        "childrenPushFalse",
        "poppedThenFail",
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let stack = reference(field(&g, r, "stack"));
        let effect = effect(&g);
        let review = child(&g, child(&g, effect, 1), 0);
        match mode {
            "emptyStack" => {
                g.nodes[stack as usize] = Node::Array {
                    length: 0,
                    properties: vec![],
                }
            }
            "nullStack" => set(&mut g, r, "stack", Atom::Null),
            "popFalse" => set(&mut g, stack, "pop", Atom::Bool(false)),
            "pushFalse" => set(&mut g, stack, "push", Atom::Bool(false)),
            "childrenPushFalse" => {
                let c = reference(field(&g, review, "children"));
                set(&mut g, c, "push", Atom::Bool(false));
            }
            "poppedThenFail" => set(&mut g, effect, "children", Atom::Null),
            _ => (),
        };
        d.apply(0, g).unwrap();
        let label = args(&mut d, &["x"])[0].clone();
        let receiver = if mode == "nullThis" {
            Atom::Null
        } else {
            d.snapshot().unwrap().root
        };
        let level = if mode == "childrenPushFalse" {
            8.
        } else if mode == "pushFalse" {
            4.
        } else {
            0.
        };
        let before = d.revision();
        let o = call(
            &mut d,
            "addNode",
            receiver,
            vec![Atom::Number(level), label],
        );
        assert_eq!(o.revision, before + 1);
        let expected = match mode {
            "nullThis" => "Cannot read properties of null (reading 'root')",
            "emptyStack" => "Cannot read properties of undefined (reading 'node')",
            "nullStack" => "Cannot read properties of null (reading 'length')",
            "popFalse" => "s.pop is not a function",
            "pushFalse" => "s.push is not a function",
            "childrenPushFalse" => "s[(s.length - 1)].node.children.push is not a function",
            _ => "Cannot read properties of null (reading 'push')",
        };
        let g = error(o, JavaScriptErrorKind::TypeError, expected);
        if mode == "poppedThenFail" {
            assert!(matches!(
                g.nodes[stack as usize],
                Node::Array { length: 1, .. }
            ));
        }
        if mode == "pushFalse" {
            assert!(matches!(
                g.nodes[stack as usize],
                Node::Array { length: 2, .. }
            ));
            let process = child(&g, effect, 1);
            let children = reference(field(&g, process, "children"));
            assert!(matches!(
                g.nodes[children as usize],
                Node::Array { length: 2, .. }
            ));
        }
        let original = document().snapshot().unwrap();
        d.apply(d.revision(), original).unwrap();
        hash(
            &mut d,
            None,
            "7d873a578cb4254201bbf40f5e9a395bec6df9c169924c36977d6c1b8011781d",
        );
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
            _ => panic!("Ishikawa baseline uses only records, maps and arrays"),
        },
        _ => panic!("Ishikawa baseline has no host references"),
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
fn mutable_ishikawa_preserves_entire_existing_pinned_render_and_error_corpus() {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/ishikawa.json"
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
    e.register(Ishikawa);

    for c in fixture["cases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(c["options"].clone()).unwrap();
        let actual = e
            .parse(c["source"].as_str().unwrap(), &options.parse)
            .and_then(|mut d| {
                if c["expected"].get("model").is_some() {
                    let mut g = d.snapshot().unwrap();
                    let r = root(&g);
                    let keep = g.nodes[r as usize]
                        .properties()
                        .iter()
                        .filter(|p| {
                            ["type", "title", "root"]
                                .iter()
                                .any(|k| g.strings[p.key as usize].is(k))
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    *g.nodes[r as usize].properties_mut() = keep;
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
fn ishikawa_borrowed_builder_root_reset_and_raw_utf16_success_are_observable() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let borrowed = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    set(&mut g, r, "borrowed", Atom::Ref(borrowed));
    d.apply(0, g).unwrap();
    let label = args(&mut d, &["First"])[0].clone();
    call(
        &mut d,
        "addNode",
        Atom::Ref(borrowed),
        vec![Atom::Number(9.), label],
    )
    .result
    .unwrap();
    let g = d.snapshot().unwrap();
    assert_eq!(keys(&g, borrowed), ["root", "stack"]);
    let old = reference(field(&g, borrowed, "root"));
    let stack = reference(field(&g, borrowed, "stack"));
    let row = reference(field(&g, stack, "0"));
    assert_eq!(reference(field(&g, row, "node")), old);
    let label = args(&mut d, &["Second"])[0].clone();
    call(
        &mut d,
        "addNode",
        Atom::Ref(borrowed),
        vec![Atom::Number(5.), label],
    )
    .result
    .unwrap();
    let g = d.snapshot().unwrap();
    assert!(matches!(field(&g,borrowed,"baseLevel"),Atom::Number(n)if *n==5.));
    assert_eq!(
        child(&g, old, 0),
        reference(field(&g, reference(field(&g, stack, "1")), "node"))
    );
    hash(
        &mut d,
        None,
        "7d873a578cb4254201bbf40f5e9a395bec6df9c169924c36977d6c1b8011781d",
    );
    // Falsy root resets stack; baseLevel is retained, exactly as actual addNode.
    let mut g = d.snapshot().unwrap();
    set(&mut g, borrowed, "root", Atom::Bool(false));
    d.apply(d.revision(), g).unwrap();
    let label = args(&mut d, &["Reset"])[0].clone();
    call(
        &mut d,
        "addNode",
        Atom::Ref(borrowed),
        vec![Atom::Number(99.), label],
    )
    .result
    .unwrap();
    let g = d.snapshot().unwrap();
    assert!(matches!(field(&g,borrowed,"baseLevel"),Atom::Number(n)if *n==5.));
    let fresh = reference(field(&g, borrowed, "stack"));
    assert_ne!(fresh, stack);
    assert!(matches!(
        g.nodes[fresh as usize],
        Node::Array { length: 1, .. }
    ));
    // Undefined method text and lone UTF16 remain stored; successful call can
    // create a model that cannot render through the current native text transport.
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    d.apply(d.revision(), g).unwrap();
    let outcome = run(&mut d, "addNode", vec![Atom::Number(4.), Atom::String(raw)]);
    outcome.result.unwrap();
    let g = outcome.graph;
    let stack = reference(field(&g, r, "stack"));
    let row = reference(field(&g, stack, "2"));
    let new = reference(field(&g, row, "node"));
    assert_eq!(g.text(field(&g, new, "text")).unwrap().0, [0xd800]);
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let mut g = d.snapshot().unwrap();
    text(&mut g, new, "text", "Repaired");
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
}
struct FailingHost;
impl RenderResources for FailingHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("Ishikawa host stopped", ErrorCode::Semantic))
    }
}
#[test]
fn ishikawa_source_equivalent_native_edits_metadata_resources_revisions_and_deep_extras() {
    let mut e = PublicEngine::empty();
    e.register(Ishikawa);
    let mut d = e.parse(SOURCE, &ParseOptions::default()).unwrap();
    e.register(Ishikawa);
    drop(e);
    for (level, label) in [(4., "Tools"), (8., "Maintenance")] {
        let a = args(&mut d, &[label]);
        run(&mut d, "addNode", vec![Atom::Number(level), a[0].clone()])
            .result
            .unwrap();
    }
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let expected = Engine::with_builtins()
        .render_source(
            &format!("{SOURCE}\n    Tools\n        Maintenance"),
            &options,
            &mut MetricsResources,
        )
        .unwrap();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    assert_eq!(d.render(&options, &mut MetricsResources).unwrap(), expected);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    text(&mut g, r, "accTitle", "Native fishbone");
    text(&mut g, r, "accDescr", "Edited causes");
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    set(&mut g, r, "applicationRaw", Atom::String(raw));
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
    assert!(!xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Native fishbone")));
    assert!(!xml
        .descendants()
        .any(|n| n.has_tag_name("desc") && n.text() == Some("Edited causes")));
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
        "Ishikawa host stopped"
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
    let revision = d.revision();
    assert!(d.apply(revision - 1, g.clone()).is_err());
    assert_eq!(d.revision(), revision);
    assert_eq!(serde_json::to_value(d.snapshot().unwrap()).unwrap(), saved);
    let result = d.invoke(
        revision - 1,
        MethodCall {
            method: MethodId(0),
            receiver: g.root,
            arguments: vec![],
        },
        &mut MetricsResources,
    );
    assert!(result.is_err());
    assert_eq!(d.revision(), revision);
}
#[test]
fn ishikawa_flat_deep_tree_round_trip_cyclic_and_expanded_alias_guards_recover() {
    let mut source = "ishikawa-beta\nEffect\n".to_string();
    for i in 0..1000 {
        source.push_str(&" ".repeat(i + 1));
        source.push_str(&format!("node{i}\n"));
    }
    let mut e = PublicEngine::empty();
    e.register(Ishikawa);
    let mut parse = ParseOptions::default();
    parse.preparation.limit = 1_000_000;
    let mut d = e.parse(&source, &parse).unwrap();
    let g = d.snapshot().unwrap();
    let bytes = serde_json::to_vec(&g).unwrap();
    d.apply(0, serde_json::from_slice(&bytes).unwrap()).unwrap();
    let initial = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut options = RenderOptions::default();
    options.parse = parse;
    let immutable = Engine::with_builtins()
        .render_source(&source, &options, &mut MetricsResources)
        .unwrap();
    assert_eq!(initial, immutable);
    let original = document().snapshot().unwrap();
    d.apply(d.revision(), original.clone()).unwrap();
    for mode in ["cycle", "expanded", "sparse", "work", "absentText"] {
        let mut g = original.clone();
        let r = root(&g);
        let effect = effect(&g);
        let children = reference(field(&g, effect, "children"));
        match mode {
            "cycle" => set(&mut g, children, "0", Atom::Ref(effect)),
            "expanded" => {
                let mut current = child(&g, effect, 0);
                for _ in 0..25 {
                    let c = empty_array(&mut g);
                    set(&mut g, c, "0", Atom::Ref(current));
                    set(&mut g, c, "1", Atom::Ref(current));
                    if let Node::Array { length, .. } = &mut g.nodes[c as usize] {
                        *length = 2;
                    }
                    let node = g.nodes.len() as u32;
                    g.nodes.push(Node::Object { properties: vec![] });
                    text(&mut g, node, "text", "Repeated");
                    set(&mut g, node, "children", Atom::Ref(c));
                    current = node;
                }
                set(&mut g, r, "root", Atom::Ref(current));
            }
            "sparse" => {
                g.nodes[children as usize].properties_mut().remove(0);
            }
            "work" => {
                g.nodes[children as usize] = Node::Array {
                    length: 1_000_001,
                    properties: vec![],
                }
            }
            "absentText" => set(&mut g, effect, "text", Atom::Undefined),
            _ => unreachable!(),
        };
        let saved = serde_json::to_value(&g).unwrap();
        d.apply(d.revision(), g).unwrap();
        let error = d
            .render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap_err();
        assert_eq!(
            error.code,
            if mode == "expanded" || mode == "work" {
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
            "7d873a578cb4254201bbf40f5e9a395bec6df9c169924c36977d6c1b8011781d",
        );
    }
}

#[test]
fn ishikawa_method_at_utf16_budget_keeps_a_valid_partial_graph_and_can_recover() {
    let mut d = document();
    let original = d.snapshot().unwrap();
    let mut g = original.clone();
    let r = root(&g);
    let label = field(&g, child(&g, child(&g, effect(&g), 1), 0), "text").clone();
    set(&mut g, r, "applicationBudget", Atom::Undefined);
    let units = g.strings.iter().map(|s| s.0.len()).sum::<usize>();
    let id = g.strings.len() as u32;
    g.strings.push(JsString(vec![65; 16_000_000 - units]));
    set(&mut g, r, "applicationBudget", Atom::String(id));
    d.apply(0, g).unwrap();
    let outcome = run(&mut d, "addNode", vec![Atom::Number(8.), label]);
    assert!(matches!(outcome.result,Err(PublicError::Pele(ref e))if e.code==ErrorCode::Limit));
    outcome.graph.validate().unwrap();
    assert_eq!(
        outcome
            .graph
            .strings
            .iter()
            .map(|s| s.0.len())
            .sum::<usize>(),
        16_000_000
    );
    d.apply(d.revision(), original).unwrap();
    hash(
        &mut d,
        None,
        "7d873a578cb4254201bbf40f5e9a395bec6df9c169924c36977d6c1b8011781d",
    );
}
