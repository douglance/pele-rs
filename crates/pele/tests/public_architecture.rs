//! Public ArchitectureDb feature slices, Pele 1.6.0 / 458322a5, MIT kepano.
//! Oracle: node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d;
//! upstream src/diagrams/architecture/db.ts SHA256
//! 95cac198fc4ed1be82140e16c2ce38b599c08d52524741cfa5474c80af84ee1e.
//! Probes use public parse(), Object.getOwnPropertyDescriptors, borrowed prototype
//! methods and render(model). Constructors, exotic coercion, active sparse text
//! arrays and active lone UTF-16 rendering remain explicitly unsupported.
use pele::compat::{diagrams::architecture::Architecture, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = "architecture-beta\ngroup g(cloud)[Group]\nservice a(server)[Alpha] in g\nservice b(database)[Beta]\na:R --> L:b";
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
    e.register(Architecture);
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
fn ids(g: &ModelGraph) -> (u32, u32, u32, u32, u32, u32) {
    let r = root(g);
    let nodes = reference(field(g, r, "nodes"));
    let groups = reference(field(g, r, "groups"));
    let edges = reference(field(g, r, "edges"));
    (
        nodes,
        groups,
        edges,
        reference(map(g, nodes, "a")),
        reference(map(g, nodes, "b")),
        reference(map(g, groups, "g")),
    )
}
fn array(g: &ModelGraph, id: u32) -> Vec<Atom> {
    let Node::Array { length, .. } = g.nodes[id as usize] else {
        panic!("array")
    };
    (0..length)
        .map(|i| field(g, id, &i.to_string()).clone())
        .collect()
}
fn object(d: &mut PublicDocument, values: &[(&str, Atom)]) -> Atom {
    let mut g = d.snapshot().unwrap();
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (k, v) in values {
        set(&mut g, id, k, v.clone())
    }
    d.apply(d.revision(), g).unwrap();
    Atom::Ref(id)
}
fn string_fields(d: &mut PublicDocument, values: &[(&str, &str)]) -> Atom {
    let texts = values.iter().map(|(_, v)| *v).collect::<Vec<_>>();
    let atoms = args(d, &texts);
    object(
        d,
        &values
            .iter()
            .zip(atoms)
            .map(|((k, _), v)| (*k, v))
            .collect::<Vec<_>>(),
    )
}
fn hint(d: &mut PublicDocument, direction: &str, members: &[&str]) -> Atom {
    let a = args(d, &[direction]);
    let mut g = d.snapshot().unwrap();
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Array {
        length: members.len() as u32,
        properties: vec![],
    });
    for (i, s) in members.iter().enumerate() {
        text(&mut g, id, &i.to_string(), s)
    }
    d.apply(d.revision(), g).unwrap();
    object(
        d,
        &[("direction", a[0].clone()), ("members", Atom::Ref(id))],
    )
}
fn pele_error(o: InvocationOutcome, message: &str) -> ModelGraph {
    match o.result {
        Err(PublicError::Pele(e)) => {
            assert_eq!(e.code, ErrorCode::Semantic);
            assert_eq!(e.message, message);
            assert_eq!(e.diagram_type.as_deref(), Some("architecture"));
        }
        v => panic!("{v:?}"),
    };
    o.graph
}
#[test]
fn architecture_real_root_maps_records_undefined_fields_and_prototype() {
    let d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let (nodes, groups, edges, a, b, group) = ids(&g);
    assert_eq!(d.model_schema().id, "pele/architecture@1");
    d.model_schema().validate().unwrap();
    assert!(matches!(
        g.nodes[r as usize],
        Node::Instance {
            prototype: PrototypeId(1),
            ..
        }
    ));
    assert_eq!(
        keys(&g, r),
        [
            "type",
            "title",
            "accTitle",
            "accDescr",
            "nodes",
            "groups",
            "edges",
            "layoutHints",
            "registered"
        ]
    );
    for k in ["title", "accTitle", "accDescr"] {
        assert!(matches!(field(&g, r, k), Atom::Undefined))
    }
    for id in [nodes, groups, reference(field(&g, r, "registered"))] {
        assert!(matches!(g.nodes[id as usize], Node::Map { .. }))
    }
    for id in [a, b] {
        assert_eq!(
            keys(&g, id),
            ["id", "type", "icon", "iconText", "title", "in"]
        );
    }
    assert_eq!(keys(&g, group), ["id", "icon", "title", "in"]);
    let edge = reference(field(&g, edges, "0"));
    assert_eq!(
        keys(&g, edge),
        [
            "lhsId", "lhsDir", "lhsInto", "lhsGroup", "rhsId", "rhsDir", "rhsInto", "rhsGroup",
            "title"
        ]
    );
    assert!(matches!(field(&g, edge, "lhsInto"), Atom::Bool(false)));
    assert!(matches!(field(&g, edge, "rhsInto"), Atom::Bool(true)));
    assert!(matches!(field(&g, edge, "title"), Atom::Undefined));
    assert!(g
        .nodes
        .iter()
        .flat_map(Node::properties)
        .all(|p| p.flags == 7));
    assert_eq!(
        d.model_schema()
            .methods
            .iter()
            .map(|m| (m.name, m.arity))
            .collect::<Vec<_>>(),
        [
            ("register", 3),
            ("addService", 1),
            ("addJunction", 1),
            ("addGroup", 1),
            ("addEdge", 1),
            ("addLayoutHint", 1)
        ]
    );
    assert!(d.model_schema().prototypes[0]
        .members
        .iter()
        .all(|m| m.flags == 6));
    let reg = reference(field(&g, r, "registered"));
    assert_eq!(
        entries(&g, reg)
            .iter()
            .map(|(k, v)| (
                g.text(k).unwrap().well_formed().unwrap(),
                g.text(v).unwrap().well_formed().unwrap()
            ))
            .collect::<Vec<_>>(),
        [
            ("g".into(), "group".into()),
            ("a".into(), "node".into()),
            ("b".into(), "node".into())
        ]
    );
}
#[test]
fn architecture_all_six_actual_methods_retain_arrays_maps_and_layout_argument_alias() {
    let mut d = document();
    let before = d.snapshot().unwrap();
    let r = root(&before);
    let (nodes, groups, edges, _, _, _) = ids(&before);
    let mut g = before;
    for (k, v) in [
        ("title", "After"),
        ("accTitle", "Accessible"),
        ("accDescr", "Line\nTwo"),
    ] {
        text(&mut g, r, k, v)
    }
    d.apply(d.revision(), g).unwrap();
    for (name, fields) in [
        (
            "addGroup",
            vec![("id", "h"), ("title", "Inner"), ("in", "g")],
        ),
        (
            "addService",
            vec![
                ("id", "c"),
                ("icon", "server"),
                ("iconText", "C"),
                ("title", "Gamma"),
                ("in", "h"),
            ],
        ),
        ("addJunction", vec![("id", "j"), ("in", "h")]),
    ] {
        let input = string_fields(&mut d, &fields);
        run(&mut d, name, vec![input.clone()]).result.unwrap();
        let g = d.snapshot().unwrap();
        let record = map(
            &g,
            if name == "addGroup" { groups } else { nodes },
            fields[0].1,
        );
        assert_ne!(reference(record), reference(&input));
    }
    let a = args(&mut d, &["c", "R", "j", "L", "New"]);
    let edge = object(
        &mut d,
        &[
            ("lhsId", a[0].clone()),
            ("lhsDir", a[1].clone()),
            ("lhsInto", Atom::Bool(true)),
            ("rhsId", a[2].clone()),
            ("rhsDir", a[3].clone()),
            ("rhsInto", Atom::Bool(true)),
            ("title", a[4].clone()),
        ],
    );
    run(&mut d, "addEdge", vec![edge]).result.unwrap();
    let h = hint(&mut d, "row", &["a", "c", "j"]);
    let mut g = d.snapshot().unwrap();
    let hi = reference(&h);
    text(&mut g, hi, "extra", "retained");
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addLayoutHint", vec![h.clone()])
        .result
        .unwrap();
    let a = args(&mut d, &["service", "ghost"]);
    run(&mut d, "register", a).result.unwrap();
    let g = d.snapshot().unwrap();
    assert_eq!(reference(field(&g, r, "nodes")), nodes);
    assert_eq!(reference(field(&g, r, "groups")), groups);
    assert_eq!(reference(field(&g, r, "edges")), edges);
    assert_eq!(
        reference(field(&g, reference(field(&g, r, "layoutHints")), "0")),
        hi
    );
    assert!(!entries(&g, nodes)
        .iter()
        .any(|(k, _)| g.text(k).is_ok_and(|s| s.is("ghost"))));
    let j = reference(map(&g, nodes, "j"));
    assert_eq!(keys(&g, j), ["id", "type", "in"]);
    let e = reference(field(&g, edges, "1"));
    assert!(matches!(field(&g, e, "lhsGroup"), Atom::Undefined));
    assert!(matches!(field(&g, e, "rhsGroup"), Atom::Undefined));
    hash(
        &mut d,
        None,
        "a0a96c5ed5cc271f0375b436c43909064813965f9cc5b130bd1b35a01609299b",
    );
    hash(
        &mut d,
        Some(190.),
        "829a70724671d35ac7b7eae98e27492af7ad1124dfde1d5cff6a2717a586ad89",
    );
}
#[test]
fn architecture_mutable_map_values_order_aliases_metadata_flags_and_hint_alias_match_pinned() {
    for (id, wide, narrow) in [
        (
            "direct",
            "50bffd8ebdebc6f4e9004a3ca1295b70efe50be02f91352a90a4afd160ef712b",
            "06259a9523c1a2a7306fe29750f338e12d35b5d223fbee09f8871a77e4b5b47a",
        ),
        (
            "aliasRecords",
            "51f5308c582f80f6ada152b82bf1b909158a96b8bfb8c73f3461249a9b0772cf",
            "6e19f346be94f28465404f9c602692b18687a4314bd60b4d6b5996198c07c5a1",
        ),
        (
            "mapReorder",
            "59de0701d35fcc9045228a9c9ea1bc0f7ec73e5aeb1338cbe93c97c8f54b142f",
            "893ccb1d020bd7ccdbdca8549ffa3f1b0bc4c891a88e31d4cda2bb5fb2a203e8",
        ),
        (
            "mapKey",
            "59de0701d35fcc9045228a9c9ea1bc0f7ec73e5aeb1338cbe93c97c8f54b142f",
            "893ccb1d020bd7ccdbdca8549ffa3f1b0bc4c891a88e31d4cda2bb5fb2a203e8",
        ),
        (
            "layoutAlias",
            "1b41c1134a77c7ec026c0ca73c41d582aa7c695bf581149c96392d7f4c133485",
            "1b41c1134a77c7ec026c0ca73c41d582aa7c695bf581149c96392d7f4c133485",
        ),
        (
            "flags",
            "bff17e63b887d74227648da9fa4072f66ce039a347d90f0b66ee3538eca96ced",
            "4092b24be485eb4afd8b8777de77e284421e6a522ff84b3f950de1cea71dbda5",
        ),
        (
            "nullAccessibility",
            "b6ec8a00c69c6f4f321712c1c757f0a607735cd7107adb1c7d488eb8137fe34e",
            "64ff8f67023539a6cee76e7012eec689e1d4acd1bfeb9374d73fb443c5a3a0f2",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let (nodes, _, edges, a, b, group) = ids(&g);
        let edge = reference(field(&g, edges, "0"));
        match id {
            "direct" => {
                for (k, v) in [
                    ("title", "A & B"),
                    ("accTitle", "Accessible"),
                    ("accDescr", "Details"),
                    ("type", "fake"),
                ] {
                    text(&mut g, r, k, v)
                }
                text(&mut g, a, "title", "Edited **Alpha**");
                text(&mut g, b, "iconText", "B");
                set(&mut g, b, "icon", Atom::Undefined);
                text(&mut g, group, "title", "Edited Group");
                text(&mut g, edge, "title", "Edited <b>edge</b>");
            }
            "aliasRecords" => {
                let key = string(&mut g, "copy");
                if let Node::Map { entries, .. } = &mut g.nodes[nodes as usize] {
                    entries.push((key, Atom::Ref(a)))
                }
                text(&mut g, a, "title", "Shared");
            }
            "mapReorder" | "mapKey" => {
                let key = if id == "mapKey" {
                    Some(string(&mut g, "renamed"))
                } else {
                    None
                };
                if let Node::Map { entries, .. } = &mut g.nodes[nodes as usize] {
                    let mut old = entries.remove(0);
                    if let Some(key) = key {
                        old.0 = key;
                    }
                    entries.push(old);
                }
            }
            "layoutAlias" => {
                let h = hint(&mut d, "row", &["a", "b"]);
                run(&mut d, "addLayoutHint", vec![h.clone()])
                    .result
                    .unwrap();
                g = d.snapshot().unwrap();
                let id = reference(&h);
                text(&mut g, id, "direction", "column");
                let members = reference(field(&g, id, "members"));
                let zero = field(&g, members, "0").clone();
                let one = field(&g, members, "1").clone();
                set(&mut g, members, "0", one);
                set(&mut g, members, "1", zero);
            }
            "flags" => {
                set(&mut g, edge, "lhsInto", Atom::Bool(true));
                set(&mut g, edge, "rhsInto", Atom::Undefined);
            }
            "nullAccessibility" => {
                set(&mut g, r, "accTitle", Atom::Null);
                set(&mut g, r, "accDescr", Atom::Bool(false));
            }
            _ => unreachable!(),
        }
        d.apply(d.revision(), g).unwrap();
        hash(&mut d, None, wide);
        hash(&mut d, Some(190.), narrow);
    }
}
#[test]
fn architecture_exact_validation_and_destructuring_errors_follow_pinned_order() {
    for(fields,message)in[(vec![("id","a"),("in","missing")],"The service id [a] is already in use by another node"),(vec![("id","x"),("in","x")],"The service [x] cannot be placed within itself"),(vec![("id","x"),("in","missing")],"The service [x]'s parent does not exist. Please make sure the parent is created before this service"),(vec![("id","x"),("in","a")],"The service [x]'s parent is not a group")]{let mut d=document();let a=string_fields(&mut d,&fields);pele_error(run(&mut d,"addService",vec![a]),message);}
    for(fields,message)in[(vec![("lhsId","a"),("lhsDir","R"),("rhsId","missing"),("rhsDir","L")],"The right-hand id [missing] does not yet exist. Please create the service/group before declaring an edge to it."),(vec![("lhsId","g"),("lhsDir","R"),("rhsId","b"),("rhsDir","L")],"The left-hand id [g] is a group. Add {group} to a service inside it instead.")]{let mut d=document();let a=string_fields(&mut d,&fields);pele_error(run(&mut d,"addEdge",vec![a]),message);}
    let mut d = document();
    let c = string_fields(&mut d, &[("id", "c"), ("in", "g")]);
    run(&mut d, "addService", vec![c]).result.unwrap();
    let a = args(&mut d, &["a", "c", "R", "L"]);
    let edge = object(
        &mut d,
        &[
            ("lhsId", a[0].clone()),
            ("rhsId", a[1].clone()),
            ("lhsDir", a[2].clone()),
            ("rhsDir", a[3].clone()),
            ("lhsGroup", Atom::Bool(true)),
        ],
    );
    pele_error(run(&mut d,"addEdge",vec![edge]),"The left-hand id [a] is modified to traverse the group boundary, but the edge does not pass through two groups.");
    for (members, message) in [
        (
            vec!["a"],
            "An align directive requires at least two members; got 1",
        ),
        (
            vec!["a", "g"],
            "align row references [g], which is not a service or junction",
        ),
        (vec!["a", "a"], "align row lists [a] more than once"),
    ] {
        let a = hint(&mut d, "row", &members);
        pele_error(run(&mut d, "addLayoutHint", vec![a]), message);
    }
    for name in ["addService", "addJunction", "addGroup"] {
        error(
            run(&mut d, name, vec![Atom::Null]),
            JavaScriptErrorKind::TypeError,
            "Cannot destructure property 'id' of 'object null' as it is null.",
        );
        error(
            run(&mut d, name, vec![]),
            JavaScriptErrorKind::TypeError,
            "Cannot destructure property 'id' of 'undefined' as it is undefined.",
        );
    }
    error(
        run(&mut d, "addEdge", vec![Atom::Null]),
        JavaScriptErrorKind::TypeError,
        "Cannot destructure property 'lhsId' of 'object null' as it is null.",
    );
    error(
        run(&mut d, "addLayoutHint", vec![Atom::Null]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'members')",
    );
}
#[test]
fn architecture_partial_registration_late_push_errors_revisions_and_recovery() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (nodes, _, edges, _, _, _) = ids(&g);
    let registered = reference(field(&g, r, "registered"));
    set(&mut g, r, "nodes", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    let a = string_fields(&mut d, &[("id", "new"), ("title", "New")]);
    let rev = d.revision();
    let g = error(
        run(&mut d, "addService", vec![a.clone()]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'set')",
    );
    assert_eq!(d.revision(), rev + 1);
    assert!(g.text(map(&g, registered, "new")).unwrap().is("node"));
    assert_eq!(entries(&g, nodes).len(), 2);
    let before = json(&g);
    assert!(d
        .invoke(
            rev,
            MethodCall {
                method: MethodId(1),
                receiver: g.root.clone(),
                arguments: vec![a.clone()]
            },
            &mut MetricsResources
        )
        .is_err());
    assert_eq!(json(&d.snapshot().unwrap()), before);
    let mut g = g;
    set(&mut g, r, "nodes", Atom::Ref(nodes));
    d.apply(d.revision(), g).unwrap();
    pele_error(
        run(&mut d, "addService", vec![a.clone()]),
        "The service id [new] is already in use by another node",
    );
    let mut g = d.snapshot().unwrap();
    let keep = entries(&g, registered)
        .iter()
        .filter(|(k, _)| !g.text(k).unwrap().is("new"))
        .cloned()
        .collect();
    if let Node::Map { entries, .. } = &mut g.nodes[registered as usize] {
        *entries = keep;
    }
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addService", vec![a]).result.unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "edges", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    let a = string_fields(
        &mut d,
        &[
            ("lhsId", "a"),
            ("lhsDir", "R"),
            ("rhsId", "b"),
            ("rhsDir", "L"),
        ],
    );
    error(
        run(&mut d, "addEdge", vec![a.clone()]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'push')",
    );
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "edges", Atom::Ref(edges));
    d.apply(d.revision(), g).unwrap();
    run(&mut d, "addEdge", vec![a]).result.unwrap();
    // registered is bookkeeping: invalidating it does not block valid rendering.
    let expected = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "registered", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let a = string_fields(&mut d, &[("id", "next")]);
    error(
        run(&mut d, "addService", vec![a]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'has')",
    );
}
#[test]
fn architecture_borrowed_prototype_register_dispatch_and_same_value_zero_keys() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (nodes, groups, edges, _, _, _) = ids(&g);
    let reg = reference(field(&g, r, "registered"));
    let borrowed = g.nodes.len() as u32;
    g.nodes.push(Node::Instance {
        prototype: PrototypeId(1),
        properties: vec![],
    });
    for (k, a) in [
        ("nodes", Atom::Ref(nodes)),
        ("groups", Atom::Ref(groups)),
        ("edges", Atom::Ref(edges)),
        ("registered", Atom::Ref(reg)),
    ] {
        set(&mut g, borrowed, k, a)
    }
    set(&mut g, r, "borrowed", Atom::Ref(borrowed));
    d.apply(d.revision(), g).unwrap();
    let a = string_fields(&mut d, &[("id", "borrowed"), ("title", "Borrowed")]);
    call(&mut d, "addService", Atom::Ref(borrowed), vec![a.clone()])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    assert!(g.text(map(&g, reg, "borrowed")).unwrap().is("node"));
    assert!(g
        .text(field(&g, reference(map(&g, nodes, "borrowed")), "title"))
        .unwrap()
        .is("Borrowed"));
    let plain = object(
        &mut d,
        &[("nodes", Atom::Ref(nodes)), ("registered", Atom::Ref(reg))],
    );
    let a = string_fields(&mut d, &[("id", "plain")]);
    error(
        call(&mut d, "addService", plain.clone(), vec![a.clone()]),
        JavaScriptErrorKind::TypeError,
        "this.register is not a function",
    );
    let mut g = d.snapshot().unwrap();
    set(
        &mut g,
        reference(&plain),
        "register",
        Atom::BuiltinFunction(0),
    );
    d.apply(d.revision(), g).unwrap();
    call(&mut d, "addService", plain, vec![a]).result.unwrap();
    let kind = args(&mut d, &["service"])[0].clone();
    run(&mut d, "register", vec![kind.clone(), Atom::Number(-0.)])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    let key = &entries(&g, reg).last().unwrap().0;
    assert!(matches!(key,Atom::Number(n)if n.to_bits()==0));
    pele_error(
        run(&mut d, "register", vec![kind.clone(), Atom::Number(0.)]),
        "The service id [0] is already in use by another node",
    );
    run(
        &mut d,
        "register",
        vec![kind.clone(), Atom::Number(f64::NAN)],
    )
    .result
    .unwrap();
    pele_error(
        run(&mut d, "register", vec![kind, Atom::Number(f64::NAN)]),
        "The service id [NaN] is already in use by another node",
    );
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
}

