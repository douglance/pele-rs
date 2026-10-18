//! Integration coverage through public preparation/discovery, using pinned upstream documents.
use pele::compat::{Engine, ErrorCode, PrepareOptions, DIAGRAM_TYPES};
use serde_json::Value;
#[test]
fn pinned_documents_prepare_identically() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/source.json")).unwrap();
    assert_eq!(fixture["revision"], pele::compat::UPSTREAM_REVISION);
    let engine = Engine::empty();
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let options: PrepareOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let source = case["source"].as_str().unwrap();
        let actual = match engine.inspect(source, &options) {
            Ok(v) => serde_json::to_value(v).unwrap(),
            Err(e) => serde_json::json!({"error":e}),
        };
        if actual != case["expected"] {
            mismatches.push(format!(
                "{}\nsource: {}\nactual: {}\nexpected: {}",
                case["id"], case["source"], actual, case["expected"]
            ));
        }
        if !case["id"].as_str().unwrap().starts_with("limit/") {
            let actual = match engine.detect_type(source) {
                Ok(v) => serde_json::to_value(v).unwrap(),
                Err(e) => serde_json::json!({"error":e}),
            };
            let expected = if case["expected"].get("error").is_some() {
                case["expected"].clone()
            } else {
                case["expected"]["type"].clone()
            };
            assert_eq!(actual, expected, "detector {}", case["id"]);
        }
        assert!(
            !engine.supports(source),
            "unregistered family must not be advertised"
        );
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    );
}
#[test]
fn discovery_and_contract_do_not_overclaim_parity() {
    let engine = Engine::empty();
    let caps = engine.capabilities();
    assert_eq!(caps.len(), 33);
    for (cap, kind) in caps.iter().zip(DIAGRAM_TYPES) {
        assert_eq!(cap.diagram_type, kind.into());
        assert!(cap.known);
        assert!(!cap.registered);
        assert!(
            !cap.syntax_parity && !cap.semantic_parity && !cap.render_parity && !cap.host_parity
        )
    }
    let (status, json) = pele::compat::inspect_envelope("---\nconfig: {\n---\npie", "");
    let result: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(status, 1);
    assert_eq!(result["version"], 2);
    assert_eq!(result["error"]["code"], "syntax");
    let error =
        pele::compat::PeleError::syntax("pie", "🙂 broken\nnext", 4, "Expected label", false);
    assert_eq!(error.code, ErrorCode::Syntax);
    assert_eq!(error.line, 1);
    assert_eq!(error.column, 3);
    assert_eq!(error.snippet, "🙂 broken\n--^");
    assert!(
        pele::render("graph TD\nA --> B", &Default::default()).is_ok(),
        "legacy interface remains usable"
    );
}

#[test]
fn very_deep_init_config_returns_documented_resource_error() {
    let source = format!(
        "%%{{init: {{\"value\": {}1{}}}}}%%\ninfo",
        "[".repeat(128),
        "]".repeat(128)
    );
    let error = Engine::empty()
        .inspect(&source, &PrepareOptions::default())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Limit);
}
