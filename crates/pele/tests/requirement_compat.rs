//! Public feature-slice integration: requirement source preparation, parser and ordered models.
use pele::compat::{
    diagrams::requirement, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
fn pinned_requirement_parser_corpus() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/requirement.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(requirement::RequirementDiagram);
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
fn pinned_requirement_complete_svg_corpus() {
    use pele::compat::{diagrams::requirement::RequirementDiagram, Engine, RenderOptions};
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/requirement.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(RequirementDiagram);
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
fn retained_requirement_tables_markers_native_reflow_and_error_recovery() {
    use pele::compat::{
        diagrams::requirement::{RequirementDiagram, RequirementModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(RequirementDiagram);
    let source="---\ntitle: Requirements\n---\nrequirementDiagram\naccTitle: System requirements\naccDescr: Verification and containment\ndirection LR\nrequirement r1 {\nid: 7\ntext: \"**Critical** behavior\"\nrisk: high\nverifymethod: test\n}\ndesignConstraint r2 {\ntext: limit\n}\nelement e {\ntype: document\ndocRef: manual\n}\ne - satisfies -> r1\nr1 - contains -> r2\nr2 - traces -> r2\nmissing - refines -> r1\nclassDef hot fill:#123456,color:white\nclass r1 hot\n";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = doc.model::<RequirementModel>().unwrap();
    assert_eq!(model.requirements.len(), 2);
    assert_eq!(model.elements.len(), 1);
    assert_eq!(model.relations.len(), 4);
    assert_eq!(model.requirements[0].1.risk, "High");
    assert_eq!(model.requirements[0].1.verify_method, "Test");
    assert_eq!(
        model.requirements[1].1.requirement_type,
        "Design Constraint"
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
    assert!(narrow.rendered.links.is_empty());
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants()
            .filter(|n| n
                .attribute("class")
                .is_some_and(|c| c.starts_with("pele-edge pele-relationship-")))
            .count(),
        3
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.has_tag_name("circle") && n.attribute("class") == Some("pele-marker"))
            .count(),
        1
    );
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("System requirements")));
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
            "requirementDiagram\nrequirement bad {\nrisk: invalid\n}\n",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .is_err());
    let recovered=engine.render_source("requirementDiagram\nrequirement __proto__ {\nid: first\nid: second\n}\nelement __proto__ {\ntype: collision\n}\n__proto__ - contains -> __proto__\n",&RenderOptions::default(),&mut MetricsResources).unwrap();
    assert_eq!(
        recovered
            .rendered
            .svg
            .matches("data-id=\"__proto__\" transform=")
            .count(),
        2
    );
    assert!(recovered.rendered.svg.contains(">first</tspan>"));
    assert!(!recovered.rendered.svg.contains(">second</tspan>"));
}
