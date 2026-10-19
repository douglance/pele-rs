//! Native consumers exchange edited public documents without JSON expansion.
use pele::compat::{
    diagrams::{class_diagram::Class, pie::Pie, sankey::Sankey},
    public::*,
    text::MetricsResources,
    *,
};
use serde_json::Value;
fn ref_id(a: &Atom) -> u32 {
    if let Atom::Ref(i) = a {
        *i
    } else {
        panic!("reference")
    }
}
fn doc() -> PublicDocument {
    let mut e = PublicEngine::empty();
    e.register(Sankey);
    e.parse("sankey\nA,B,5\nA,C,2", &Default::default())
        .unwrap()
}
#[test]
fn binary_document_exchange_preserves_aliases_cycles_raw_strings_numbers_and_rendering() {
    let mut d = doc();
    let mut graph = d.snapshot().unwrap();
    let root = ref_id(&graph.root);
    let key = graph.strings.len() as u32;
    graph.strings.push("consumer".into());
    let text = graph.strings.len() as u32;
    graph.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
    let next = graph.strings.len() as u32;
    graph.strings.push("next".into());
    let payload = graph.nodes.len() as u32;
    graph.nodes.push(Node::Array {
        length: 5,
        properties: vec![Property {
            key,
            value: Atom::String(text),
            flags: 7,
        }],
    });
    let date = graph.nodes.len() as u32;
    graph.nodes.push(Node::Date {
        milliseconds: f64::from_bits(0x7ff8000000000042),
        properties: vec![],
    });
    let values = graph.nodes.len() as u32;
    graph.nodes.push(Node::Set {
        values: vec![
            Atom::Number(-0.),
            Atom::Number(f64::INFINITY),
            Atom::Number(f64::NEG_INFINITY),
            Atom::Bool(true),
            Atom::Bool(false),
            Atom::Null,
            Atom::Undefined,
            Atom::Ref(date),
            Atom::HostReference(42),
            Atom::BuiltinFunction(d.model_schema().methods[0].id.0),
        ],
        properties: vec![],
    });
    graph.nodes[payload as usize]
        .properties_mut()
        .push(Property {
            key: next,
            value: Atom::Ref(values),
            flags: 5,
        });
    let mut tail = Atom::Ref(payload);
    for _ in 0..5000 {
        let id = graph.nodes.len() as u32;
        graph.nodes.push(Node::Object {
            properties: vec![Property {
                key: next,
                value: tail,
                flags: 7,
            }],
        });
        tail = Atom::Ref(id);
    }
    graph.nodes[root as usize].properties_mut().push(Property {
        key,
        value: tail,
        flags: 7,
    });
    graph.nodes[date as usize].properties_mut().push(Property {
        key: next,
        value: Atom::Ref(payload),
        flags: 7,
    });
    let metadata = b"{\"kind\":\"snapshot\"}";
    let bytes = encode_binary_exchange(0, &graph, metadata, DEFAULT_BINARY_BUDGET).unwrap();
    let exchange = decode_binary_exchange(&bytes, DEFAULT_BINARY_BUDGET).unwrap();
    assert_eq!(exchange.revision, 0);
    assert_eq!(exchange.metadata, metadata);
    assert_eq!(
        serde_json::to_value(&exchange.graph).unwrap(),
        serde_json::to_value(&graph).unwrap()
    );
    d.apply(exchange.revision, exchange.graph).unwrap();
    let rendered = d
        .render(&Default::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!(
        rendered.rendered.svg,
        Engine::with_builtins()
            .render_source(
                "sankey\nA,B,5\nA,C,2",
                &Default::default(),
                &mut MetricsResources
            )
            .unwrap()
            .rendered
            .svg
    );
    // Returning a new operation graph keeps all native identities, not a JSON view.
    let outcome = d
        .render_operation(1, &Default::default(), &mut MetricsResources)
        .unwrap();
    let result = encode_binary_exchange(
        outcome.revision,
        &outcome.graph,
        b"result",
        DEFAULT_BINARY_BUDGET,
    )
    .unwrap();
    let saved = decode_binary_exchange(&result, DEFAULT_BINARY_BUDGET).unwrap();
    assert_eq!(saved.revision, 2);
    assert_eq!(ref_id(&saved.graph.root), root);
    if let Node::Date { milliseconds, .. } = &saved.graph.nodes[date as usize] {
        assert_eq!(milliseconds.to_bits(), 0x7ff8000000000042)
    } else {
        panic!("Date identity")
    }
}
#[test]
fn binary_limits_and_invalid_exchanges_leave_consumer_documents_usable() {
    let mut e = PublicEngine::empty();
    e.register(Pie);
    let mut d = e.parse("pie\n\"A\":2", &Default::default()).unwrap();
    let graph = d.snapshot().unwrap();
    let good = encode_binary_exchange(0, &graph, b"", DEFAULT_BINARY_BUDGET).unwrap();
    assert_eq!(
        encode_binary_exchange(0, &graph, b"", good.len() - 1)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(
        decode_binary_exchange(&good, good.len() - 1)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    for n in (0..34).chain([good.len() - 1, good.len() - 2]) {
        assert!(
            decode_binary_exchange(&good[..n], DEFAULT_BINARY_BUDGET).is_err(),
            "truncation {n}"
        );
    }
    let mut bad = good.clone();
    bad[0] ^= 1;
    assert!(decode_binary_exchange(&bad, DEFAULT_BINARY_BUDGET).is_err());
    let mut bad = good.clone();
    bad[8..12].copy_from_slice(&2u32.to_le_bytes());
    assert!(decode_binary_exchange(&bad, DEFAULT_BINARY_BUDGET).is_err());
    let mut bad = good.clone();
    bad[29..33].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode_binary_exchange(&bad, DEFAULT_BINARY_BUDGET)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let mut bad = good.clone();
    bad.push(0);
    assert!(decode_binary_exchange(&bad, DEFAULT_BINARY_BUDGET).is_err());
    let mut invalid_graph = graph.clone();
    invalid_graph.root = Atom::Ref(u32::MAX);
    assert!(encode_binary_exchange(0, &invalid_graph, b"", DEFAULT_BINARY_BUDGET).is_err());
    let decoded = decode_binary_exchange(&good, DEFAULT_BINARY_BUDGET).unwrap();
    d.apply(decoded.revision, decoded.graph).unwrap();
    assert_eq!(
        d.render(&Default::default(), &mut MetricsResources)
            .unwrap()
            .diagram_type
            .as_str(),
        "pie"
    );
    assert!(d.apply(0, graph).is_err());
}
#[test]
fn valid_class_stress_models_fit_binary_without_raising_json_or_graph_limits() {
    let f: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/class.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut engine = PublicEngine::empty();
    engine.register(Class);
    for name in [
        "stress/1,600 classDefs applied to 1,600 classes",
        "stress/a classDef of 5,000 declarations on 800 classes, each with a style of its own",
    ] {
        let case = f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == name)
            .unwrap();
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let mut document = engine
            .parse(case["source"].as_str().unwrap(), &options.parse)
            .unwrap();
        let graph = document.snapshot().unwrap();
        let encoded = encode_binary_exchange(0, &graph, b"", DEFAULT_BINARY_BUDGET).unwrap();
        assert!(encoded.len() < DEFAULT_BINARY_BUDGET);
        let exchanged = decode_binary_exchange(&encoded, DEFAULT_BINARY_BUDGET).unwrap();
        document.apply(exchanged.revision, exchanged.graph).unwrap();
        let result = document.render(&options, &mut MetricsResources).unwrap();
        use sha2::{Digest, Sha256};
        assert_eq!(
            format!("{:x}", Sha256::digest(result.rendered.svg.as_bytes())),
            case["expected"]["result"]["svgSha256"],
            "{name}"
        );
    }
}
