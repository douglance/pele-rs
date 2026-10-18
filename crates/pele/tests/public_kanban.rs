//! Public KanbanDb feature slices, Pele 1.6.0 / 458322a5, MIT kepano.
//! Oracle: node_modules/pele/dist/pele.js SHA256
//! d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d;
//! upstream src/diagrams/kanban/db.ts SHA256
//! 578c60eb05960cc0457c908ce1dc31e74c420224484fec450624e6633b66fc22.
//! src/diagrams/common/outline.ts SHA256
//! 49bf03d677f1d3a5d7a87fb1082c3ac67ee625533696cb83f189dd8705e740cc.
//! Probes use public parse(), Object.getOwnPropertyDescriptors, borrowed prototype
//! methods and render(model). Constructors, exotic coercion, active sparse text
//! arrays and active lone UTF-16 rendering remain explicitly unsupported.
use pele::compat::{diagrams::kanban::Kanban, public::*, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
const SOURCE: &str = "kanban\nTodo\n  task[Task]";
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
    e.register(Kanban);
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
fn ids(g: &ModelGraph) -> (u32, u32, u32, u32) {
    let r = root(g);
    let sections = reference(field(g, r, "sections"));
    let section = reference(field(g, sections, "0"));
    let items = reference(field(g, section, "items"));
    let task = reference(field(g, items, "0"));
    (sections, section, items, task)
}
fn add(
    d: &mut PublicDocument,
    level: f64,
    id: &str,
    label: &str,
    metadata: Option<&str>,
) -> InvocationOutcome {
    let mut values = vec![id, label];
    if let Some(data) = metadata {
        values.push(data)
    }
    let a = args(d, &values);
    let mut arguments = vec![
        Atom::Number(level),
        a[0].clone(),
        a[1].clone(),
        Atom::Number(999.),
    ];
    if metadata.is_some() {
        arguments.push(a[2].clone())
    }
    run(d, "addNode", arguments)
}
fn decoration(d: &mut PublicDocument, icon: Option<&str>, class: Option<&str>) -> Atom {
    let mut g = d.snapshot().unwrap();
    let id = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (key, v) in [("icon", icon), ("class", class)] {
        if let Some(v) = v {
            text(&mut g, id, key, v)
        }
    }
    d.apply(d.revision(), g).unwrap();
    Atom::Ref(id)
}
#[test]
fn kanban_exact_own_fields_methods_key_order_and_shared_last_provenance() {
    let d = document();
    let g = d.snapshot().unwrap();
    let r = root(&g);
    let (_, section, _, task) = ids(&g);
    let schema = d.model_schema();
    assert_eq!(schema.id, "pele/kanban@1");
    schema.validate().unwrap();
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
            "sections",
            "getType",
            "last",
            "sectionLevel",
            "misplaced"
        ]
    );
    assert!(matches!(field(&g, r, "title"), Atom::Undefined));
    assert!(matches!(field(&g, r, "misplaced"), Atom::Undefined));
    assert_eq!(reference(field(&g, r, "last")), task);
    assert!(matches!(field(&g, r, "getType"), Atom::BuiltinFunction(0)));
    assert_eq!(keys(&g, section), ["id", "level", "label", "items"]);
    assert_eq!(keys(&g, task), ["id", "level", "label"]);
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
        [("getType", 2), ("addNode", 5), ("decorateNode", 1)]
    );
    assert_eq!(
        schema.prototypes[0]
            .members
            .iter()
            .map(|m| (m.name, m.flags))
            .collect::<Vec<_>>(),
        [("addNode", 6), ("decorateNode", 6)]
    );
    let mut e = PublicEngine::empty();
    e.register(Kanban);
    // Metadata insertion precedes items; later decoration follows items. Replacing
    // an existing icon leaves its original insertion position intact.
    for (source, expected) in [
        (
            "kanban\nCol @{icon: x}\n  task[Task]",
            vec!["id", "level", "label", "icon", "items"],
        ),
        (
            "kanban\nCol\n::icon(x)\n  task[Task]",
            vec!["id", "level", "label", "items", "icon"],
        ),
        (
            "kanban\nCol @{icon: x}\n::icon(y)\n  task[Task]",
            vec!["id", "level", "label", "icon", "items"],
        ),
    ] {
        let d = e.parse(source, &ParseOptions::default()).unwrap();
        let g = d.snapshot().unwrap();
        assert_eq!(keys(&g, ids(&g).1), expected);
    }
}
#[test]
fn kanban_real_add_node_yaml_decoration_and_get_type_render_pinned_svg() {
    let mut d = document();
    let before = d.snapshot().unwrap();
    let r = root(&before);
    let (sections, section, items, task) = ids(&before);
    add(&mut d, 0., "doing", "Doing", None).result.unwrap();
    add(
        &mut d,
        2.,
        "new",
        "New",
        Some("label: \"Edited\", assigned: Alice, ticket: ABC-1, priority: High"),
    )
    .result
    .unwrap();
    let a = decoration(&mut d, Some("fa:fa-user"), Some("hot urgent"));
    assert!(matches!(
        run(&mut d, "decorateNode", vec![a]).result,
        Ok(Atom::Undefined)
    ));
    let g = d.snapshot().unwrap();
    assert_eq!(r, root(&g));
    assert_eq!(reference(field(&g, r, "sections")), sections);
    assert_eq!(reference(field(&g, sections, "0")), section);
    assert_eq!(reference(field(&g, section, "items")), items);
    assert_eq!(reference(field(&g, items, "0")), task);
    let last = reference(field(&g, r, "last"));
    assert_eq!(
        keys(&g, last),
        [
            "id",
            "level",
            "label",
            "assigned",
            "ticket",
            "priority",
            "icon",
            "cssClasses"
        ]
    );
    assert!(g.text(field(&g, last, "label")).unwrap().is("Edited"));
    hash(
        &mut d,
        None,
        "12eb46f12228bf6bfb20c7cb373b6ca06997581f3eabfe1a44e7cd7ad08b75b4",
    );
    hash(
        &mut d,
        Some(190.),
        "18ac59644d6632ab8ee5c113d0afd5e91f323f73e5020b6eb62f40f22126cda6",
    );
    // getType is an enumerable own callable, ignores its receiver and switches
    // by strict strings; _type passed to addNode has no effect on Kanban shape.
    for (start, end, expected) in [
        ("[", "]", 2.),
        ("(", ")", 1.),
        ("(", "]", 4.),
        ("((", "]", 3.),
        (")", "]", 4.),
        ("))", "]", 5.),
        ("{{", "}}", 6.),
        ("?", "", 0.),
        ("\u{1f642}", "", 0.),
    ] {
        let a = args(&mut d, &[start, end]);
        assert!(
            matches!(call(&mut d,"getType",Atom::Null,a).result,Ok(Atom::Number(n))if n==expected)
        );
    }
    assert!(matches!(
        call(
            &mut d,
            "getType",
            Atom::Undefined,
            vec![Atom::Null, Atom::Number(2.)]
        )
        .result,
        Ok(Atom::Number(0.))
    ));
}
#[test]
fn kanban_direct_edits_aliases_metadata_and_bookkeeping_match_pinned_svg() {
    // Exact hashes from public pinned parse/render and the mutations named here.
    for (id, wide, narrow) in [
        (
            "direct",
            "8a66ac2c2bfc5aa39b57b56c7f28c7a6cd257e388681c83d1d22a4db2d3b5914",
            "56f4c1d80eb9a36dede1af6f34e41aebe44d415f9ed31cc40153434ff16f1987",
        ),
        (
            "aliasItems",
            "5912217bda2cd794eb6bbbb8d5f97f55b50d30ac608e40d706018cb11c191fcc",
            "c4df12211ec034a5f83993d7f3ef3fcd1f974153dcf6d188dbebad864ecfdec9",
        ),
        (
            "aliasSections",
            "c271a4d5a0ecc9eb06b8e363f625668d639e1704646e3ccf0e1187f5ae7704c2",
            "63b2003de04e8c0ebfc0863db19760a8aa44f9589561f7d47a27c50edfdf9e8d",
        ),
        (
            "lastSection",
            "b0bde2fc81114f5bbe649f1b83d4b28bdbeb969d8c32f5eec8078b104f0ad1fa",
            "8cf9c14105076891ac05b5ba5fdb9217ce3b3b68bd0831d17b1ee17f98241452",
        ),
        (
            "metadata",
            "0e5fda073fa49a28c0c7d19270b85b8c26ac7e4bf3a51be700c74ed6c3469744",
            "b6352cfc4dcccbe9b9e7c39c330da0943498f2c0036a7878981de3056e0b5b79",
        ),
        (
            "falsyMetadata",
            "288d04b3b793b56aee6675cfd232317c246cd0951ec7db44ea911325c9d55fd0",
            "ead81a249427e2c66b359e72d2541b347eac6c3184c38cfa3a076d8200f2f788",
        ),
        (
            "misplaced",
            "e860e20053de44b495360007192527d702212614555f79dd07512e7090b7e2f1",
            "8376049cb7aed3207b7460da7b6d1070d68fb76034d115d4036f2a906880e759",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let r = root(&g);
        let (sections, section, items, task) = ids(&g);
        match id {
            "direct" => {
                text(&mut g, r, "title", "A & B");
                text(&mut g, r, "type", "fake");
                for (id, key, v) in [
                    (section, "label", "Edited <b>column</b>"),
                    (section, "cssClasses", "one two"),
                    (task, "label", "Edited **task**"),
                    (task, "assigned", "Alice"),
                    (task, "ticket", "ABC-1"),
                    (task, "priority", "very high"),
                    (task, "cssClasses", "urgent hot"),
                ] {
                    text(&mut g, id, key, v)
                }
                d.apply(d.revision(), g).unwrap();
            }
            "aliasItems" => {
                add(&mut d, 0., "other", "Other", None).result.unwrap();
                let mut g = d.snapshot().unwrap();
                let second = reference(field(&g, sections, "1"));
                set(&mut g, second, "items", Atom::Ref(items));
                text(&mut g, task, "label", "Shared");
                d.apply(d.revision(), g).unwrap();
                let a = decoration(&mut d, None, Some("section-class"));
                run(&mut d, "decorateNode", vec![a]).result.unwrap();
            }
            "aliasSections" => {
                set(&mut g, sections, "1", Atom::Ref(section));
                if let Node::Array { length, .. } = &mut g.nodes[sections as usize] {
                    *length = 2
                }
                set(&mut g, r, "last", Atom::Ref(section));
                d.apply(d.revision(), g).unwrap();
                let a = decoration(&mut d, None, Some("same"));
                run(&mut d, "decorateNode", vec![a]).result.unwrap();
            }
            "lastSection" => {
                set(&mut g, r, "last", Atom::Ref(section));
                d.apply(d.revision(), g).unwrap();
                let a = decoration(&mut d, Some("fa:fa-user"), Some("section-class"));
                run(&mut d, "decorateNode", vec![a]).result.unwrap();
            }
            "metadata" => {
                add(
                    &mut d,
                    2.,
                    "meta",
                    "Metadata",
                    Some("label: [one, two], assigned: 17, ticket: true, priority: Low, icon: x"),
                )
                .result
                .unwrap();
            }
            "falsyMetadata" => {
                add(
                    &mut d,
                    2.,
                    "meta",
                    "Metadata",
                    Some("label: false, assigned: 0, ticket: null, priority: false, icon: \"\""),
                )
                .result
                .unwrap();
            }
            "misplaced" => {
                set(&mut g, r, "sectionLevel", Atom::Number(2.));
                d.apply(d.revision(), g).unwrap();
                add(&mut d, 0., "below", "Below", None).result.unwrap();
                let g = d.snapshot().unwrap();
                assert_eq!(
                    reference(field(&g, r, "last")),
                    reference(field(&g, r, "misplaced"))
                );
            }
            _ => unreachable!(),
        }
        hash(&mut d, None, wide);
        hash(&mut d, Some(190.), narrow);
    }
}
#[test]
fn kanban_partial_errors_keep_last_misplaced_and_recover_without_reparsing() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (_, section, items, original) = ids(&g);
    set(&mut g, section, "items", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    let rev = d.revision();
    let g = error(
        add(&mut d, 2., "new", "New", None),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'push')",
    );
    let failed = reference(field(&g, r, "last"));
    assert_ne!(failed, original);
    assert!(g.text(field(&g, failed, "label")).unwrap().is("New"));
    assert!(d.revision() > rev);
    let before = json(&g);
    assert!(d
        .invoke(
            rev,
            MethodCall {
                method: MethodId(1),
                receiver: g.root.clone(),
                arguments: vec![]
            },
            &mut MetricsResources
        )
        .is_err());
    assert_eq!(json(&d.snapshot().unwrap()), before);
    let mut g = d.snapshot().unwrap();
    set(&mut g, section, "items", Atom::Ref(items));
    d.apply(d.revision(), g).unwrap();
    add(&mut d, 2., "fixed", "Fixed", None).result.unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "sectionLevel", Atom::Number(2.));
    d.apply(d.revision(), g).unwrap();
    add(&mut d, 0., "below", "Below", None).result.unwrap();
    let before = d.snapshot().unwrap();
    let last = reference(field(&before, r, "last"));
    assert_eq!(last, reference(field(&before, r, "misplaced")));
    let outcome = add(&mut d, 2., "after", "After", None);
    match outcome.result {
        Err(PublicError::Pele(e)) => {
            assert_eq!(e.code, ErrorCode::Semantic);
            assert_eq!(
                e.message,
                "Items without section detected, found section (\"Below\")"
            );
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(reference(field(&outcome.graph, r, "last")), last);
    // YAML validation runs before the misplaced check and mutates no reachable node.
    let outcome = add(&mut d, 2., "bad", "Bad", Some("shape: BAD"));
    assert!(
        matches!(outcome.result,Err(PublicError::Pele(PeleError{code:ErrorCode::Semantic,message,..}))if message=="No such shape: BAD. Shape names should be lowercase.")
    );
    let outcome = add(&mut d, 2., "bad", "Bad", Some("a: ["));
    assert!(
        matches!(outcome.result,Err(PublicError::Pele(PeleError{code:ErrorCode::Syntax,message,..}))if message=="Metadata of \"bad\" is not valid YAML. YAML: unexpected end of the stream within a flow collection")
    );
    let a = args(&mut d, &["id", "Label"]);
    error(
        run(
            &mut d,
            "addNode",
            vec![
                Atom::Number(0.),
                a[0].clone(),
                a[1].clone(),
                Atom::Undefined,
                Atom::Null,
            ],
        ),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'includes')",
    );
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "misplaced", Atom::Undefined);
    d.apply(d.revision(), g).unwrap();
    add(&mut d, 2., "recovered", "Recovered", None)
        .result
        .unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    // Array.push publishes the new section index before its length overflows.
    let mut g = d.snapshot().unwrap();
    let sections = reference(field(&g, r, "sections"));
    g.nodes[sections as usize] = Node::Array {
        length: u32::MAX,
        properties: vec![],
    };
    set(&mut g, r, "sectionLevel", Atom::Number(0.));
    d.apply(d.revision(), g).unwrap();
    let g = error(
        add(&mut d, 0., "overflow", "Overflow", None),
        JavaScriptErrorKind::RangeError,
        "Invalid array length",
    );
    assert!(matches!(
        g.nodes[sections as usize],
        Node::Array {
            length: u32::MAX,
            ..
        }
    ));
    assert_eq!(
        reference(field(&g, sections, "4294967295")),
        reference(field(&g, r, "last"))
    );
}
#[test]
fn kanban_borrowed_builder_and_decorator_preserve_exact_alias_targets() {
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (sections, _, _, task) = ids(&g);
    let borrowed = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    for (key, a) in [
        ("sections", Atom::Ref(sections)),
        ("sectionLevel", Atom::Number(0.)),
        ("last", Atom::Ref(task)),
        ("misplaced", Atom::Undefined),
    ] {
        set(&mut g, borrowed, key, a)
    }
    set(&mut g, r, "borrowed", Atom::Ref(borrowed));
    d.apply(d.revision(), g).unwrap();
    let a = args(&mut d, &["new", "New"]);
    call(
        &mut d,
        "addNode",
        Atom::Ref(borrowed),
        vec![
            Atom::Number(2.),
            a[0].clone(),
            a[1].clone(),
            Atom::Undefined,
        ],
    )
    .result
    .unwrap();
    let g = d.snapshot().unwrap();
    let new = reference(field(&g, borrowed, "last"));
    assert_ne!(new, task);
    assert_eq!(reference(field(&g, r, "last")), task);
    let a = decoration(&mut d, Some("x"), Some("hot"));
    call(&mut d, "decorateNode", Atom::Ref(borrowed), vec![a])
        .result
        .unwrap();
    let g = d.snapshot().unwrap();
    assert!(g.text(field(&g, new, "cssClasses")).unwrap().is("hot"));
    assert!(!keys(&g, task).contains(&"cssClasses".into()));
    // No previous node: decoration is intentionally ignored, as in actual Pele.
    let mut g = d.snapshot().unwrap();
    set(&mut g, r, "last", Atom::Null);
    d.apply(d.revision(), g).unwrap();
    let a = decoration(&mut d, Some("ignored"), None);
    assert!(matches!(
        run(&mut d, "decorateNode", vec![a]).result,
        Ok(Atom::Undefined)
    ));
    error(
        call(&mut d, "decorateNode", Atom::Null, vec![Atom::Null]),
        JavaScriptErrorKind::TypeError,
        "Cannot read properties of null (reading 'last')",
    );
}
struct Failure;
impl RenderResources for Failure {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Err(PeleError::new("kanban host failed", ErrorCode::Semantic))
    }
}
#[test]
fn kanban_native_render_deep_extras_utf16_and_error_recovery() {
    let mut d = document();
    let expected = d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut g = d.snapshot().unwrap();
    let r = root(&g);
    let (_, _, _, task) = ids(&g);
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
        "kanban host failed"
    );
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(1);
    assert_eq!(
        d.render(&options, &mut MetricsResources).unwrap_err().code,
        ErrorCode::Limit
    );
    assert_eq!(json(&d.snapshot().unwrap()), saved);
    let a = args(&mut d, &["Task"]);
    let mut g = d.snapshot().unwrap();
    set(&mut g, task, "label", Atom::String(lone));
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("lone UTF-16"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, task, "label", a[0].clone());
    d.apply(d.revision(), g).unwrap();
    assert_eq!(
        d.render(&RenderOptions::default(), &mut MetricsResources)
            .unwrap(),
        expected
    );
    let mut g = d.snapshot().unwrap();
    text(&mut g, task, "label", "Native **edited**");
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

