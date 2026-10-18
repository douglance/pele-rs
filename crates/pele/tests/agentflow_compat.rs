//! Public feature-slice integration: agentflow source preparation, parser and ordered models.
use pele::compat::{
    diagrams::agentflow, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
};
struct Host<'a>(&'a Value);
impl RenderResources for Host<'_> {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        if self.0["measure"] == "fixed" {
            Ok(request.text.encode_utf16().count() as f64
                * request.size
                * if request.style & 1 != 0 { 0.71 } else { 0.6 })
        } else {
            pele::compat::text::MetricsResources.text_width(request)
        }
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        Ok(self.0["icons"][name]
            .as_str()
            .map(|s| TrustedSvg::from_trusted_markup(s.into())))
    }
}
use serde_json::Value;
fn fixture() -> &'static Value {
    static FIXTURE: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    FIXTURE.get_or_init(|| {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(
            include_bytes!("../../../fixtures/pele/agentflow.json.gz").as_slice(),
        );
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    })
}
#[test]
fn pinned_agentflow_parser_corpus() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(agentflow::AgentFlow);
            engine
                .parse(case["source"].as_str().unwrap(), &options)
                .map(|d| serde_json::to_value(d.model_view().unwrap()).unwrap())
        } {
            Ok(model) => serde_json::json!({"model":model}),
            Err(error) => serde_json::json!({"error":error}),
        };
        let mut expected = case["expected"].clone();
        expected.as_object_mut().unwrap().remove("result");
        expected.as_object_mut().unwrap().remove("semanticModel");
        if expected.get("model").is_some() {
            expected.as_object_mut().unwrap().remove("error");
        }
        if actual != expected {
            if failures.len() < 10 {
                eprintln!(
                    "{} source={:?}\nactual={}\nexpected={}",
                    case["id"], case["source"], actual, expected
                )
            }
            failures.push(case["id"].clone())
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} mismatches: {:?}",
        failures.len(),
        cases.len(),
        failures.get(..10.min(failures.len())).unwrap()
    );
}

#[test]
fn pinned_agentflow_complete_svg_corpus() {
    use pele::compat::{diagrams::agentflow::AgentFlow, Engine, RenderOptions};
    let fixture = fixture();
    let mut engine = Engine::empty();
    engine.register(AgentFlow);
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match engine.render_source(
            case["source"].as_str().unwrap(),
            &options,
            &mut Host(&case["host"]),
        ) {
            Ok(result) => {
                let mut v = serde_json::to_value(result).unwrap();
                if case["expected"]["result"].get("svgSha256").is_some() {
                    use sha2::{Digest, Sha256};
                    let svg = v.as_object_mut().unwrap().remove("svg").unwrap();
                    v["svgSha256"] =
                        format!("{:x}", Sha256::digest(svg.as_str().unwrap().as_bytes())).into();
                }
                v
            }
            Err(error) => serde_json::json!({"error":error}),
        };
        let expected = if case["expected"].get("error").is_some() {
            serde_json::json!({"error":case["expected"]["error"]})
        } else {
            case["expected"]["result"].clone()
        };
        if actual != expected {
            if failures.len() < 6 {
                eprintln!(
                    "{} source={:?}\nactual={}\nexpected={}",
                    case["id"], case["source"], actual, expected
                )
            }
            failures.push(case["id"].clone());
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} SVG mismatches: {:?}",
        failures.len(),
        cases.len(),
        failures.get(..10.min(failures.len())).unwrap()
    );
}

