//! Public MindmapDb feature slices, Pele 1.6.0 / 458322a5, MIT kepano.
//! Oracle: node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d;
//! upstream src/diagrams/mindmap/db.ts SHA256
//! 1ce7f21d91258f1b44b03f9a03341413dae3ff568883aa504082963fc6f23a08.
//! src/diagrams/common/outline.ts SHA256
//! 49bf03d677f1d3a5d7a87fb1082c3ac67ee625533696cb83f189dd8705e740cc.
//! Probes use public parse(), Object.getOwnPropertyDescriptors, borrowed prototype
//! methods and render(model). Constructors, exotic coercion, active sparse text
//! arrays and active lone UTF-16 rendering remain explicitly unsupported.
use pele::compat::{diagrams::mindmap::Mindmap, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = "mindmap\nRoot\n  A\n    B\n  C";
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
fn json(g: &ModelGraph) -> Json {
    serde_json::to_value(g).unwrap()
}
fn document() -> PublicDocument {
    let mut e = PublicEngine::empty();
    e.register(Mindmap);
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
fn keys(g: &ModelGraph, id: u32) -> Vec<String> {
    g.nodes[id as usize]
        .properties()
        .iter()
        .map(|p| g.strings[p.key as usize].well_formed().unwrap())
        .collect()
}
fn text(g: &mut ModelGraph, id: u32, key: &str, v: &str) {
    let a = string(g, v);
    set(g, id, key, a)
}
fn array(g: &ModelGraph, id: u32) -> Vec<Atom> {
    let Node::Array { length, .. } = g.nodes[id as usize] else {
        panic!("array")
    };
    (0..length)
        .map(|i| field(g, id, &i.to_string()).clone())
        .collect()
}
fn nodes(g: &ModelGraph) -> (u32, Vec<u32>) {
    let list = reference(g.field("nodes").unwrap().unwrap());
    (list, array(g, list).iter().map(reference).collect())
}
fn add(d: &mut PublicDocument, level: f64, id: &str, descr: &str, shape: f64) -> InvocationOutcome {
    let a = args(d, &[id, descr]);
    run(
        d,
        "addNode",
        vec![
            Atom::Number(level),
            a[0].clone(),
            a[1].clone(),
            Atom::Number(shape),
        ],
    )
}
fn decoration(d: &mut PublicDocument, icon: Option<&str>, class: Option<&str>) -> Atom {
    let mut g = d.snapshot().unwrap();
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (k, v) in [("icon", icon), ("class", class)] {
        if let Some(v) = v {
            text(&mut g, id, k, v)
        }
    }
    d.apply(d.revision(), g).unwrap();
    Atom::Ref(id)
}
#[test]
fn mindmap_actual_instance_own_function_node_fields_and_shared_open_children() {
    let d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let (_, n) = nodes(&g);
    assert_eq!(d.model_schema().id, "pele/mindmap@1");
    d.model_schema().validate().unwrap();
    assert_eq!(
        keys(&g, r),
        ["type", "title", "nodes", "getType", "baseLevel", "open"]
    );
    assert!(matches!(
        g.nodes[r as usize],
        Node::Instance {
            prototype: PrototypeId(1),
            ..
        }
    ));
    assert!(matches!(field(&g, r, "title"), Atom::Undefined));
    assert!(matches!(field(&g, r, "getType"), Atom::BuiltinFunction(0)));
    assert_eq!(
        d.model_schema()
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        [("getType", 2), ("addNode", 4), ("decorateNode", 1)]
    );
    assert_eq!(
        d.model_schema().prototypes[0]
            .members
            .iter()
            .map(|m| (m.name, m.flags))
            .collect::<Vec<_>>(),
        [("addNode", 6), ("decorateNode", 6)]
    );
    assert!(g
        .nodes
        .iter()
        .flat_map(Node::properties)
        .all(|p| p.flags == 7));
    assert_eq!(
        keys(&g, n[0]),
        ["id", "nodeId", "level", "descr", "type", "children", "isRoot"]
    );
    assert_eq!(
        keys(&g, n[1]),
        ["id", "nodeId", "level", "descr", "type", "children", "isRoot", "section"]
    );
    assert_eq!(
        array(&g, reference(field(&g, n[0], "children")))
            .iter()
            .map(reference)
            .collect::<Vec<_>>(),
        [n[1], n[3]]
    );
    assert_eq!(
        reference(field(&g, reference(field(&g, n[1], "children")), "0")),
        n[2]
    );
    assert_eq!(
        array(&g, reference(field(&g, r, "open")))
            .iter()
            .map(reference)
            .collect::<Vec<_>>(),
        [n[0], n[3]]
    );
    let mut e = PublicEngine::empty();
    e.register(Mindmap);
    let d = e
        .parse(
            "mindmap\nR\n:::root\n::icon(x)\n  C\n::icon(y)\n:::leaf",
            &ParseOptions::default(),
        )
        .unwrap();
    let g = d.snapshot().unwrap();
    let (_, n) = nodes(&g);
    assert_eq!(&keys(&g, n[0])[7..], ["class", "icon"]);
    assert_eq!(&keys(&g, n[1])[8..], ["icon", "class"]);
}
#[test]
fn mindmap_real_builder_decorator_and_get_type_match_pinned_svg_and_aliases() {
    let mut d = document();
    let before = d.snapshot().unwrap();
    let r = root(&before);
    let (list, original) = nodes(&before);
    let open = reference(field(&before, r, "open"));
    let mut g = before;
    text(&mut g, r, "title", "After");
    d.apply(d.revision(), g).unwrap();
    add(&mut d, 2., "d", "D", 6.).result.unwrap();
    let a = decoration(&mut d, Some("fa:fa-user"), Some("hot urgent"));
    run(&mut d, "decorateNode", vec![a]).result.unwrap();
    add(&mut d, 4., "leaf", "Leaf", 2.).result.unwrap();
    let g = d.snapshot().unwrap();
    let (newlist, n) = nodes(&g);
    assert_eq!(newlist, list);
    assert_eq!(&n[..4], original);
    assert_eq!(reference(field(&g, r, "open")), open);
    assert_eq!(
        array(&g, open).iter().map(reference).collect::<Vec<_>>(),
        [n[0], n[4], n[5]]
    );
    assert_eq!(
        reference(field(&g, reference(field(&g, n[4], "children")), "0")),
        n[5]
    );
    assert!(matches!(field(&g, n[5], "section"), Atom::Number(2.)));
    assert_eq!(&keys(&g, n[4])[8..], ["icon", "class"]);
    hash(
        &mut d,
        None,
        "e1b86edfffd19fa8a320c048bbbfd96b374fc525d3f985f07aa2aa84445bad28",
    );
    hash(
        &mut d,
        Some(190.),
        "f08eb425582c49726ebc49d42c3fd8b53f492b06edd85c7dd81f1f6e672356b6",
    );
    for (start, end, expected) in [
        ("[", "]", 2.),
        ("(", ")", 1.),
        ("(", "]", 4.),
        ("((", "]", 3.),
        (")", "]", 4.),
        ("))", "]", 5.),
        ("{{", "}}", 6.),
        ("?", "", 0.),
    ] {
        let a = args(&mut d, &[start, end]);
        assert!(
            matches!(call(&mut d,"getType",Atom::Null,a).result,Ok(Atom::Number(n))if n==expected)
        );
    }
}
#[test]
fn mindmap_direct_edits_reordering_aliases_and_inert_ieee_fields_match_pinned() {
    for (id, wide, narrow) in [
        (
            "direct",
            "b4705a6748f64227eb354a1c3cf93058dd96861e269eca8cbc8134f68f1400e0",
            "06421512812f1e3df2623854dc08a5e5d3ad57f626af1ee8517c6206515f45c0",
        ),
        (
            "reverseChildren",
            "c0cc76a8ea4cc700f427527a4c34f8fc390e106ea8ac6753dc797a31d9a8ccd6",
            "d91282f3d0ad4c8a4d0200913950aefe8dc72bc9866af09d54ae8ec1fd77d51d",
        ),
        (
            "sharedChildren",
            "441c19ee8438b2c83cca2f8fc2cfc6f36be07d5f67a0752aeebf94c7c6d005e1",
            "72e1b9598b38df79252fdb9ef4c3655084e18cf1e2951b2afc9d5f80b28372b5",
        ),
        (
            "duplicateChild",
            "de70a0551b80cf57372b31b7cbbea691abdfa472c8a0df20f8e03a2c0632f5e5",
            "840657f06fcb3ded79694bb9ad80cf04ad33ef307ec1d2db45dbb91daa2e7e0a",
        ),
        (
            "reorderAndRenumber",
            "fef674ea86240eb1a78ce5ef34bf7ad481464de7ca887fc067c4b42f509235c0",
            "394630f313f3ed17ab811cd538e50f491efde44a90cc4a6befd1b3e4f76cb7b3",
        ),
        (
            "levels",
            "137764275f7e4bd9e08f2afcc70fe2be01a6f575a4048bf86d9a9ef06fcbd985",
            "d91282f3d0ad4c8a4d0200913950aefe8dc72bc9866af09d54ae8ec1fd77d51d",
        ),
        (
            "nullClass",
            "137764275f7e4bd9e08f2afcc70fe2be01a6f575a4048bf86d9a9ef06fcbd985",
            "d91282f3d0ad4c8a4d0200913950aefe8dc72bc9866af09d54ae8ec1fd77d51d",
        ),
        (
            "inheritUndefinedSection",
            "eea5b1adbd00b4870a45928b5a31dcb8c20ce3697ed3e61738475b05ab8f1503",
            "cea21900daeb4e82b9ba39d9196421f6d3db68fe3232f06c88a923824af5c411",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let (list, n) = nodes(&g);
        let root_children = reference(field(&g, n[0], "children"));
        match id {
            "direct" => {
                text(&mut g, r, "title", "A & B");
                text(&mut g, r, "type", "fake");
                text(&mut g, n[1], "descr", "Edited **A**");
                set(&mut g, n[2], "type", Atom::Number(5.));
                text(&mut g, n[3], "class", "hot");
                text(&mut g, n[3], "nodeId", "<script>&");
            }
            "reverseChildren" => {
                set(&mut g, root_children, "0", Atom::Ref(n[3]));
                set(&mut g, root_children, "1", Atom::Ref(n[1]));
            }
            "sharedChildren" => {
                let empty = field(&g, n[3], "children").clone();
                set(&mut g, n[1], "children", empty);
            }
            "duplicateChild" => set(&mut g, root_children, "1", Atom::Ref(n[1])),
            "reorderAndRenumber" => {
                set(&mut g, list, "1", Atom::Ref(n[3]));
                set(&mut g, list, "3", Atom::Ref(n[1]));
                set(&mut g, n[3], "id", Atom::Number(1.));
                set(&mut g, n[1], "id", Atom::Number(3.));
            }
            "levels" => {
                set(&mut g, r, "baseLevel", Atom::Number(f64::NAN));
                for (i, v) in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN]
                    .into_iter()
                    .enumerate()
                {
                    set(&mut g, n[i], "level", Atom::Number(v));
                }
            }
            "nullClass" => set(&mut g, n[1], "class", Atom::Null),
            "inheritUndefinedSection" => set(&mut g, n[0], "isRoot", Atom::Bool(false)),
            _ => unreachable!(),
        }
        d.apply(d.revision(), g).unwrap();
        if id == "inheritUndefinedSection" {
            add(&mut d, 2., "d", "D", 0.).result.unwrap();
            let g = d.snapshot().unwrap();
            let (_, n) = nodes(&g);
            assert!(matches!(field(&g, n[4], "section"), Atom::Undefined));
        }
        hash(&mut d, None, wide);
        hash(&mut d, Some(190.), narrow);
    }
}
#[test]
fn mindmap_partial_builder_errors_publish_exact_open_parent_and_base_effects() {
    let mut d = document();
    let before = d.snapshot().unwrap();
    let r = root(&before);
    let (list, n) = nodes(&before);
    let open = reference(field(&before, r, "open"));
    let c_children = reference(field(&before, n[3], "children"));
    let mut g = before;
    set(&mut g, n[3], "children", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    let g = error(
        add(&mut d, 4., "bad", "Bad", 0.),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'push')",
    );
    assert_eq!(array(&g, open).len(), 2);
    assert_eq!(array(&g, list).len(), 4);
    let mut g = d.snapshot().unwrap();
    set(&mut g, n[3], "children", Atom::Ref(c_children));
    d.apply(d.revision(), g).unwrap();
    add(&mut d, 4., "fixed", "Fixed", 0.).result.unwrap();
    let before = d.snapshot().unwrap();
    let open_before = array(&before, open)
        .iter()
        .map(reference)
        .collect::<Vec<_>>();
    let rev = d.revision();
    let o = add(&mut d, 0., "root2", "Second root", 0.);
    assert!(
        matches!(o.result,Err(PublicError::Pele(PeleError{message,..}))if message=="There can be only one root. No parent could be found for (\"Second root\")")
    );
    assert_eq!(array(&o.graph, open).len(), 3);
    assert_eq!(array(&o.graph, list).len(), 5);
    assert_eq!(o.graph.nodes.len(), before.nodes.len());
    let snapshot = json(&d.snapshot().unwrap());
    assert!(d
        .invoke(
            rev,
            MethodCall {
                method: MethodId(1),
                receiver: o.graph.root.clone(),
                arguments: vec![]
            },
            &mut MetricsResources
        )
        .is_err());
    assert_eq!(json(&d.snapshot().unwrap()), snapshot);
    assert_eq!(
        array(&o.graph, open)
            .iter()
            .map(reference)
            .collect::<Vec<_>>(),
        open_before
    );
    // In actual JS nodes=3 fails only at the final push: parent.children and open
    // already contain the new record, whose id is explicitly undefined.
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (_, n) = nodes(&g);
    let open = reference(field(&g, r, "open"));
    set(&mut g, r, "nodes", Atom::Number(3.));
    d.apply(d.revision(), g).unwrap();
    let g = error(
        add(&mut d, 4., "new", "New", 0.),
        JavaScriptErrorKind::TypeError,
        "this.nodes.push is not a function",
    );
    let last = reference(field(&g, open, "2"));
    assert!(matches!(field(&g, last, "id"), Atom::Undefined));
    assert_eq!(
        reference(field(&g, reference(field(&g, n[3], "children")), "0")),
        last
    );
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (list, _) = nodes(&g);
    g.nodes[list as usize] = Node::Array {
        length: 0,
        properties: vec![],
    };
    set(&mut g, r, "open", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    let g = error(
        add(&mut d, 5., "new", "New", 0.),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'length')",
    );
    assert!(matches!(field(&g, r, "baseLevel"), Atom::Number(5.)));
    assert_eq!(array(&g, list).len(), 0);
}
#[test]
fn mindmap_borrowed_methods_and_ieee_indentation_match_pinned() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (list, n) = nodes(&g);
    let open = reference(field(&g, r, "open"));
    let borrowed = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (k, a) in [
        ("nodes", Atom::Ref(list)),
        ("open", Atom::Ref(open)),
        ("baseLevel", Atom::Number(1.)),
    ] {
        set(&mut g, borrowed, k, a)
    }
    set(&mut g, r, "borrowed", Atom::Ref(borrowed));
    d.apply(d.revision(), g).unwrap();
    let a = args(&mut d, &["new", "New"]);
    call(
        &mut d,
        "addNode",
        Atom::Ref(borrowed),
        vec![
            Atom::Number(4.),
            a[0].clone(),
            a[1].clone(),
            Atom::Number(0.),
        ],
    )
    .result
    .unwrap();
    let a = decoration(&mut d, Some("x"), Some("borrowed"));
    call(&mut d, "decorateNode", Atom::Ref(borrowed), vec![a])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    let (_, new) = nodes(&g);
    assert_eq!(&new[..4], n);
    assert!(matches!(field(&g, new[4], "level"), Atom::Number(3.)));
    assert!(matches!(field(&g, r, "baseLevel"), Atom::Number(0.)));
    hash(
        &mut d,
        None,
        "7de082d7c853272e6ee275dd9d9636d7fa739de70eb3a8de8724fecb8ef35b98",
    );
    for v in [f64::NAN, f64::INFINITY] {
        let mut d = document();
        add(&mut d, v, "numeric", "Numeric", 2.).result.unwrap();
        let g = d.snapshot().unwrap();
        let (_, n) = nodes(&g);
        let Atom::Number(level) = field(&g, n[4], "level") else {
            panic!("number")
        };
        assert_eq!(level.to_bits(), v.to_bits());
        hash(
            &mut d,
            None,
            "1dc4810005023152b31dc7f4bf10ebaef800b2567b709624e5848f426f204706",
        );
    }
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    text(&mut g, r, "baseLevel", "0x2");
    d.apply(d.revision(), g).unwrap();
    add(&mut d, 6., "numeric", "Numeric", 2.).result.unwrap();
    let g = d.snapshot().unwrap();
    assert!(matches!(
        field(&g, nodes(&g).1[4], "level"),
        Atom::Number(4.)
    ));
    hash(
        &mut d,
        None,
        "1dc4810005023152b31dc7f4bf10ebaef800b2567b709624e5848f426f204706",
    );
    // Decorator targets nodes[nodes.length-1], independently of the open chain.
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let (list, n) = nodes(&g);
    set(&mut g, list, "1", Atom::Ref(n[3]));
    set(&mut g, list, "3", Atom::Ref(n[1]));
    set(&mut g, n[3], "id", Atom::Number(1.));
    set(&mut g, n[1], "id", Atom::Number(3.));
    d.apply(d.revision(), g).unwrap();
    let a = decoration(&mut d, None, Some("last-array"));
    run(&mut d, "decorateNode", vec![a]).result.unwrap();
    let g = d.snapshot().unwrap();
    assert!(g.text(field(&g, n[1], "class")).unwrap().is("last-array"));
    assert!(!keys(&g, n[3]).contains(&"class".into()));
}
#[test]
fn mindmap_explicit_runtime_shape_gaps_keep_graph_and_recover() {
    let mut d = document();
    let expected = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut g = d.snapshot().unwrap();
    let (list, n) = nodes(&g);
    let children = reference(field(&g, n[1], "children"));
    let detached = g.nodes.len() as u32;
    g.nodes.push(g.nodes[n[2] as usize].clone());
    set(&mut g, children, "0", Atom::Ref(detached));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("detached child records"));
    let g = d.snapshot().unwrap();
    assert_eq!(reference(field(&g, children, "0")), detached);
    let mut g = g;
    set(&mut g, children, "0", Atom::Ref(n[2]));
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let mut g = d.snapshot().unwrap();
    set(&mut g, n[2], "id", Atom::Number(-1.));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("non-index child IDs"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, n[2], "id", Atom::Number(2.));
    set(&mut g, n[1], "section", Atom::Number(-1.));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("negative"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, n[1], "section", Atom::Number(0.));
    let keep = field(&g, list, "2").clone();
    let key = g.nodes[list as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is("2"))
        .unwrap();
    g.nodes[list as usize].properties_mut().remove(key);
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("sparse"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, list, "2", keep);
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    // Pinned addNode(4,'raw','\ud800A\udc00',0) retains [55296,65,56320]
    // and the new object is shared by nodes, C.children and open.
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    d.apply(d.revision(), g).unwrap();
    let a = args(&mut d, &["raw"]);
    let outcome = run(
        &mut d,
        "addNode",
        vec![
            Atom::Number(4.),
            a[0].clone(),
            Atom::String(raw),
            Atom::Number(0.),
        ],
    );
    outcome.result.unwrap();
    let g = outcome.graph;
    let (_, all) = nodes(&g);
    assert_eq!(
        g.text(field(&g, all[4], "descr")).unwrap().0,
        [0xd800, 65, 0xdc00]
    );
    assert_eq!(
        reference(field(&g, reference(field(&g, all[3], "children")), "0")),
        all[4]
    );
    assert_eq!(
        reference(field(&g, reference(field(&g, root(&g), "open")), "2")),
        all[4]
    );
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let mut g = d.snapshot().unwrap();
    text(&mut g, all[4], "descr", "Recovered");
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    // Actual pinned String-indexing errors are known, not silently approximated:
    // nodes='abc'; decorateNode({class:'x'}) throws for primitive 'c'.
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    text(&mut g, r, "nodes", "abc");
    d.apply(d.revision(), g).unwrap();
    let a = decoration(&mut d, None, Some("x"));
    assert!(
        matches!(run(&mut d,"decorateNode",vec![a]).result,Err(PublicError::Pele(PeleError{message,..}))if message.contains("String collection indexing"))
    );
}

