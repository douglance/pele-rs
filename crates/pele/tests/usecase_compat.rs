//! Public feature-slice integration: usecase source preparation, parser and ordered models.
use pele::compat::{
    diagrams::usecase, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
            include_bytes!("../../../fixtures/pele/usecase.json.gz").as_slice(),
        );
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    })
}
#[test]
fn pinned_usecase_parser_corpus() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(usecase::Usecase);
            engine
                .parse(case["source"].as_str().unwrap(), &options)
                .map(|d| {
                    if case["expected"]["modelProjectionLimit"] == true {
                        assert!(d.model_view().is_err());
                        serde_json::json!({"projectionLimit":true})
                    } else {
                        serde_json::to_value(d.model_view().unwrap()).unwrap()
                    }
                })
        } {
            Ok(model) => serde_json::json!({"model":model}),
            Err(error) => serde_json::json!({"error":error}),
        };
        let mut expected = case["expected"].clone();
        expected.as_object_mut().unwrap().remove("result");
        expected.as_object_mut().unwrap().remove("projectionBudget");
        if expected["modelProjectionLimit"] == true {
            expected = serde_json::json!({"model":{"projectionLimit":true}})
        }
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
fn pinned_usecase_complete_svg_corpus() {
    use pele::compat::{diagrams::usecase::Usecase, Engine, RenderOptions};
    let fixture = fixture();
    let mut engine = Engine::empty();
    engine.register(Usecase);
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
fn retained_usecase_json_actor_boundaries_native_reflow_and_recovery() {
    use pele::compat::{
        diagrams::usecase::{flatten_json_rows, Usecase, UsecaseModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Usecase);
    let source="---\ntitle: Use cases\n---\nusecase-beta\naccTitle: Checkout workflow\naccDescr: Actors and payload\nactor customer(\"Customer\")@{ type: hollow, business: true } <<Person>>\nactor service(\"Service\")@{ type: awesome }\nactor icon(\"Icon\")@{ icon: \"fa:user\" }\nsystemBoundary store[\"Store\"]@{ type: package }:::hot\ncheckout(\"Checkout\") <<Core>>\ninspect[\"Inspect\"]\nend\ncustomer starts@-- \"buy\" ---> checkout\ncheckout ..> : include inspect\ncheckout -- retry --> checkout\nservice --|> customer\njson data@{\"2\":\"b\",\"1\":\"a\",\"items\":[1,2],\"nested\":{\"old\":1},\"nested\":{\"new\":true}}\ninspect --> data\nnote for checkout \"Remember\"\nstarts@{ animate: false, animation: slow }\nclassDef hot fill:#123456,color:white\n";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = doc.model::<UsecaseModel>().unwrap();
    assert_eq!(model.actors.len(), 3);
    assert_eq!(model.system_boundaries[0].1.boundary_type, "package");
    assert_eq!(model.relationships[0].animation.as_deref(), Some("slow"));
    assert!(model.relationships[0].animate);
    let json = &model.json_nodes[0].1;
    let rows = flatten_json_rows(&json.value, &json.property_order);
    assert_eq!(
        rows.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(),
        vec!["2", "1", "items", "", "nested.new"]
    );
    assert_eq!(
        model
            .statements
            .iter()
            .filter(|s| s.kind == "edgeMetadata")
            .count(),
        1
    );
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
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Checkout workflow")));
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-actor-figure"))
            .count(),
        3
    );
    assert!(narrow.rendered.svg.contains("pele-boundary-tab"));
    assert!(narrow.rendered.svg.contains("pele-json-key"));
    assert!(narrow.rendered.svg.contains("stroke-dasharray=\"9 5\""));
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
        .parse("usecase-beta\nactor a\na(x)\n", &ParseOptions::default())
        .is_err());
    let recovered = engine
        .render_source(
            "usecase-beta\nactor a@{ type: normal }\na --> b\n",
            &RenderOptions::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert!(recovered.rendered.svg.contains("pele-actor"));
    let deep = format!(
        "usecase-beta\njson deep@{}1{}",
        "{\"a\":".repeat(200),
        "}".repeat(200)
    );
    let doc = engine.parse(&deep, &ParseOptions::default()).unwrap();
    assert!(doc.model_view().is_err());
    assert_eq!(
        doc.model::<UsecaseModel>().unwrap().json_nodes[0]
            .1
            .value
            .depth,
        201
    );
    assert!(engine
        .render_document(&doc, &RenderOptions::default(), &mut MetricsResources)
        .unwrap()
        .rendered
        .svg
        .contains("pele-json"));
}
