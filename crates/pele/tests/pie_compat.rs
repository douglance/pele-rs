//! Complete public pie parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::pie::Pie, text::MetricsResources, *};
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
fn all_pinned_pie_scenarios_match_model_svg_and_errors() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/pie.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Pie);
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
fn native_export_resolves_palette_without_changing_authored_text() {
    let engine = Engine::with_builtins();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let source = "pie title Text style='var(--pele-bg)'\n\"style='var(--_fg)'\":1\n\"Other\":2";
    let diagram = engine
        .render_source(source, &options, &mut MetricsResources)
        .unwrap();
    assert!(diagram.rendered.svg.contains("Text style='var(--pele-bg)'"));
    assert!(diagram
        .rendered
        .svg
        .contains("data-id=\"style='var(--_fg)'\""));
    let xml = roxmltree::Document::parse(&diagram.rendered.svg).unwrap();
    for text in xml.descendants().filter(|n| n.has_tag_name("text")) {
        if let Some(span) = text
            .first_element_child()
            .filter(|n| n.has_tag_name("tspan"))
        {
            assert_eq!(text.attribute("x"), span.attribute("x"));
            assert_eq!(text.attribute("y"), span.attribute("y"));
        }
    }
    for node in xml.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            if matches!(
                attr.name(),
                "fill" | "stroke" | "font-family" | "font-weight" | "style"
            ) {
                assert!(
                    !attr.value().contains("var("),
                    "unresolved native attribute: {}",
                    attr.value()
                );
            }
        }
    }
    let tree = resvg::usvg::Tree::from_str(&diagram.rendered.svg, &Default::default()).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(
        diagram.rendered.width as u32,
        diagram.rendered.height as u32,
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
        "native export must paint the blue series"
    );
    options.render.native_theme.as_mut().unwrap().foreground = "red;url(x)".into();
    assert_eq!(
        engine
            .render_source("pie", &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Semantic
    );
}
