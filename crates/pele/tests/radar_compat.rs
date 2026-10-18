//! Complete public radar parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::radar::Radar, text::MetricsResources, *};
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
fn all_pinned_radar_scenarios_match_model_svg_and_errors() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/radar.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Radar);
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
fn radar_document_reuse_native_pixels_limits_and_recovery() {
    let mut engine = Engine::empty();
    engine.register(Radar);
    let source="---\nconfig:\n  radar:\n    width: 280\n    curveTension: 0.5\n---\nradar-beta\ntitle Radar reuse\naccTitle: Four skills\naccDescr: Two ordered curves\naxis a[\"Top\"],b[\"Right\"],c[\"Bottom\"],d[\"Left\"]\ncurve one{1,2,3,4},two{4,3,2,1}";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document
        .model::<pele::compat::diagrams::radar::RadarModel>()
        .unwrap();
    assert_eq!(model.axes.len(), 4);
    assert_eq!(model.curves[0].entries, vec![1., 2., 3., 4.]);
    let mut options = RenderOptions::default();
    options.parse.preparation.limit = 0;
    options.render.max_width = Some(220.);
    options.render.native_theme = Some(NativeTheme::default());
    let diagram = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert!(diagram.rendered.svg.contains("Four skills"));
    assert!(diagram.rendered.svg.contains("Two ordered curves"));
    let xml = roxmltree::Document::parse(&diagram.rendered.svg).unwrap();
    for node in xml.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes().filter(|a| {
            matches!(
                a.name(),
                "fill" | "stroke" | "font-family" | "font-weight" | "style"
            )
        }) {
            assert!(
                !attr.value().contains("var("),
                "unresolved native attribute {}",
                attr.value()
            );
        }
    }
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-curve"))
            .count(),
        2
    );
    let tree = resvg::usvg::Tree::from_str(&diagram.rendered.svg, &Default::default()).unwrap();
    let mut image = resvg::tiny_skia::Pixmap::new(
        diagram.rendered.width as u32,
        diagram.rendered.height as u32,
    )
    .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut image.as_mut(),
    );
    assert!(
        image
            .pixels()
            .iter()
            .filter(|p| p.alpha() > 30 && p.blue() > p.red())
            .count()
            > 1000,
        "native radar must paint actual colored curves"
    );
    options.render.output_limit = Some(32);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    options.render.output_limit = None;
    assert_eq!(
        engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap(),
        diagram
    );
    assert_eq!(
        engine
            .render_source(source, &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert!(engine
        .parse("radar-beta\naxis A\ncurve c{A 3}", &ParseOptions::default())
        .is_ok());
}
