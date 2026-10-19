//! Complete public block parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::block::Block, text::MetricsResources, *};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use std::io::Read;
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
fn all_pinned_block_scenarios_match_model_svg_and_errors() {
    let mut text = String::new();
    flate2::read::GzDecoder::new(include_bytes!("../../../fixtures/pele/block.json.gz").as_slice())
        .read_to_string(&mut text)
        .unwrap();
    let fixture: Json = serde_json::from_str(&text).unwrap();
    let mut engine = Engine::empty();
    engine.register(Block);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let source = case["source"].as_str().unwrap();
        if std::env::var("PELE_CASE")
            .ok()
            .is_some_and(|id| !case["id"].as_str().unwrap().contains(&id))
        {
            continue;
        }
        let mut resources = Resources {
            host: case["host"].clone(),
            metrics: MetricsResources,
        };
        let mut parsed_model = None;
        let actual = (|| -> Result<Json, PeleError> {
            let document = engine.parse(source, &options.parse)?;
            let model = if case["expected"]["modelProjectionLimit"] == true {
                assert!(
                    document.model_view().is_err(),
                    "{} expected bounded projection",
                    case["id"]
                );
                None
            } else {
                Some(serde_json::to_value(document.model_view()?).unwrap())
            };
            parsed_model = model.clone();
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
            Ok(if let Some(model) = model {
                serde_json::json!({"model":model,"result":result})
            } else {
                serde_json::json!({"modelProjectionLimit":true,"result":result})
            })
        })();
        let actual = match actual {
            Ok(v) => v,
            Err(e) => match parsed_model {
                Some(model) => serde_json::json!({"model":model,"error":e}),
                None => serde_json::json!({"error":e}),
            },
        };
        if actual != case["expected"] {
            let differing =
                if actual.get("error").is_some() || case["expected"].get("error").is_some() {
                    format!(
                        "actual error {:?}\nexpected error {:?}",
                        actual.get("error"),
                        case["expected"].get("error")
                    )
                } else {
                    let a = actual["result"]["svg"].as_str().unwrap_or("");
                    let b = case["expected"]["result"]["svg"].as_str().unwrap_or("");
                    let p = a
                        .bytes()
                        .zip(b.bytes())
                        .position(|(a, b)| a != b)
                        .unwrap_or(a.len().min(b.len()));
                    format!(
                        "models equal {}\nSVG first diff byte {p}: actual {:?} expected {:?}",
                        actual["model"] == case["expected"]["model"],
                        a.get(p.saturating_sub(40)..(p + 140).min(a.len())),
                        b.get(p.saturating_sub(40)..(p + 140).min(b.len()))
                    )
                };
            mismatches.push(format!(
                "{} source {}\n{}",
                case["id"],
                case["source"]
                    .as_str()
                    .unwrap()
                    .chars()
                    .take(300)
                    .collect::<String>(),
                differing
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
fn block_documents_reflow_nested_grids_route_links_and_paint_native_shapes() {
    let mut engine = Engine::empty();
    engine.register(Block);
    let source="---\ntitle: Independent library\n---\nblock\ncolumns 2\nblock:g[\"Shared core\"]:2\ncolumns 2\na[\"Native\"] b((\"Web\"))\nend\nc<[\"Extend\"]>(right) d{{\"Host\"}}\na -- \"Resources\" --> d\nc-->c\nclassDef hot fill:#00f,color:#fff\nclass a hot";
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let doc = engine.parse(source, &options.parse).unwrap();
    let before = serde_json::to_value(doc.model_view().unwrap()).unwrap();
    let wide = engine
        .render_document(&doc, &options, &mut MetricsResources)
        .unwrap();
    options.render.max_width = Some(120.);
    let narrow = engine
        .render_document(&doc, &options, &mut MetricsResources)
        .unwrap();
    assert!(narrow.rendered.width < wide.rendered.width);
    assert_eq!(
        serde_json::to_value(doc.model_view().unwrap()).unwrap(),
        before
    );
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-edge"))
            .count(),
        2
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-cluster"))
            .count(),
        1
    );
    assert!(!narrow.rendered.svg.contains("var("));
    assert!(!narrow.rendered.svg.contains("NaN"));
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
            .filter(|p| p.blue() > 200 && p.red() < 30)
            .count()
            > 100
    );
    options.render.output_limit = Some(10);
    assert_eq!(
        engine
            .render_document(&doc, &options, &mut MetricsResources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    options.render.output_limit = None;
    assert!(engine.parse("block\nend", &options.parse).is_err());
    assert!(engine
        .render_document(&doc, &options, &mut MetricsResources)
        .is_ok());
}

#[test]
fn deep_block_documents_keep_owned_models_after_document_and_engine_drop() {
    use pele::compat::diagrams::block::BlockModel;
    let mut engine = Engine::empty();
    engine.register(Block);
    let source = format!(
        "block\n{}leaf\n{}",
        "block\n".repeat(5000),
        "end\n".repeat(5000)
    );
    let mut options = RenderOptions::default();
    options.parse.preparation.limit = 1_000_000;
    let document = engine.parse(&source, &options.parse).unwrap();
    assert!(document.model_view().is_err());
    let expected = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap()
        .rendered;
    let owned = document.model::<BlockModel>().unwrap().clone();
    drop(document);
    drop(engine);
    let rendered = Block
        .render(
            &owned,
            &Object::default(),
            &options.render,
            &mut MetricsResources,
        )
        .unwrap();
    assert_eq!(rendered, expected);
    drop(owned);
}
