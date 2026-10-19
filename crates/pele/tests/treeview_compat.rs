//! Complete public treeview parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::tree_view::TreeView, text::MetricsResources, *};
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
fn all_pinned_treeview_scenarios_match_model_svg_and_errors() {
    let mut text = String::new();
    flate2::read::GzDecoder::new(
        include_bytes!("../../../fixtures/pele/treeview.json.gz").as_slice(),
    )
    .read_to_string(&mut text)
    .unwrap();
    let fixture: Json = serde_json::from_str(&text).unwrap();
    let mut engine = Engine::empty();
    engine.register(TreeView);
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
fn treeview_documents_wrap_literal_names_resolve_icons_and_paint_native_guides() {
    let mut engine = Engine::empty();
    engine.register(TreeView);
    let source="treeView-beta\ntitle Project\naccTitle: Accessible files\n├── src/\n│   ├── VeryLongFileNameWithLiteral<Tags>AndWords.ts :::highlight ## Main module description\n│   └── child/ icon()\n│       └── index.rs icon(rust)\n└── README.md";
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    options.parse.preparation.config =
        serde_json::from_value(serde_json::json!({"treeView":{"showIcons":true}})).unwrap();
    let document = engine.parse(source, &options.parse).unwrap();
    let before = serde_json::to_value(document.model_view().unwrap()).unwrap();
    let wide = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    options.render.max_width = Some(180.);
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
            .filter(|n| n.attribute("class") == Some("pele-tree-highlight"))
            .count(),
        1
    );
    assert!(narrow.rendered.svg.contains("Accessible files"));
    assert!(narrow.rendered.svg.contains("&lt;Tags&gt;"));
    assert!(!narrow.rendered.svg.contains("var("));
    let mut svg_options = resvg::usvg::Options::default();
    svg_options
        .fontdb_mut()
        .load_font_data(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec());
    svg_options.fontdb_mut().set_sans_serif_family("Noto Sans");
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
            .filter(|p| p.alpha() > 20 && p.blue() > p.red())
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
        .parse(
            "treeView-beta\n│\n├── src/\n│\n│   └── \"a\n",
            &options.parse,
        )
        .err()
        .unwrap();
    assert_eq!(error.line, 5);
    assert!(error.message.contains("line 5"));
    assert!(engine
        .render_document(&document, &options, &mut MetricsResources)
        .is_ok());
}

#[test]
fn treeview_owned_deep_models_survive_document_and_engine_drop() {
    use pele::compat::diagrams::tree_view::TreeViewModel;
    let source = format!(
        "treeView-beta\n{}",
        (0..300)
            .map(|i| format!("{}directory/\n", " ".repeat(i)))
            .collect::<String>()
    );
    let mut engine = Engine::empty();
    engine.register(TreeView);
    let options = RenderOptions::default();
    let document = engine.parse(&source, &options.parse).unwrap();
    assert!(document.model_view().is_err());
    let expected = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap()
        .rendered;
    let model = document.model::<TreeViewModel>().unwrap().clone();
    drop(document);
    drop(engine);
    assert_eq!(
        TreeView
            .render(
                &model,
                &Object::default(),
                &options.render,
                &mut MetricsResources
            )
            .unwrap(),
        expected
    );
    drop(model);
}

#[test]
fn treeview_resolvers_observe_each_row_and_one_inline_title_lookup_per_render() {
    struct Host {
        calls: Vec<String>,
    }
    impl RenderResources for Host {
        fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
            MetricsResources.text_width(request)
        }
        fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
            self.calls.push(name.into());
            Ok(Some(TrustedSvg::from_trusted_markup(format!(
                "<path data-call=\"{}\" d=\"M0 0L24 24\"/>",
                self.calls.len()
            ))))
        }
    }
    let mut engine = Engine::empty();
    engine.register(TreeView);
    let source="treeView-beta\ntitle fa:fa-user fa:fa-user\na icon(item)\nb icon(item)\nc icon(none)\nd icon()\ne icon(item)";
    let mut options = RenderOptions::default();
    options.parse.preparation.config =
        serde_json::from_value(serde_json::json!({"treeView":{"defaultIconPack":"pack&#35;"}}))
            .unwrap();
    let document = engine.parse(source, &options.parse).unwrap();
    let mut host = Host { calls: vec![] };
    let result = engine
        .render_document(&document, &options, &mut host)
        .unwrap();
    assert_eq!(
        host.calls,
        ["fa:fa-user", "pack#:item", "pack#:item", "pack#:item"]
    );
    for call in 2..=4 {
        assert!(result
            .rendered
            .svg
            .contains(&format!("data-call=\"{call}\"")));
    }
    assert!(!result.rendered.svg.contains("data-call=\"1\""));
    host.calls.clear();
    assert_eq!(
        engine
            .render_document(&document, &options, &mut host)
            .unwrap(),
        result
    );
    assert_eq!(
        host.calls,
        ["fa:fa-user", "pack#:item", "pack#:item", "pack#:item"]
    );
}
