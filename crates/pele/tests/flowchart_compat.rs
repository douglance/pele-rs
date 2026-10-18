//! Public feature-slice integration: flowchart source preparation, parser and ordered models.
use pele::compat::{
    diagrams::flowchart, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
#[test]
fn pinned_flowchart_parser_corpus() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/flowchart.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match flowchart::parse(case["source"].as_str().unwrap(), &options) {
            Ok(model) => serde_json::json!({"model":model}),
            Err(error) => serde_json::json!({"error":error}),
        };
        let mut expected = case["expected"].clone();
        expected.as_object_mut().unwrap().remove("result");
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
fn pinned_flowchart_complete_svg_corpus() {
    use pele::compat::{diagrams::flowchart::Flowchart, Engine, RenderOptions};
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/flowchart.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Flowchart);
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match engine.render_source(
            case["source"].as_str().unwrap(),
            &options,
            &mut Host(&case["host"]),
        ) {
            Ok(result) => serde_json::to_value(result).unwrap(),
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
fn owned_document_reuse_links_and_native_pixels() {
    use pele::compat::{
        diagrams::flowchart::Flowchart, text::MetricsResources, Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Flowchart);
    let source="flowchart LR\naccTitle: Order processing\naccDescr: A grouped decision with a retained link\nsubgraph client[Client]\nA[Start] --> B{Ready?}\nend\nB -->|yes| C[(Store)]\nB -->|no| A\nclassDef blue fill:#247acd,color:#ffffff,stroke:#164779\nclass A blue\nclick A \"https://example.com/orders\" \"Order details\" _blank";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    // Reuse retains its parse configuration; render-only changes never reparse or retire the model.
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    let native = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert_eq!(native.rendered.links.len(), 1);
    assert_eq!(native.rendered.links[0].id, "A");
    assert_eq!(native.rendered.links[0].href, "https://example.com/orders");
    assert!(!native.rendered.svg.contains("color-mix("));
    let xml = roxmltree::Document::parse(&native.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("a") && n.attribute("href") == Some("https://example.com/orders")));
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Order processing")));
    assert_eq!(
        xml.descendants()
            .filter(|n| n
                .attribute("class")
                .is_some_and(|c| c.starts_with("pele-node ")))
            .count(),
        3
    );
    let tree = resvg::usvg::Tree::from_str(&native.rendered.svg, &Default::default()).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(native.rendered.width as u32, native.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(
        pixels
            .pixels()
            .iter()
            .filter(|p| p.alpha() > 200 && p.blue() > 150 && p.red() < 80)
            .count()
            > 1000,
        "native SVG must paint the authored blue node"
    );
    options.render.max_width = Some(200.);
    let narrow = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert!(narrow.rendered.width < native.rendered.width);
    assert_eq!(narrow.rendered.links, native.rendered.links);
}

#[test]
fn pinned_large_typed_document_parser() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/flowchart.json")).unwrap();
    for case in fixture["generatedStress"].as_array().unwrap() {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let model = flowchart::parse(case["source"].as_str().unwrap(), &options).unwrap();
        assert_eq!(
            model.nodes.len() as u64,
            case["expected"]["nodes"].as_u64().unwrap()
        );
        assert_eq!(
            model.edges.len() as u64,
            case["expected"]["edges"].as_u64().unwrap()
        );
        assert_eq!(
            model.edges[0].edge_type.as_deref(),
            case["expected"]["firstEdgeType"].as_str()
        );
    }
}