#[test]
fn pinned_agentflow_semantic_model_corpus() {
    use pele::compat::{
        diagrams::agentflow::{AgentFlow, AgentflowModel},
        Engine,
    };
    let fixture = fixture();
    let mut engine = Engine::empty();
    engine.register(AgentFlow);
    let mut failures = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let Some(expected) = case["expected"].get("semanticModel") else {
            continue;
        };
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let doc = engine
            .parse(case["source"].as_str().unwrap(), &options)
            .unwrap();
        let actual =
            serde_json::to_value(doc.model::<AgentflowModel>().unwrap().semantic_model()).unwrap();
        assert_eq!(serde_json::to_value(doc.query("semanticModel", &pele::compat::Value::Null).unwrap()).unwrap(), *expected);
        if &actual != expected {
            if failures.len() < 6 {
                eprintln!(
                    "{} source={:?}\nactual={}\nexpected={}",
                    case["id"], case["source"], actual, expected
                )
            }
            failures.push(case["id"].clone());
        }
    }
    assert!(
        failures.is_empty(),
        "{} semantic mismatches: {:?}",
        failures.len(),
        failures.get(..10.min(failures.len())).unwrap()
    );
}

#[test]
fn retained_agentflow_semantics_containment_native_reflow_and_recovery() {
    use pele::compat::{
        diagrams::agentflow::{AgentFlow, AgentflowModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(AgentFlow);
    let source="---\ntitle: Agent workflow\n---\nagentflow-beta LR\naccTitle: Workflow\naccDescr: Tools and shared reference\nglobal\nref@{ shape: refdoc, uri: shared }\nend\nflow f[\"Agent\"]@{ model: local }\na[\"**Plan**\"]@{ shape: task, instruction: plan } --> b@{ shape: tool, instruction: execute }\nb --> d{Ready?}\nd --> act@{ shape: action }\nact -.- ref\nb --> f\nend\nflow hidden[\"Collapsed\"]@{ view: collapsed }\nx --> y\nend\ny --> a\nconnector c[\"MCP\"]@{ protocol: mcp }\nc --> a\ninput@{ shape: input } --> c\nclick c href \"https://example.com/tool\" \"Open connector\" _blank\n";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = doc.model::<AgentflowModel>().unwrap();
    assert_eq!(model.connectors.len(), 1);
    assert!(model
        .subgraphs
        .iter()
        .all(|s| !s.nodes.iter().any(|id| id == "ref")));
    let semantic = model.semantic_model();
    let semantic_json = serde_json::to_value(semantic).unwrap();
    assert_eq!(
        semantic_json["connectors"][0]["metadata"]["protocol"],
        "mcp"
    );
    assert_eq!(
        semantic_json["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == "b")
            .unwrap()["metadata"],
        serde_json::json!({"instruction":"execute"})
    );
    assert!(model.mappings.iter().any(|m| m.id == "f"));
    let before = doc.model_view().unwrap();
    let wide = engine
        .render_document(&doc, &RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut options = RenderOptions::default();
    options.render.max_width = Some(320.);
    options.render.native_theme = Some(NativeTheme::default());
    let narrow = engine
        .render_document(&doc, &options, &mut MetricsResources)
        .unwrap();
    assert!(narrow.rendered.width < wide.rendered.width);
    assert_eq!(before, doc.model_view().unwrap());
    assert!(!narrow.rendered.svg.contains("var("));
    assert_eq!(narrow.rendered.links.len(), 1);
    assert!(!narrow.rendered.svg.contains("data-id=\"x\""));
    assert!(narrow.rendered.svg.contains("pele-collapsed-mark"));
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Workflow")));
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("a") && n.attribute("href") == Some("https://example.com/tool")));
    let tree = resvg::usvg::Tree::from_str(&narrow.rendered.svg, &Default::default()).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(narrow.rendered.width as u32, narrow.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 0).count() > 1000);
    assert!(engine
        .render_source(
            "agentflow-beta\nflow orphan\na --> b",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .is_err());
    let recovered = engine
        .parse(
            "agentflow-beta\nflow A\na --> B\nend\nflow B\nb --> A\nend\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(
        recovered
            .model::<AgentflowModel>()
            .unwrap()
            .diagnostics
            .len(),
        1
    );
    assert!(engine
        .render_document(&recovered, &RenderOptions::default(), &mut MetricsResources)
        .is_ok());
}
