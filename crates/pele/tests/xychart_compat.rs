//! Public feature-slice integration: xychart source preparation, parser and ordered models.
use pele::compat::{
    diagrams::xychart, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
fn pinned_xychart_parser_corpus() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/xychart.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(xychart::XyChart);
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
fn pinned_xychart_complete_svg_corpus() {
    use pele::compat::{diagrams::xychart::XyChart, Engine, RenderOptions};
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/xychart.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(XyChart);
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
fn retained_xychart_reflow_clipping_native_pixels_and_recovery() {
    use pele::compat::{
        diagrams::xychart::{XAxis, XyChart, XyChartModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(XyChart);
    let source="xychart horizontal\naccTitle: Sales forecast\naccDescr: Actual and forecast values\ntitle \"Sales chart\"\nx-axis Month [January, February, March]\ny-axis Revenue -10 --> 20\nline Forecast [-30 \"Low\", 5, 40 \"High\"]\nbar Actual [-5, 10, 15]";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<XyChartModel>().unwrap();
    assert_eq!(model.plots.len(), 2);
    assert!(matches!(&model.x_axis,XAxis::Band{categories,..}if categories.len()==3));
    let before = document.model_view().unwrap();
    let wide = engine
        .render_document(&document, &RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    assert_eq!((wide.rendered.width, wide.rendered.height), (700., 500.));
    for max_width in [0., -5., f64::NAN, f64::INFINITY] {
        let mut options = RenderOptions::default();
        options.render.max_width = Some(max_width);
        assert_eq!(
            engine
                .render_document(&document, &options, &mut MetricsResources)
                .unwrap()
                .rendered
                .svg,
            wide.rendered.svg
        );
    }
    let mut options = RenderOptions::default();
    options.render.max_width = Some(320.);
    options.render.native_theme = Some(NativeTheme::default());
    options.parse.preparation.limit = 0;
    let narrow = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert_eq!(
        (narrow.rendered.width, narrow.rendered.height),
        (320., 300.)
    );
    assert_eq!(document.model_view().unwrap(), before);
    assert!(narrow.rendered.links.is_empty());
    assert!(!narrow.rendered.svg.contains("var("));
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Sales forecast")));
    let series: Vec<_> = xml
        .descendants()
        .filter(|n| {
            n.attribute("class")
                .is_some_and(|c| c.starts_with("pele-series "))
        })
        .collect();
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].attribute("data-id"), Some("Actual"));
    assert_eq!(series[1].attribute("data-id"), Some("Forecast"));
    assert_eq!(
        series[0]
            .children()
            .filter(|n| n.has_tag_name("rect"))
            .count(),
        3
    );
    assert_eq!(
        series[1]
            .children()
            .filter(|n| n.has_tag_name("circle"))
            .count(),
        1
    );
    let tree = resvg::usvg::Tree::from_str(&narrow.rendered.svg, &Default::default()).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(320, 300).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 0).count() > 1000);
    assert!(engine
        .render_source(
            "xychart\ntitle Empty",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .is_err());
    let recovered = engine
        .render_source(
            "xychart\nline [1,2]",
            &RenderOptions::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert!(recovered.rendered.svg.contains("pele-line"));
}
