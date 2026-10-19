//! Complete public eventmodeling parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::event_modeling::EventModeling, text::MetricsResources, *};
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
fn all_pinned_eventmodeling_scenarios_match_model_svg_and_errors() {
    let mut text = String::new();
    flate2::read::GzDecoder::new(
        include_bytes!("../../../fixtures/pele/eventmodeling.json.gz").as_slice(),
    )
    .read_to_string(&mut text)
    .unwrap();
    let fixture: Json = serde_json::from_str(&text).unwrap();
    let mut engine = Engine::empty();
    engine.register(EventModeling);
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
fn eventmodeling_documents_reflow_lanes_data_notes_and_specifications_to_native_rails() {
    let mut engine = Engine::empty();
    engine.register(EventModeling);
    let source="eventmodeling title Checkout\ntf 01 ui Shop.View\ntf 02 cmd Shop.Update [[D]]\ntf 03 evt Shop.Updated\ntf 04 rmo Shop.Items ->> 01 ->> 02 ->> 03\nrf 05 processor Other.Index ->> 03\ndata D `json` {\n  product: 7\n  %% kept\n}\nnote 03 `md` {\n  Important detail\n}\ngwt 02 given evt Updated when cmd Update then rmo Items\n";
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
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
            .filter(|n| n.attribute("class") == Some("pele-edge"))
            .count(),
        6
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-em-note"))
            .count(),
        1
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-em-specification"))
            .count(),
        1
    );
    assert!(narrow.rendered.svg.contains("%% kept"));
    assert!(!narrow.rendered.svg.contains("var("));
    let mut svg_options = resvg::usvg::Options::default();
    svg_options
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec());
    svg_options.fontdb_mut().set_sans_serif_family("Noto Sans");
    svg_options.fontdb_mut().set_monospace_family("Noto Sans");
    svg_options.font_family = "Noto Sans".into();
    let tree = resvg::usvg::Tree::from_str(&narrow.rendered.svg, &svg_options).unwrap();
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
            .filter(|p| p.alpha() > 200 && p.blue() > p.red().saturating_add(10))
            .count()
            > 100
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
    let error = engine
        .parse("eventmodeling\ntf 01 evt A\ntf 01 cmd B", &options.parse)
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Semantic);
    assert_eq!(error.line, 3);
    assert!(engine
        .render_document(&document, &options, &mut MetricsResources)
        .is_ok());
}

#[test]
fn eventmodeling_retained_models_keep_shared_lane_and_relation_objects_after_document_drop() {
    use pele::compat::diagrams::event_modeling::EventModelingModel;
    use std::sync::Arc;
    let mut engine = Engine::empty();
    engine.register(EventModeling);
    let source="eventmodeling\ntf 01 cmd Cart.Add { x: 1 }\ntf 02 evt Cart.Added\ntf 03 rmo Cart.View ->> 01 ->> 02";
    let options = RenderOptions::default();
    let document = engine.parse(source, &options.parse).unwrap();
    let expected = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap()
        .rendered;
    let model = document.model::<EventModelingModel>().unwrap().clone();
    drop(document);
    drop(engine);
    assert!(Arc::ptr_eq(&model.boxes[0].lane, &model.boxes[2].lane));
    assert!(Arc::ptr_eq(&model.relations[0].source, &model.boxes[0]));
    assert!(Arc::ptr_eq(&model.relations[0].target, &model.boxes[1]));
    assert_eq!(
        EventModeling
            .render(
                &model,
                &Object::default(),
                &options.render,
                &mut MetricsResources
            )
            .unwrap(),
        expected
    );
}

#[test]
fn eventmodeling_utf16_output_ledger_still_preserves_complete_models() {
    let mut text = String::new();
    flate2::read::GzDecoder::new(
        include_bytes!("../../../fixtures/pele/eventmodeling.json.gz").as_slice(),
    )
    .read_to_string(&mut text)
    .unwrap();
    let fixture: Json = serde_json::from_str(&text).unwrap();
    let mut engine = Engine::empty();
    engine.register(EventModeling);
    let exceptions = fixture["extraction"]["utf16OutputExclusions"]
        .as_array()
        .unwrap();
    assert_eq!(exceptions.len(), 2);
    for case in exceptions {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let document = engine
            .parse(case["source"].as_str().unwrap(), &options.parse)
            .unwrap();
        assert_eq!(
            serde_json::to_value(document.model_view().unwrap()).unwrap(),
            case["model"]
        );
        assert!(case["svgUtf16"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unit| (0xd800..=0xdfff).contains(&(unit.as_u64().unwrap() as u16))));
    }
}
