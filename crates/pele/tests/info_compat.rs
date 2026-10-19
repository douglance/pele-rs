//! Complete public info parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::info::Info, text::MetricsResources, *};
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
fn all_pinned_info_scenarios_match_model_svg_and_errors() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/info.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Info);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let source = case["source"].as_str().unwrap();
        let mut resources = Resources {
            host: case["host"].clone(),
            metrics: MetricsResources,
        };
        let mut parsed_model = None;
        let actual = (|| -> Result<Json, PeleError> {
            let document = engine.parse(source, &options.parse)?;
            let model = serde_json::to_value(document.model_view()?).unwrap();
            parsed_model = Some(model.clone());
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
            Err(e) => match parsed_model {
                Some(model) => serde_json::json!({"model":model,"error":e}),
                None => serde_json::json!({"error":e}),
            },
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
fn info_documents_preserve_metadata_reserve_icons_and_paint_native_version() {
    struct InformationResources {
        icons: usize,
    }
    impl RenderResources for InformationResources {
        fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
            MetricsResources.text_width(request)
        }
        fn icon(&mut self, _name: &str) -> Result<Option<TrustedSvg>, PeleError> {
            self.icons += 1;
            Ok(Some(TrustedSvg::from_trusted_markup(
                "<path id=\"trusted-icon\" d=\"M0 0L24 24\"/>".into(),
            )))
        }
    }
    let mut engine = Engine::empty();
    engine.register(Info);
    let source = "---\ntitle: Fallback\n---\ninfo showInfo\ntitle <b>Library</b><br>fa:fa-user Portable\naccTitle: Compatibility version\naccDescr { Native\n and web }";
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let document = engine.parse(source, &options.parse).unwrap();
    let before = serde_json::to_value(document.model_view().unwrap()).unwrap();
    let mut resources = InformationResources { icons: 0 };
    let rendered = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    assert!(resources.icons > 0);
    assert!(rendered.rendered.svg.contains("class=\"pele-icon\""));
    assert!(!rendered.rendered.svg.contains("trusted-icon"));
    assert!(rendered.rendered.svg.contains("Pele v1.6.0"));
    assert!(rendered.rendered.svg.contains("Compatibility version"));
    assert!(!rendered.rendered.svg.contains("Fallback"));
    assert!(!rendered.rendered.svg.contains("var("));
    let xml = roxmltree::Document::parse(&rendered.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-version"))
            .count(),
        1
    );
    let mut svg_options = resvg::usvg::Options::default();
    svg_options
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec());
    svg_options.font_family = "Noto Sans".into();
    svg_options.fontdb_mut().set_sans_serif_family("Noto Sans");
    let tree = resvg::usvg::Tree::from_str(&rendered.rendered.svg, &svg_options).unwrap();
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
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 200).count() > 100);
    options.render.max_width = Some(1.);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut resources)
            .unwrap(),
        rendered
    );
    assert_eq!(
        serde_json::to_value(document.model_view().unwrap()).unwrap(),
        before
    );
    options.render.output_limit = Some(10);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    options.render.output_limit = None;
    assert!(engine.parse("info unsupported", &options.parse).is_err());
    assert!(engine
        .render_document(&document, &options, &mut MetricsResources)
        .is_ok());
}
