//! Ordinary mutable parsed-model feature through the opt-in public document API.
use pele::compat::{
    diagrams::pie::Pie,
    public::{Atom, JsString, Node, PublicEngine},
    Engine, ErrorCode, RenderOptions,
};
fn add_string(graph: &mut pele::compat::public::ModelGraph, value: &str) -> Atom {
    let id = graph.strings.len() as u32;
    graph.strings.push(JsString::from(value));
    Atom::String(id)
}
#[test]
fn mutable_pie_map_edits_replacement_provenance_and_recovery() {
    let mut registry = PublicEngine::empty();
    registry.register(Pie);
    let mut document = registry
        .parse("pie title Before\n\"A\":1\n\"B\":2", &Default::default())
        .unwrap();
    let mut graph = document.snapshot().unwrap();
    let original_map = match graph.field("sections").unwrap().unwrap() {
        Atom::Ref(id) => *id,
        _ => panic!(),
    };
    let b = add_string(&mut graph, "B");
    let a = add_string(&mut graph, "A");
    let Node::Map { entries, .. } = &mut graph.nodes[original_map as usize] else {
        panic!()
    };
    entries.clear();
    entries.extend([(b, Atom::Number(4.0)), (a, Atom::Number(1.0))]);
    let title = add_string(&mut graph, "After");
    let fake_type = add_string(&mut graph, "custom-language");
    let Atom::Ref(root) = graph.root else {
        panic!()
    };
    let Node::Object { properties } = &mut graph.nodes[root as usize] else {
        panic!()
    };
    for field in properties {
        if graph.strings[field.key as usize].is("title") {
            field.value = title.clone()
        } else if graph.strings[field.key as usize].is("type") {
            field.value = fake_type.clone()
        }
    }
    document.apply(0, graph.clone()).unwrap();
    assert_eq!(
        document.apply(0, graph.clone()).unwrap_err().message,
        "Stale public model revision"
    );
    let result = document
        .render(
            &RenderOptions::default(),
            &mut pele::compat::text::MetricsResources,
        )
        .unwrap();
    let expected = Engine::with_builtins()
        .render_source(
            "pie title After\n\"B\":4\n\"A\":1",
            &Default::default(),
            &mut pele::compat::text::MetricsResources,
        )
        .unwrap();
    assert_eq!(result, expected);
    assert_eq!(document.diagram_type().as_str(), "pie");
    let saved = document.snapshot().unwrap();
    assert!(matches!(saved.field("sections").unwrap(),Some(Atom::Ref(id))if *id==original_map));
    let mut replacement = document.snapshot().unwrap();
    let label = add_string(&mut replacement, "Replacement");
    let map_id = replacement.nodes.len() as u32;
    replacement.nodes.push(Node::Map {
        entries: vec![(label, Atom::Number(8.0))],
        properties: vec![],
    });
    let Atom::Ref(root_id) = replacement.root else {
        panic!()
    };
    let Node::Object { properties } = &mut replacement.nodes[root_id as usize] else {
        panic!()
    };
    properties
        .iter_mut()
        .find(|property| replacement.strings[property.key as usize].is("sections"))
        .unwrap()
        .value = Atom::Ref(map_id);
    document.apply(1, replacement).unwrap();
    let expected = Engine::with_builtins()
        .render_source(
            "pie title After\n\"Replacement\":8",
            &Default::default(),
            &mut pele::compat::text::MetricsResources,
        )
        .unwrap();
    assert_eq!(
        document
            .render(
                &Default::default(),
                &mut pele::compat::text::MetricsResources
            )
            .unwrap(),
        expected
    );
    assert!(
        matches!(document.snapshot().unwrap().field("sections").unwrap(), Some(Atom::Ref(id)) if *id == map_id)
    );
    let mut limit = RenderOptions::default();
    limit.render.output_limit = Some(1);
    assert_eq!(
        document
            .render(&limit, &mut pele::compat::text::MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(
        document
            .render(
                &RenderOptions::default(),
                &mut pele::compat::text::MetricsResources
            )
            .unwrap(),
        expected
    );
    registry.register(Pie);
    drop(registry);
    assert_eq!(
        document
            .render(
                &RenderOptions::default(),
                &mut pele::compat::text::MetricsResources
            )
            .unwrap(),
        expected
    );
}
#[test]
fn public_document_graph_keeps_ieee_utf16_deep_cycles_without_recursive_json() {
    let mut engine = PublicEngine::empty();
    engine.register(Pie);
    let mut document = engine.parse("pie\n\"A\":1", &Default::default()).unwrap();
    let mut graph = document.snapshot().unwrap();
    let root = graph.root.clone();
    let mut tail = root.clone();
    let key = graph.strings.len() as u32;
    graph.strings.push(JsString::from("next"));
    for _ in 0..5000 {
        let id = graph.nodes.len() as u32;
        graph.nodes.push(Node::Object {
            properties: vec![pele::compat::public::Property {
                key,
                value: tail,
                flags: 7,
            }],
        });
        tail = Atom::Ref(id)
    }
    let Atom::Ref(root_id) = root.clone() else {
        panic!()
    };
    let Node::Object { properties } = &mut graph.nodes[root_id as usize] else {
        panic!()
    };
    properties.push(pele::compat::public::Property {
        key,
        value: tail,
        flags: 7,
    });
    let units = graph.strings.len() as u32;
    graph.strings.push(JsString(vec![0xd800, 0x0041, 0xdc00]));
    let Node::Object { properties } = &mut graph.nodes[root_id as usize] else {
        panic!()
    };
    for (n, value) in [
        ("payload", Atom::String(units)),
        ("nan", Atom::Number(f64::NAN)),
        ("negativeZero", Atom::Number(-0.0)),
        ("infinity", Atom::Number(f64::INFINITY)),
    ] {
        let k = graph.strings.len() as u32;
        graph.strings.push(JsString::from(n));
        properties.push(pele::compat::public::Property {
            key: k,
            value,
            flags: 7,
        })
    }
    let json = serde_json::to_string(&graph).unwrap();
    let decoded = serde_json::from_str(&json).unwrap();
    document.apply(0, decoded).unwrap();
    let kept = document.snapshot().unwrap();
    assert_eq!(kept.strings[units as usize].0, vec![0xd800, 0x0041, 0xdc00]);
    assert!(
        matches!(kept.field("negativeZero").unwrap(),Some(Atom::Number(n))if n.to_bits()==(-0.0f64).to_bits())
    );
    assert!(matches!(kept.field("nan").unwrap(),Some(Atom::Number(n))if n.is_nan()));
    let result = document
        .render(
            &Default::default(),
            &mut pele::compat::text::MetricsResources,
        )
        .unwrap();
    assert!(result.rendered.svg.contains("A"));
}