#[test]
fn kanban_optional_values_partial_decoration_and_explicit_shape_gaps() {
    for (undefined_label, expected) in [
        (
            false,
            "3da43b79434c6e80d808304bd70d490ee4f25e306aa0e9be08a00b18f3f54f21",
        ),
        (
            true,
            "320db0e34a2f5895ef619950c1547f531282cc7ca34a808297486baa7c42a560",
        ),
    ] {
        let mut d = document();
        let mut g = d.snapshot().unwrap();
        let task = ids(&g).3;
        if undefined_label {
            set(&mut g, task, "label", Atom::Undefined);
        } else {
            set(&mut g, task, "icon", Atom::Bool(false));
            set(&mut g, task, "priority", Atom::Null);
            set(&mut g, task, "cssClasses", Atom::Null);
        }
        d.apply(d.revision(), g).unwrap();
        hash(&mut d, None, expected);
    }
    let mut d = document();
    let mut g = d.snapshot().unwrap();
    let (_, _, items, task) = ids(&g);
    text(&mut g, task, "cssClasses", "old");
    let property = g.nodes[task as usize].properties_mut().last_mut().unwrap();
    property.flags = 5;
    d.apply(d.revision(), g).unwrap();
    let decoration = decoration(&mut d, Some("new-icon"), Some("new-class"));
    let result = run(&mut d, "decorateNode", vec![decoration]);
    // Exact read-only TypeError wording includes a minified constructor name in
    // pinned JS; native classification and earlier icon mutation are preserved.
    assert!(matches!(
        result.result,
        Err(PublicError::JavaScript {
            name: JavaScriptErrorKind::TypeError,
            ..
        })
    ));
    assert!(result
        .graph
        .text(field(&result.graph, task, "icon"))
        .unwrap()
        .is("new-icon"));
    assert!(result
        .graph
        .text(field(&result.graph, task, "cssClasses"))
        .unwrap()
        .is("old"));
    let mut g = d.snapshot().unwrap();
    let original = field(&g, items, "0").clone();
    g.nodes[items as usize].properties_mut().clear();
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .unwrap_err()
        .message
        .contains("sparse active arrays"));
    let mut g = d.snapshot().unwrap();
    set(&mut g, items, "0", original);
    d.apply(d.revision(), g).unwrap();
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
    let mut g = d.snapshot().unwrap();
    let raw = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800]));
    d.apply(d.revision(), g).unwrap();
    let a = args(&mut d, &["new", "New"]);
    let outcome = run(
        &mut d,
        "addNode",
        vec![
            Atom::Number(0.),
            a[0].clone(),
            a[1].clone(),
            Atom::Undefined,
            Atom::String(raw),
        ],
    );
    assert!(
        matches!(outcome.result,Err(PublicError::Pele(PeleError{message,..}))if message.contains("YAML metadata containing lone UTF-16"))
    );
    assert!(d
        .render(&RenderOptions::default(), &mut MetricsResources)
        .is_ok());
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
            _ => panic!("Kanban baseline uses only records and arrays"),
        },
        _ => panic!("Kanban baseline has no host references"),
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
fn mutable_kanban_preserves_entire_existing_pinned_render_and_error_corpus() {
    let fixture: Json = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/kanban.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut e = PublicEngine::empty();
    e.register(Kanban);
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
