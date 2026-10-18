//! Returned Error values are branded arena objects, not thrown operation errors.
use pele::compat::{diagrams::agentflow::AgentFlow, public::*, text::MetricsResources};
fn reference(a: &Atom) -> u32 {
    if let Atom::Ref(id) = a {
        *id
    } else {
        panic!("reference")
    }
}
fn field<'a>(g: &'a ModelGraph, id: u32, key: &str) -> &'a Atom {
    &g.nodes[id as usize]
        .properties()
        .iter()
        .find(|p| g.strings[p.key as usize].is(key))
        .unwrap()
        .value
}
fn set(g: &mut ModelGraph, id: u32, key: &str, value: Atom) {
    if let Some(p) = g.nodes[id as usize]
        .properties()
        .iter()
        .position(|p| g.strings[p.key as usize].is(key))
    {
        g.nodes[id as usize].properties_mut()[p].value = value
    } else {
        let k = g.strings.len() as u32;
        g.strings.push(key.into());
        g.nodes[id as usize].properties_mut().push(Property {
            key: k,
            value,
            flags: 7,
        });
    }
}
#[test]
fn agentflow_returns_native_pele_errors_with_raw_utf16_ieee_fields_and_stable_identity() {
    let mut e = PublicEngine::empty();
    e.register(AgentFlow);
    let mut d = e
        .parse("agentflow-beta\nA --> B", &Default::default())
        .unwrap();
    let mut g = d.snapshot().unwrap();
    let root = reference(&g.root);
    set(&mut g, root, "lineOffset", Atom::Number(-1.5));
    let loc = g.nodes.len() as u32;
    g.nodes.push(Node::Object { properties: vec![] });
    set(&mut g, loc, "first_line", Atom::Number(4.));
    set(&mut g, loc, "first_column", Atom::Number(-1.25));
    let message = g.strings.len() as u32;
    g.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    d.apply(0, g).unwrap();
    let method = d
        .model_schema()
        .methods
        .iter()
        .find(|m| m.name == "shapeError")
        .unwrap()
        .id;
    let out = d
        .invoke(
            1,
            MethodCall {
                method,
                receiver: Atom::Ref(root),
                arguments: vec![Atom::String(message), Atom::Ref(loc)],
            },
            &mut MetricsResources,
        )
        .unwrap();
    let id = reference(out.result.as_ref().unwrap());
    assert!(matches!(
        out.graph.nodes[id as usize],
        Node::Error {
            error_type: ErrorObjectKind::Pele,
            ..
        }
    ));
    assert!(matches!(field(&out.graph,id,"line"),Atom::Number(v)if *v==2.5));
    assert!(matches!(field(&out.graph,id,"column"),Atom::Number(v)if *v== -0.25));
    assert!(
        matches!(field(&out.graph,id,"message"),Atom::String(i)if out.graph.strings[*i as usize].0==[0xd800,65,0xdc00])
    );
    assert_eq!(out.graph.nodes[id as usize].properties()[0].flags, 6);
    let mut graph = out.graph;
    set(&mut graph, root, "error", Atom::Ref(id));
    let bytes = encode_binary_exchange(out.revision, &graph, b"", DEFAULT_BINARY_BUDGET).unwrap();
    let decoded = decode_binary_exchange(&bytes, DEFAULT_BINARY_BUDGET).unwrap();
    d.apply(decoded.revision, decoded.graph).unwrap();
    let kept = d.snapshot().unwrap();
    assert_eq!(reference(field(&kept, root, "error")), id);
    assert!(d
        .render_operation(d.revision(), &Default::default(), &mut MetricsResources)
        .unwrap()
        .result
        .is_ok());
    let revision = d.revision();
    let out = d
        .invoke(
            revision,
            MethodCall {
                method,
                receiver: Atom::Ref(root),
                arguments: vec![],
            },
            &mut MetricsResources,
        )
        .unwrap();
    let id = reference(out.result.as_ref().unwrap());
    assert!(!out.graph.nodes[id as usize]
        .properties()
        .iter()
        .any(|p| out.graph.strings[p.key as usize].is("message")));
    assert!(matches!(field(&out.graph,id,"line"),Atom::Number(v)if *v==0.));
}
#[test]
fn all_error_brands_survive_native_graph_edit_exchange_without_changing_drawing() {
    let mut e = PublicEngine::empty();
    e.register(AgentFlow);
    let mut d = e.parse("agentflow-beta\nA", &Default::default()).unwrap();
    let before = d
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    let mut graph = d.snapshot().unwrap();
    let root = reference(&graph.root);
    let kinds = [
        ErrorObjectKind::Error,
        ErrorObjectKind::Pele,
        ErrorObjectKind::Type,
        ErrorObjectKind::Range,
        ErrorObjectKind::Syntax,
        ErrorObjectKind::Reference,
        ErrorObjectKind::Eval,
        ErrorObjectKind::Uri,
    ];
    for (i, kind) in kinds.into_iter().enumerate() {
        let id = graph.nodes.len() as u32;
        graph.nodes.push(Node::Error {
            error_type: kind,
            properties: vec![],
        });
        set(&mut graph, id, "line", Atom::Number(f64::INFINITY));
        set(&mut graph, root, &format!("error{i}"), Atom::Ref(id));
    }
    let bytes = encode_binary_exchange(0, &graph, b"", DEFAULT_BINARY_BUDGET).unwrap();
    let decoded = decode_binary_exchange(&bytes, DEFAULT_BINARY_BUDGET).unwrap();
    assert_eq!(
        serde_json::to_value(&graph).unwrap(),
        serde_json::to_value(&decoded.graph).unwrap()
    );
    d.apply(0, decoded.graph).unwrap();
    assert_eq!(
        d.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .rendered
            .svg,
        before.rendered.svg
    );
}
