//! Complete public journey parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::journey::Journey, text::MetricsResources, *};
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
fn all_pinned_journey_scenarios_match_model_svg_and_errors() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/journey.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Journey);
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
            let mut result = serde_json::to_value(result).unwrap();
            if case["expected"]["result"].get("svgSha256").is_some() {
                use sha2::{Digest, Sha256};
                result["svgSha256"] = Json::String(format!(
                    "{:x}",
                    Sha256::digest(result["svg"].as_str().unwrap().as_bytes())
                ));
                result.as_object_mut().unwrap().remove("svg");
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
fn journey_document_reuse_native_drawing_actors_and_limits() {
    use pele::compat::diagrams::journey::JourneyModel;
    let mut engine = Engine::empty();
    engine.register(Journey);
    let source = "journey\naccTitle: Work day\naccDescr: Tasks and people\nsection Work\nMake tea:5:Me,Cat\nRead <b>notes</b>:3:Cat\nsection Home\nRest:1:Me\n";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<JourneyModel>().unwrap();
    assert_eq!(model.actors(), vec!["Cat", "Me"]);
    assert_eq!(model.sections, vec!["Work", "Home"]);
    assert_eq!(model.current_section, "Home");
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    options.parse.preparation.limit = 0;
    for vertical in [false, true] {
        options.render.max_width = vertical.then_some(190.);
        let rendered = engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap();
        let xml = roxmltree::Document::parse(&rendered.rendered.svg).unwrap();
        assert_eq!(
            xml.descendants()
                .filter(|n| n.has_tag_name("g")
                    && n.attribute("class")
                        .is_some_and(|c| c.starts_with("pele-face ")))
                .count(),
            3
        );
        assert_eq!(
            xml.descendants()
                .filter(|n| n.has_tag_name("circle") && n.attribute("class") == Some("pele-actor"))
                .count(),
            4
        );
        assert!(xml
            .descendants()
            .any(|n| n.has_tag_name("title") && n.text() == Some("Work day")));
        assert!(xml
            .descendants()
            .any(|n| n.has_tag_name("desc") && n.text() == Some("Tasks and people")));
        for element in xml.descendants().filter(|n| n.is_element()) {
            for attribute in element.attributes() {
                if matches!(
                    attribute.name(),
                    "fill" | "stroke" | "style" | "font-family"
                ) {
                    assert!(!attribute.value().contains("var("));
                }
            }
        }
        let tree =
            resvg::usvg::Tree::from_str(&rendered.rendered.svg, &Default::default()).unwrap();
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
                > 100,
            "native output must paint actor dots"
        );
        assert!(rendered.rendered.svg.contains(if vertical {
            "pele-journey-step"
        } else {
            "pele-task"
        }));
    }
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
    // Actors follow UTF-16 code unit sorting, including astral characters preceding high BMP code points.
    let unicode = engine
        .parse("journey\nT:3:\u{e000},😀,A,😀,", &ParseOptions::default())
        .unwrap();
    assert_eq!(
        unicode.model::<JourneyModel>().unwrap().actors(),
        vec!["", "A", "😀", "\u{e000}"]
    );
}