struct Failure;
impl RenderResources for Failure {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new(
            "architecture host failed",
            ErrorCode::Semantic,
        ))
    }
}
#[test]
fn architecture_native_render_deep_extras_utf16_and_error_recovery() {
    let mut d = document();
    let expected = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let task = ids(&g).3;
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
        "architecture host failed"
    );
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(1);
    assert_eq!(
        d.render(&options, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(json(&d.snapshot().unwrap()), saved);
    let a = args(&mut d, &["Alpha"]);
    let mut g = d.snapshot().unwrap();
    set(&mut g, task, "title", Atom::String(lone));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, task, "title", a[0].clone());
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let mut g = d.snapshot().unwrap();
    text(&mut g, task, "title", "Native **edited**");
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
            _ => panic!("Architecture baseline uses only records, maps and arrays"),
        },
        _ => panic!("Architecture baseline has no host references"),
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
fn mutable_architecture_preserves_entire_existing_pinned_render_and_error_corpus() {
    let fixture: Json = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/architecture.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut e = PublicEngine::empty();
    e.register(Architecture);
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
fn architecture_opaque_utf16_map_keys_inactive_fields_and_hint_iterables_match_pinned() {
    for mode in [
        "opaqueKey",
        "utf16Key",
        "inactiveIconText",
        "junctionExtras",
        "stringMembers",
        "setMembers",
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let (nodes, _, _, a, _, _) = ids(&g);
        match mode {
            "opaqueKey" => {
                if let Node::Map { entries, .. } = &mut g.nodes[nodes as usize] {
                    entries[0].0 = Atom::Number(7.)
                }
                d.apply(d.revision(), g).unwrap();
            }
            "utf16Key" => {
                let raw = g.strings.len() as u32;
                g.strings.push(JsString(vec![0xd800]));
                if let Node::Map { entries, .. } = &mut g.nodes[nodes as usize] {
                    entries[0].0 = Atom::String(raw)
                }
                d.apply(d.revision(), g).unwrap();
                let kind = args(&mut d, &["service"]);
                run(&mut d, "register", vec![kind[0].clone(), Atom::String(raw)])
                    .result
                    .unwrap();
                let g = d.snapshot().unwrap();
                let reg = reference(field(&g, r, "registered"));
                assert_eq!(
                    g.text(&entries(&g, reg).last().unwrap().0).unwrap().0,
                    [0xd800]
                );
            }
            "inactiveIconText" => {
                set(&mut g, a, "iconText", Atom::Null);
                d.apply(d.revision(), g).unwrap();
            }
            "junctionExtras" => {
                let a = string_fields(&mut d, &[("id", "j")]);
                run(&mut d, "addJunction", vec![a]).result.unwrap();
                let mut g = d.snapshot().unwrap();
                let j = reference(map(&g, nodes, "j"));
                set(&mut g, j, "icon", Atom::Null);
                set(&mut g, j, "iconText", Atom::Bool(true));
                set(&mut g, j, "title", Atom::Number(7.));
                d.apply(d.revision(), g).unwrap();
            }
            "stringMembers" | "setMembers" => {
                let members = if mode == "stringMembers" {
                    args(&mut d, &["ab"])[0].clone()
                } else {
                    let a = args(&mut d, &["a", "b"]);
                    let mut g = d.snapshot().unwrap();
                    let id = g.nodes.len() as u32;
                    g.nodes.push(Node::Set {
                        values: a,
                        properties: vec![],
                    });
                    d.apply(d.revision(), g).unwrap();
                    Atom::Ref(id)
                };
                let direction = args(&mut d, &["row"])[0].clone();
                let h = object(&mut d, &[("direction", direction), ("members", members)]);
                run(&mut d, "addLayoutHint", vec![h]).result.unwrap();
            }
            _ => unreachable!(),
        }
        hash(
            &mut d,
            None,
            if mode == "junctionExtras" {
                "37a2221a22f76f4a8f96b8888b92fb70849e8d1e56d8b91df356f745e176269b"
            } else {
                "b6ec8a00c69c6f4f321712c1c757f0a607735cd7107adb1c7d488eb8137fe34e"
            },
        );
    }
}
#[test]
fn architecture_explicit_callback_cycle_and_work_limits_preserve_graph_and_recover() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (_, _, edges, _, _, group) = ids(&g);
    let registered = reference(field(&g, r, "registered"));
    set(&mut g, registered, "has", Atom::Number(3.));
    d.apply(d.revision(), g).unwrap();
    let a = args(&mut d, &["service", "x"]);
    error(
        run(&mut d, "register", a),
        JavaScriptErrorKind::TypeError,
        "this.registered.has is not a function",
    );
    let mut g = d.snapshot().unwrap();
    g.nodes[registered as usize].properties_mut().clear();
    set(&mut g, edges, "push", Atom::Number(3.));
    d.apply(d.revision(), g).unwrap();
    let a = string_fields(
        &mut d,
        &[
            ("lhsId", "a"),
            ("lhsDir", "R"),
            ("rhsId", "b"),
            ("rhsDir", "L"),
        ],
    );
    error(
        run(&mut d, "addEdge", vec![a]),
        JavaScriptErrorKind::TypeError,
        "this.edges.push is not a function",
    );
    let g = d.snapshot().unwrap();
    assert_eq!(array(&g, edges).len(), 1);
    let mut g = g;
    let keep = g.nodes[edges as usize]
        .properties()
        .iter()
        .filter(|p| !g.strings[p.key as usize].is("push"))
        .cloned()
        .collect();
    *g.nodes[edges as usize].properties_mut() = keep;
    set(&mut g, r, "register", Atom::HostReference(91));
    d.apply(d.revision(), g).unwrap();
    let a = string_fields(&mut d, &[("id", "callback")]);
    assert!(
        matches!(run(&mut d,"addService",vec![a]).result,Err(PublicError::Pele(PeleError{message,..}))if message.contains("overridden register callbacks"))
    );
    let mut g = d.snapshot().unwrap();
    let keep = g.nodes[r as usize]
        .properties()
        .iter()
        .filter(|p| !g.strings[p.key as usize].is("register"))
        .cloned()
        .collect();
    *g.nodes[r as usize].properties_mut() = keep;
    d.apply(d.revision(), g).unwrap();
    let a = string_fields(&mut d, &[("id", "h"), ("in", "g")]);
    run(&mut d, "addGroup", vec![a]).result.unwrap();
    let mut g = d.snapshot().unwrap();
    text(&mut g, group, "in", "h");
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("cyclic group ancestry"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, group, "in", Atom::Undefined);
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    // Explicit native safety ceiling; this test makes no pinned unsafe-layout claim.
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let (nodes, _, _, a, _, _) = ids(&g);
    if let Node::Map { entries, .. } = &mut g.nodes[nodes as usize] {
        *entries = (0..6000)
            .map(|i| (Atom::Number(i as f64), Atom::Ref(a)))
            .collect()
    }
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(entries(&d.snapshot().unwrap(), nodes).len(), 6000);
}