struct Failure;
impl RenderResources for Failure {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("mindmap host failed", ErrorCode::Semantic))
    }
}
#[test]
fn mindmap_native_render_deep_extras_utf16_and_error_recovery() {
    let mut d = document();
    let expected = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let task = nodes(&g).1[1];
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
        "bits",
        Atom::Number(f64::from_bits(0x7ff8000000000042)),
    );
    let lone = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    set(&mut g, last, "lone", Atom::String(lone));
    g.nodes[last as usize].properties_mut()[0].flags = 4;
    d.apply(d.revision(), g).unwrap();
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
        "mindmap host failed"
    );
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(1);
    assert_eq!(
        d.render(&options, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(json(&d.snapshot().unwrap()), saved);
    let a = args(&mut d, &["A"]);
    let mut g = d.snapshot().unwrap();
    set(&mut g, task, "descr", Atom::String(lone));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, task, "descr", a[0].clone());
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let mut g = d.snapshot().unwrap();
    text(&mut g, task, "descr", "Native **edited**");
    text(&mut g, r, "title", "Native board");
    d.apply(d.revision(), g).unwrap();
    let edited = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    assert!(edited.rendered.svg.contains("Native board"));
    let mut native = RenderOptions::default();
    native.render.native_theme = Some(NativeTheme::default());
    let output = d.render(&native, &mut MetricsResources).unwrap();
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
            _ => panic!("Mindmap baseline uses only records and arrays"),
        },
        _ => panic!("Mindmap baseline has no host references"),
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
        if self.host["allIcons"] == true {
            return Ok(Some(TrustedSvg::from_trusted_markup(
                "<path d=\"M0,0\"/>".into(),
            )));
        }
        Ok(self.host["icons"][name]
            .as_str()
            .map(|s| TrustedSvg::from_trusted_markup(s.into())))
    }
}
#[test]
fn mutable_mindmap_preserves_entire_existing_pinned_render_and_error_corpus() {
    let fixture: Json = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/mindmap.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut e = PublicEngine::empty();
    e.register(Mindmap);
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
fn mindmap_flat_mutable_graph_transports_all_three_previously_projection_limited_deep_cases() {
    let fixture: Json = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/mindmap.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut e = PublicEngine::empty();
    e.register(Mindmap);
    for c in fixture["typedCases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(c["options"].clone()).unwrap();
        let mut d = e
            .parse(c["source"].as_str().unwrap(), &options.parse)
            .unwrap();
        let graph = d.snapshot().unwrap();
        let (list, n) = nodes(&graph);
        let count = c["expected"]["nodeCount"].as_u64().unwrap() as usize;
        assert_eq!(n.len(), count);
        assert_eq!(array(&graph, list).len(), count);
        assert!(matches!(field(&graph,n[count-1],"level"),Atom::Number(v)if *v==(count-1)as f64));
        let encoded = serde_json::to_vec(&graph).unwrap();
        let decoded: ModelGraph = serde_json::from_slice(&encoded).unwrap();
        d.apply(d.revision(), decoded).unwrap();
        let mut result =
            serde_json::to_value(d.render(&options, &mut MetricsResources).unwrap()).unwrap();
        result["svgSha256"] = Json::String(format!(
            "{:x}",
            Sha256::digest(result["svg"].as_str().unwrap().as_bytes())
        ));
        result.as_object_mut().unwrap().remove("svg");
        assert_eq!(result, c["expected"]["result"], "{}", c["id"]);
    }
}
