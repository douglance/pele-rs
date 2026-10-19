//! Public JourneyDb feature slices, Pele 1.6.0 / 458322a5, MIT kepano.
//! Oracle: node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d;
//! upstream src/diagrams/journey/db.ts SHA256
//! 635df08a15bf032bf957fdb613269cddb6cbc29b152cc19c80077779e70f4960.
//! Probes use public parse(), Object.getOwnPropertyDescriptors, borrowed prototype
//! methods and render(model). Constructors, exotic coercion, active sparse text
//! arrays and active lone UTF-16 rendering remain explicitly unsupported.
use pele::compat::{diagrams::journey::Journey, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = "journey\nsection Start\nOld:4:Bob";
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
fn array(g: &ModelGraph, id: u32) -> Vec<Atom> {
    let Node::Array { length, .. } = g.nodes[id as usize] else {
        panic!("array")
    };
    (0..length)
        .map(|i| field(g, id, &i.to_string()).clone())
        .collect()
}
fn texts(g: &ModelGraph, id: u32) -> Vec<String> {
    array(g, id)
        .iter()
        .map(|a| g.text(a).unwrap().well_formed().unwrap())
        .collect()
}
fn json(g: &ModelGraph) -> Json {
    serde_json::to_value(g).unwrap()
}
fn document() -> PublicDocument {
    let mut e = PublicEngine::empty();
    e.register(Journey);
    e.parse(SOURCE, &ParseOptions::default()).unwrap()
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
fn journey_actual_instance_fields_prototype_methods_and_task_identity() {
    let d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let schema = d.model_schema();
    assert_eq!(schema.id, "pele/journey@1");
    schema.validate().unwrap();
    assert!(matches!(
        g.nodes[r as usize],
        Node::Instance {
            prototype: PrototypeId(1),
            ..
        }
    ));
    assert_eq!(
        g.nodes[r as usize]
            .properties()
            .iter()
            .map(|p| g.strings[p.key as usize].well_formed().unwrap())
            .collect::<Vec<_>>(),
        [
            "type",
            "title",
            "accTitle",
            "accDescr",
            "sections",
            "tasks",
            "currentSection"
        ]
    );
    for key in ["title", "accTitle", "accDescr"] {
        assert!(matches!(field(&g, r, key), Atom::Undefined));
    }
    assert!(g
        .nodes
        .iter()
        .flat_map(Node::properties)
        .all(|p| p.flags == 7));
    assert_eq!(
        schema
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        [
            ("setDiagramTitle", 1),
            ("setAccTitle", 1),
            ("setAccDescription", 1),
            ("addSection", 1),
            ("addTask", 2),
            ("getActors", 0)
        ]
    );
    assert!(schema.prototypes[0].members.iter().all(|m| m.flags == 6));
    assert!(matches!(
        schema.prototypes[0].parent,
        PrototypeParent::Object
    ));
    let tasks = reference(field(&g, r, "tasks"));
    let t = reference(field(&g, tasks, "0"));
    assert_eq!(
        g.nodes[t as usize]
            .properties()
            .iter()
            .map(|p| g.strings[p.key as usize].well_formed().unwrap())
            .collect::<Vec<_>>(),
        ["task", "score", "people", "section"]
    );
    assert_eq!(texts(&g, reference(field(&g, t, "people"))), ["Bob"]);
}
#[test]
fn journey_real_builders_mutate_retained_arrays_and_render_exact_pinned_svg() {
    // Oracle sequence: parse(SOURCE); setDiagramTitle('After');
    // setAccTitle('  Accessible'); setAccDescription('First\n  Second');
    // addSection('Next'); addTask('New',':2: Alice, Bob, Alice'); getActors().
    let mut d = document();
    let before = d.snapshot().unwrap();
    let r = root(&before);
    let tasks = reference(field(&before, r, "tasks"));
    let original = reference(field(&before, tasks, "0"));
    let sections = reference(field(&before, r, "sections"));
    let a = args(
        &mut d,
        &[
            "After",
            "  Accessible",
            "First\n  Second",
            "Next",
            "New",
            ":2: Alice, Bob, Alice",
        ],
    );
    for (name, arguments) in [
        ("setDiagramTitle", vec![a[0].clone()]),
        ("setAccTitle", vec![a[1].clone()]),
        ("setAccDescription", vec![a[2].clone()]),
        ("addSection", vec![a[3].clone()]),
        ("addTask", vec![a[4].clone(), a[5].clone()]),
    ] {
        assert!(matches!(
            run(&mut d, name, arguments).result,
            Ok(Atom::Undefined)
        ));
    }
    let o = run(&mut d, "getActors", vec![]);
    let actors = reference(o.result.as_ref().unwrap());
    assert_eq!(texts(&o.graph, actors), ["Alice", "Bob"]);
    let again = run(&mut d, "getActors", vec![]);
    assert_ne!(actors, reference(again.result.as_ref().unwrap()));
    let g = again.graph;
    assert_eq!(r, root(&g));
    assert_eq!(reference(field(&g, r, "tasks")), tasks);
    assert_eq!(reference(field(&g, tasks, "0")), original);
    assert_eq!(reference(field(&g, r, "sections")), sections);
    assert_eq!(texts(&g, sections), ["Start", "Next"]);
    let new = reference(field(&g, tasks, "1"));
    assert_eq!(
        texts(&g, reference(field(&g, new, "people"))),
        ["Alice", "Bob", "Alice"]
    );
    assert!(g
        .text(field(&g, r, "accDescr"))
        .unwrap()
        .is("First\nSecond"));
    hash(
        &mut d,
        None,
        "19ce41669a581f4c36d5f67904e8af7e51f254d5a6c5845cee0cde1479a1d7d3",
    );
    hash(
        &mut d,
        Some(190.),
        "3aba6a359c9c43f9f2c52515295a8f50b204428cb5907865a2ea5be3cf28f239",
    );
    let mut e = Engine::empty();
    e.register(Journey);
    let source="journey\ntitle After\naccTitle: Accessible\naccDescr { First\nSecond }\nsection Start\nOld:4:Bob\nsection Next\nNew:2:Alice, Bob, Alice";
    let expected = e
        .render_source(source, &RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
}
#[test]
fn journey_partial_method_errors_publish_mutations_and_stale_calls_do_not_run() {
    let mut d = document();
    let a = args(&mut d, &["Broken", "New", ":2: Alice"]);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let sections = field(&g, r, "sections").clone();
    let tasks = reference(field(&g, r, "tasks"));
    set(&mut g, r, "sections", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    let rev = d.revision();
    let g = error(
        run(&mut d, "addSection", vec![a[0].clone()]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'push')",
    );
    assert_eq!(d.revision(), rev + 1);
    assert!(g.text(field(&g, r, "currentSection")).unwrap().is("Broken"));
    assert_eq!(array(&g, tasks).len(), 1);
    // Even failed bookkeeping remains renderable: renderer never reads sections.
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    let before = json(&g);
    let stale = d
        .invoke(
            rev,
            MethodCall {
                method: MethodId(3),
                receiver: g.root.clone(),
                arguments: vec![],
            },
            &mut MetricsResources,
        )
        .unwrap_err();
    assert_eq!(stale.message, "Stale public model revision");
    assert_eq!(json(&d.snapshot().unwrap()), before);
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "sections", sections);
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addTask", vec![a[1].clone(), a[2].clone()])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    let task = reference(field(&g, tasks, "1"));
    assert!(g.text(field(&g, task, "section")).unwrap().is("Broken"));
    for (name, arg, message) in [
        (
            "setAccTitle",
            Atom::Null,
            "Cannot read properties of null (reading 'replace')",
        ),
        (
            "setAccDescription",
            Atom::Number(3.),
            "t.replace is not a function",
        ),
    ] {
        error(
            run(&mut d, name, vec![arg]),
            JavaScriptErrorKind::TypeError,
            message,
        );
    }
    error(
        run(&mut d, "addTask", vec![a[1].clone(), Atom::Number(3.)]),
        JavaScriptErrorKind::TypeError,
        "n.substring is not a function",
    );
    let mut g = d.snapshot().unwrap();
    let section_array = reference(field(&g, r, "sections"));
    g.nodes[section_array as usize] = Node::Array {
        length: u32::MAX,
        properties: vec![],
    };
    d.apply(d.revision(), g).unwrap();
    let g = error(
        run(&mut d, "addSection", vec![a[0].clone()]),
        JavaScriptErrorKind::RangeError,
        "Invalid array length",
    );
    assert!(matches!(
        g.nodes[section_array as usize],
        Node::Array {
            length: u32::MAX,
            ..
        }
    ));
    assert!(g
        .text(field(&g, section_array, "4294967295"))
        .unwrap()
        .is("Broken"));
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "tasks", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    error(
        run(&mut d, "getActors", vec![]),
        JavaScriptErrorKind::TypeError,
        "e is not iterable",
    );
    error(
        run(&mut d, "addTask", vec![a[1].clone(), a[2].clone()]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'push')",
    );
    let mut g = d.snapshot().unwrap();
    let excessive = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: 1_000_001,
        properties: vec![],
    });
    set(&mut g, r, "tasks", Atom::Ref(excessive));
    d.apply(d.revision(), g).unwrap();
    let outcome = run(&mut d, "getActors", vec![]);
    assert!(matches!(
        outcome.result,
        Err(PublicError::Pele(PeleError {
            code: ErrorCode::Limit,
            ..
        }))
    ));
}
#[test]
fn journey_borrowed_receiver_shares_collections_without_changing_root_section() {
    // Pinned: p.addSection.call({sections:m.sections,tasks:m.tasks,currentSection:
    // 'Borrowed'},'Away'); p.addTask.call(receiver,'Shared',':3: Alice').
    let mut d = document();
    let a = args(&mut d, &["Borrowed", "Away", "Shared", ":3: Alice"]);
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let borrowed = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (key, v) in [
        ("sections", field(&g, r, "sections").clone()),
        ("tasks", field(&g, r, "tasks").clone()),
        ("currentSection", a[0].clone()),
    ] {
        set(&mut g, borrowed, key, v)
    }
    set(&mut g, r, "borrowed", Atom::Ref(borrowed));
    d.apply(d.revision(), g).unwrap();
    call(
        &mut d,
        "addSection",
        Atom::Ref(borrowed),
        vec![a[1].clone()],
    )
    .result
    .unwrap();
    call(
        &mut d,
        "addTask",
        Atom::Ref(borrowed),
        vec![a[2].clone(), a[3].clone()],
    )
    .result
    .unwrap();
    let g = d.snapshot().unwrap();
    assert!(g.text(field(&g, r, "currentSection")).unwrap().is("Start"));
    assert!(g
        .text(field(&g, borrowed, "currentSection"))
        .unwrap()
        .is("Away"));
    hash(
        &mut d,
        None,
        "671470c1fc6e530b7d99bfde9d04e1dff43002db634de14dbb8a1137c79ee586",
    );
    hash(
        &mut d,
        Some(190.),
        "c26a6f35f2a253df91144ec386131283aff68814657195ab9224150c07a22cd3",
    );
    error(
        call(&mut d, "setDiagramTitle", Atom::Null, vec![]),
        JavaScriptErrorKind::TypeError,
        "Cannot set properties of null (setting 'title')",
    );
    error(
        call(&mut d, "setAccTitle", Atom::Null, vec![Atom::Null]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'replace')",
    );
}
#[test]
fn journey_add_task_number_conversion_and_actor_set_sort_match_pinned_runtime() {
    for (score, hash_value) in [
        (
            "-0",
            "a5f0b50a14e77198a638b4efaa1f13f9730dab8f307538df692dabc4c9559d81",
        ),
        (
            "NaN",
            "643fad73898028f84eda9fac11c9d7f0f5e7b5ea02606caf2fcca07b7c12f893",
        ),
        (
            "Infinity",
            "4a910402bc637426755e14b493b8804711a03f6993ce53013fe661568af2c72e",
        ),
        (
            "-Infinity",
            "a5f0b50a14e77198a638b4efaa1f13f9730dab8f307538df692dabc4c9559d81",
        ),
        (
            "0xFF",
            "4a910402bc637426755e14b493b8804711a03f6993ce53013fe661568af2c72e",
        ),
        (
            "0b100",
            "44c33dd9192df9d75a1f86da5918d8ecc5715028f04c86d1476882add772160d",
        ),
        (
            "0o7",
            "4a910402bc637426755e14b493b8804711a03f6993ce53013fe661568af2c72e",
        ),
        (
            "1_000",
            "643fad73898028f84eda9fac11c9d7f0f5e7b5ea02606caf2fcca07b7c12f893",
        ),
    ] {
        let mut d = document();
        let a = args(&mut d, &["Added", &format!(":{score}: Alice")]);
        run(&mut d, "addTask", a).result.unwrap();
        hash(&mut d, None, hash_value);
        if score == "-0" {
            let g = d.snapshot().unwrap();
            let t = reference(field(
                &g,
                reference(g.field("tasks").unwrap().unwrap()),
                "1",
            ));
            let Atom::Number(n) = field(&g, t, "score") else {
                panic!("score")
            };
            assert_eq!(n.to_bits(), (-0f64).to_bits());
        }
    }
    // Pinned tasks=[{people:[null,7,NaN,undefined,-0,'A',NaN,0]}]
    // getActors() => [0,7,'A',NaN,null,undefined], fresh array each time.
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let tasks = reference(field(&g, r, "tasks"));
    let t = reference(field(&g, tasks, "0"));
    let people = reference(field(&g, t, "people"));
    let a = string(&mut g, "A");
    g.nodes[people as usize] = Node::Array {
        length: 8,
        properties: vec![],
    };
    for (i, a) in [
        Atom::Null,
        Atom::Number(7.),
        Atom::Number(f64::NAN),
        Atom::Undefined,
        Atom::Number(-0.),
        a,
        Atom::Number(f64::NAN),
        Atom::Number(0.),
    ]
    .into_iter()
    .enumerate()
    {
        set(&mut g, people, &i.to_string(), a)
    }
    d.apply(d.revision(), g).unwrap();
    let o = run(&mut d, "getActors", vec![]);
    let values = array(&o.graph, reference(o.result.as_ref().unwrap()));
    assert_eq!(values.len(), 6);
    assert!(matches!(&values[0],Atom::Number(n)if n.to_bits()==0));
    assert!(matches!(&values[1],Atom::Number(n)if *n==7.));
    assert!(o.graph.text(&values[2]).unwrap().is("A"));
    assert!(matches!(&values[3],Atom::Number(n)if n.is_nan()));
    assert!(matches!(values[4], Atom::Null));
    assert!(matches!(values[5], Atom::Undefined));
}
#[test]
fn journey_utf16_normalization_and_builtin_string_set_iteration_match_pinned() {
    // Actual pinned operation probes, code units printed with charCodeAt.
    for (input, title, description) in [
        (
            vec![10, 32, 9, 65, 10, 32, 10, 32, 66],
            vec![65, 10, 32, 10, 32, 66],
            vec![10, 65, 10, 66],
        ),
        (
            vec![13, 10, 8195, 65, 10, 10, 9, 66],
            vec![65, 10, 10, 9, 66],
            vec![13, 10, 65, 10, 66],
        ),
        (
            vec![0xd800, 10, 160, 0xdc00],
            vec![0xd800, 10, 160, 0xdc00],
            vec![0xd800, 10, 0xdc00],
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let id = g.strings.len() as u32;
        g.strings.push(JsString(input));
        d.apply(d.revision(), g).unwrap();
        run(&mut d, "setAccTitle", vec![Atom::String(id)])
            .result
            .unwrap();
        let o = run(&mut d, "setAccDescription", vec![Atom::String(id)]);
        o.result.unwrap();
        let g = o.graph;
        let r = root(&g);
        assert_eq!(g.text(field(&g, r, "accTitle")).unwrap().0, title);
        assert_eq!(g.text(field(&g, r, "accDescr")).unwrap().0, description);
    }
    // String iterator emits paired code points and unmatched units, not Rust replacements.
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let tasks = reference(field(&g, r, "tasks"));
    let task = reference(field(&g, tasks, "0"));
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![
        122, 0xd83d, 0xde00, 97, 0xd800, 0xd83d, 0xde00,
    ]));
    set(&mut g, task, "people", Atom::String(raw));
    d.apply(d.revision(), g).unwrap();
    let o = run(&mut d, "getActors", vec![]);
    let values = array(&o.graph, reference(o.result.as_ref().unwrap()));
    assert_eq!(
        values
            .iter()
            .map(|a| o.graph.text(a).unwrap().0.clone())
            .collect::<Vec<_>>(),
        [vec![97], vec![122], vec![0xd800], vec![0xd83d, 0xde00]]
    );
    let mut g = d.snapshot().unwrap();
    let z = string(&mut g, "z");
    let a = string(&mut g, "a");
    let people = g.nodes.len() as u32;
    g.nodes.push(Node::Set {
        values: vec![z, a],
        properties: vec![],
    });
    set(&mut g, task, "people", Atom::Ref(people));
    let collection = g.nodes.len() as u32;
    g.nodes.push(Node::Set {
        values: vec![Atom::Ref(task)],
        properties: vec![],
    });
    set(&mut g, r, "tasks", Atom::Ref(collection));
    d.apply(d.revision(), g).unwrap();
    let o = run(&mut d, "getActors", vec![]);
    assert_eq!(
        texts(&o.graph, reference(o.result.as_ref().unwrap())),
        ["a", "z"]
    );
    // Builtin iteration works; renderer still explicitly requires typed task arrays.
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("Arrays"));
}
struct Failure;
impl RenderResources for Failure {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("journey host failed", ErrorCode::Semantic))
    }
}
#[test]
fn journey_lossless_extras_utf16_method_results_and_render_recovery() {
    let mut d = document();
    let expected = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let fake_type = string(&mut g, "future-application-type");
    set(&mut g, r, "type", fake_type);
    let mut last = r;
    for _ in 0..5000 {
        let id = g.nodes.len() as u32;
        g.nodes.push(Node::Object { properties: vec![] });
        set(&mut g, last, "next", Atom::Ref(id));
        last = id;
    }
    set(&mut g, last, "next", Atom::Ref(r));
    set(
        &mut g,
        last,
        "nan",
        Atom::Number(f64::from_bits(0x7ff8000000000042)),
    );
    let extra = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    set(&mut g, last, "lone", Atom::String(extra));
    g.nodes[last as usize].properties_mut()[0].flags = 4;
    d.apply(0, g).unwrap();
    let saved = json(&d.snapshot().unwrap());
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    assert_eq!(
        d.render(&RenderOptions::default(), &mut Failure)
            .unwrap_err()
            .message,
        "journey host failed"
    );
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(1);
    assert_eq!(
        d.render(&options, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(json(&d.snapshot().unwrap()), saved);
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let mut native = RenderOptions::default();
    native.render.native_theme = Some(NativeTheme::default());
    let native_output = d.render(&native, &mut MetricsResources).unwrap();
    let mut raster = resvg::usvg::Options::default();
    raster
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec());
    raster.font_family = "Noto Sans".into();
    raster.fontdb_mut().set_sans_serif_family("Noto Sans");
    let tree = resvg::usvg::Tree::from_str(&native_output.rendered.svg, &raster).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(
        native_output.rendered.width as u32,
        native_output.rendered.height as u32,
    )
    .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 100).count() > 100);
    // Normalization never replaces lone code units. Rendering reports explicit gap.
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![32, 32, 0xd800]));
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "setAccTitle", vec![Atom::String(raw)])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    assert_eq!(g.text(field(&g, r, "accTitle")).unwrap().0, [0xd800]);
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let a = args(&mut d, &["Restored"]);
    run(&mut d, "setAccTitle", a).result.unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
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
fn mutable_journey_preserves_entire_existing_pinned_render_and_error_corpus() {
    let fixture: Json = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/journey.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut e = PublicEngine::empty();
    e.register(Journey);
    for c in fixture["cases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(c["options"].clone()).unwrap();
        let actual = e
            .parse(c["source"].as_str().unwrap(), &options.parse)
            .and_then(|mut d| {
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
