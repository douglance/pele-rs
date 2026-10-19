//! Complete public kanban parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::kanban::Kanban, text::MetricsResources, *};
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
fn all_pinned_kanban_scenarios_match_model_svg_and_errors() {
    let fixture: Json =
        serde_json::from_str(include_str!("../../../fixtures/pele/kanban.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Kanban);
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
                format!(
                    "models actual {} expected {}\nSVG first diff byte {p}: actual {:?} expected {:?}",
                    actual["model"],
                    case["expected"]["model"],
                    a.get(p.saturating_sub(40)..(p + 140).min(a.len())),
                    b.get(p.saturating_sub(40)..(p + 140).min(b.len()))
                )
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
fn kanban_document_reuse_folded_columns_icons_links_native_raster_and_limits() {
    use pele::compat::diagrams::kanban::KanbanModel;
    let mut engine = Engine::empty();
    engine.register(Kanban);
    let source = "---\ntitle: Sprint\nconfig:\n  kanban:\n    ticketBaseUrl: 'https://example.test/#TICKET#'\n---\nkanban\n todo[Todo]\n ::icon(inbox)\n  a[**First** card]@{ ticket: T-1, assigned: Person, priority: 'Very High', icon: star }\n  b[Second]\n done[Done]\n  c[Third]\n later[Later]";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<KanbanModel>().unwrap();
    assert_eq!(model.sections.len(), 3);
    assert_eq!(model.sections[0].items.len(), 2);
    let mut resources = Resources {
        host: serde_json::json!({"icons":{"star":"<path d=\"M0 0L24 24\"/>","inbox":"<circle cx=\"12\" cy=\"12\" r=\"8\"/>"}}),
        metrics: MetricsResources,
    };
    let mut options = RenderOptions::default();
    options.parse.preparation.limit = 0;
    options.render.native_theme = Some(NativeTheme::default());
    let wide = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    options.render.max_width = Some(300.);
    let narrow = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    assert!(wide.rendered.width > narrow.rendered.width);
    assert!(wide.rendered.height < narrow.rendered.height);
    assert_eq!(
        narrow.rendered.links,
        vec![LinkInfo {
            id: "a".into(),
            href: "https://example.test/T-1".into(),
            internal: false
        }]
    );
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants()
            .filter(|n| n
                .attribute("class")
                .is_some_and(|c| c.starts_with("pele-cluster pele-column")))
            .count(),
        3
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n
                .attribute("class")
                .is_some_and(|c| c.starts_with("pele-node pele-card")))
            .count(),
        3
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-icon"))
            .count(),
        2
    );
    for node in xml.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            if matches!(attr.name(), "stroke" | "fill" | "style" | "font-family") {
                assert!(!attr.value().contains("var("));
            }
        }
    }
    let tree = resvg::usvg::Tree::from_str(&narrow.rendered.svg, &Default::default()).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(narrow.rendered.width as u32, narrow.rendered.height as u32)
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
            .filter(|p| p.alpha() > 100 && p.red() > 150 && p.green() < 130 && p.blue() < 130)
            .count()
            > 50,
        "priority stripe must paint red pixels"
    );
    assert_eq!(
        engine
            .render_source(source, &options, &mut resources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    options.render.links = Some(false);
    assert!(engine
        .render_document(&document, &options, &mut resources)
        .unwrap()
        .rendered
        .links
        .is_empty());
    options.render.output_limit = Some(20);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut resources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
}
