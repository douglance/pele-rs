//! Complete public quadrant parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::quadrant_chart::QuadrantChart, text::MetricsResources, *};
use serde_json::Value as Json;
struct Resources {
    host: Json,
    metrics: MetricsResources,
}
impl RenderResources for Resources {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        if self.host["measure"] == "fixed" {
            Ok(request.text.encode_utf16().count() as f64
                * request.size
                * if request.style & 1 != 0 { 0.71 } else { 0.6 })
        } else {
            self.metrics.text_width(request)
        }
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        Ok(self.host["icons"][name]
            .as_str()
            .map(|s| TrustedSvg::from_trusted_markup(s.into())))
    }
}
#[test]
fn all_pinned_quadrant_scenarios_match_model_svg_and_errors() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/quadrant.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(QuadrantChart);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let source = case["source"].as_str().unwrap();
        let mut resources = Resources {
            host: case["host"].clone(),
            metrics: MetricsResources,
        };
        let actual = (|| -> Result<Json, PeleError> {
            let document = engine.parse(source, &options.parse)?;
            let model = serde_json::to_value(document.model_view()?).unwrap();
            let result = engine.render_document(&document, &options, &mut resources)?;
            roxmltree::Document::parse(&result.rendered.svg)
                .expect("successful SVG is well-formed XML");
            let result = serde_json::to_value(result).unwrap();
            Ok(serde_json::json!({"model":model,"result":result}))
        })();
        let actual = match actual {
            Ok(v) => v,
            Err(e) => serde_json::json!({"error":e}),
        };
        if actual != case["expected"] {
            let differing = if actual.get("error").is_some()
                || case["expected"].get("error").is_some()
            {
                format!("actual {actual}\nexpected {}", case["expected"])
            } else {
                let a = actual["result"]["svg"].as_str().unwrap_or("");
                let b = case["expected"]["result"]["svg"].as_str().unwrap_or("");
                let p = a
                    .bytes()
                    .zip(b.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(a.len().min(b.len()));
                format!("models actual {} expected {}\nSVG first diff byte {p}: actual {:?} expected {:?}",actual["model"],case["expected"]["model"],a.get(p.saturating_sub(40)..(p+140).min(a.len())),b.get(p.saturating_sub(40)..(p+140).min(b.len())))
            };
            mismatches.push(format!(
                "{} source {}\n{}",
                case["id"], case["source"], differing
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n{}",
        mismatches.len(),
        mismatches
            .into_iter()
            .take(25)
            .collect::<Vec<_>>()
            .join("\n\n")
    );
}

#[test]
fn quadrant_native_export_document_reuse_and_limits() {
    let mut engine = Engine::empty();
    engine.register(QuadrantChart);
    let source="quadrantChart\naccTitle: Portfolio\nx-axis Low --> High\ny-axis Low --> High\nquadrant-1 Promote\nA:::c: [0.25,0.75] color: abc, radius: 15\nB: [0.75,0.25] radius: 20\nclassDef c stroke-width: 4px, stroke-color: f00";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    options.parse.preparation.limit = 0;
    let rendered = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    let xml = roxmltree::Document::parse(&rendered.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants()
            .filter(|n| n.has_tag_name("circle"))
            .count(),
        2
    );
    assert_eq!(
        xml.descendants().filter(|n| n.has_tag_name("rect")).count(),
        4
    );
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Portfolio")));
    for n in xml.descendants().filter(|n| n.is_element()) {
        for a in n.attributes() {
            if matches!(a.name(), "fill" | "stroke" | "style" | "font-family") {
                assert!(!a.value().contains("var("));
            }
        }
    }
    let tree = resvg::usvg::Tree::from_str(&rendered.rendered.svg, &Default::default()).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(
        rendered.rendered.width as u32,
        rendered.rendered.height as u32,
    )
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
            .filter(|p| p.alpha() > 200 && p.blue() > 100 && p.red() < 100)
            .count()
            > 1000,
        "native renderer must paint second blue point"
    );
    assert_eq!(
        engine
            .render_source(source, &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    options.render.output_limit = Some(20);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let projected = serde_json::to_string(&document.model_view().unwrap()).unwrap();
    assert!(
        projected.contains("\"className\":\"c\",\"color\":\"abc\",\"radius\":15"),
        "style declaration property order must survive model projection"
    );
    assert!(projected.contains("[\"c\",{\"strokeWidth\":\"4px\",\"strokeColor\":\"f00\"}]"));
}
