//! Public feature-slice integration: swimlane source preparation, parser and ordered models.
use pele::compat::{
    diagrams::swimlane, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
            include_bytes!("../../../fixtures/pele/swimlane.json.gz").as_slice(),
        );
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    })
}
#[test]
fn pinned_swimlane_parser_corpus() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(swimlane::Swimlane);
            engine
                .parse(case["source"].as_str().unwrap(), &options)
                .map(|d| serde_json::to_value(d.model_view().unwrap()).unwrap())
        } {
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
fn pinned_swimlane_complete_svg_corpus() {
    use pele::compat::{diagrams::swimlane::Swimlane, Engine, RenderOptions};
    let fixture = fixture();
    let mut engine = Engine::empty();
    engine.register(Swimlane);
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
fn retained_swimlane_shared_flowchart_model_native_lanes_reflow_and_recovery() {
    use pele::compat::{
        diagrams::swimlane::{Swimlane, SwimlaneModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Swimlane);
    let source="---\ntitle: Service workflow\n---\nswimlane-beta LR\naccTitle: Service workflow\naccDescr: Cross lane handoffs\nsubgraph Customer\ndirection BT\nrequest[Request service]\nreceive[Receive update]\nend\nsubgraph Support\ntriage[Triage request]\nanswer[Send answer]\nend\nsubgraph Engineering\nsubgraph Investigation[Investigation]\ninvestigate[Investigate issue]\nfix[Prepare fix]\nend\nend\nrequest --> triage\ntriage -->|Known issue| answer\ntriage -->|Needs code change| investigate\ninvestigate --> fix --> answer\nanswer --> receive\nreceive --> outside\nclick request \"https://example.com/service\" \"Request help\" _blank\nclassDef hot fill:#123456,color:white\nclass request hot\n";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    assert_eq!(doc.diagram_type().as_str(), "swimlane");
    let model = doc.model::<SwimlaneModel>().unwrap();
    assert_eq!(model.subgraphs.len(), 4);
    assert_eq!(model.direction.as_deref(), Some("LR"));
    assert_eq!(
        model
            .subgraphs
            .iter()
            .find(|s| s.id == "Customer")
            .unwrap()
            .dir
            .as_deref(),
        Some("BT")
    );
    let before = doc.model_view().unwrap();
    let wide = engine
        .render_document(&doc, &RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let wide_xml = roxmltree::Document::parse(&wide.rendered.svg).unwrap();
    let lane_widths: Vec<_> = wide_xml
        .descendants()
        .filter(|n| {
            n.attribute("class")
                .is_some_and(|s| s.starts_with("pele-cluster pele-lane"))
        })
        .map(|g| {
            g.children()
                .find(|n| n.has_tag_name("rect"))
                .unwrap()
                .attribute("width")
                .unwrap()
                .parse::<f64>()
                .unwrap()
        })
        .collect();
    assert_eq!(lane_widths.len(), 4);
    assert!(lane_widths.iter().all(|w| *w == lane_widths[0]));
    assert!(wide.rendered.svg.contains("rotate(-90 "));
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
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Service workflow")));
    assert!(
        xml.descendants()
            .any(|n| n.has_tag_name("a")
                && n.attribute("href") == Some("https://example.com/service"))
    );
    assert!(narrow.rendered.svg.contains("__swimlane_default__"));
    assert!(narrow.rendered.svg.contains("fill:#123456;"));
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
            "swimlane-beta\nsubgraph missing\na --> b",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .is_err());
    let recovered = engine
        .render_source(
            "swimlane-beta TB\na --> b\n",
            &RenderOptions::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert!(recovered.rendered.svg.contains("pele-lane"));
}
