//! Complete public timeline parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::timeline::Timeline, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
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
fn all_pinned_timeline_scenarios_match_model_svg_and_errors() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/timeline.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Timeline);
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
            let mut result = serde_json::to_value(result).unwrap();
            if case["expected"]["result"].get("svgSha256").is_some() {
                let hash = format!(
                    "{:x}",
                    Sha256::digest(result["svg"].as_str().unwrap().as_bytes())
                );
                result.as_object_mut().unwrap().remove("svg");
                result["svgSha256"] = Json::String(hash);
            }
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
fn timeline_documents_reflow_preserve_events_and_paint_native_svg() {
    let mut engine = Engine::empty();
    engine.register(Timeline);
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let source = "timeline\ntitle Release history\naccTitle: Project milestones\nBefore : Idea\nsection First\n2025 : Prototype : Feedback\n2026 : Library\nsection Empty\nsection Next\n2027 : Extensions";
    let document = engine.parse(source, &options.parse).unwrap();
    let before = serde_json::to_value(document.model_view().unwrap()).unwrap();
    let wide = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    options.render.max_width = Some(320.);
    let narrow = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert!(narrow.rendered.width < wide.rendered.width);
    assert!(narrow.rendered.height > wide.rendered.height);
    assert_eq!(
        serde_json::to_value(document.model_view().unwrap()).unwrap(),
        before
    );
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-cluster pele-section"))
            .count(),
        3
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-node pele-period"))
            .count(),
        4
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-node pele-event"))
            .count(),
        5
    );
    assert!(narrow.rendered.svg.contains("Project milestones"));
    assert!(!narrow.rendered.svg.contains("var("));
    let tree = resvg::usvg::Tree::from_str(&narrow.rendered.svg, &Default::default()).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(narrow.rendered.width as u32, narrow.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 200).count() > 1000);
    assert!(engine
        .parse("timeline\n: orphan event", &options.parse)
        .is_err());
    assert!(engine
        .render_document(&document, &options, &mut MetricsResources)
        .is_ok());
    options.render.output_limit = Some(10);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
}
