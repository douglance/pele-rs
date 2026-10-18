//! Public feature-slice integration: treemap source preparation, parser and ordered models.
use pele::compat::{
    diagrams::treemap, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
};
struct Host<'a>(&'a Value);
impl RenderResources for Host<'_> {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        if self.0["measure"] == "fixed" {
            Ok(request.text.encode_utf16().count() as f64
                * request.size
                * if request.style & 1 != 0 { 0.71 } else { 0.6 })
        } else {
            pele::compat::text::MetricsResources.text_width(request)
        }
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        Ok(self.0["icons"][name]
            .as_str()
            .map(|s| TrustedSvg::from_trusted_markup(s.into())))
    }
}
use serde_json::Value;
#[test]
fn pinned_treemap_parser_corpus() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/treemap.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(treemap::Treemap);
            engine
                .parse(case["source"].as_str().unwrap(), &options)
                .map(|d| {
                    if case["expected"]["modelProjectionLimit"] == true {
                        assert!(d.model_view().is_err());
                        serde_json::json!({"projectionLimit":true})
                    } else {
                        serde_json::to_value(d.model_view().unwrap()).unwrap()
                    }
                })
        } {
            Ok(model) => serde_json::json!({"model":model}),
            Err(error) => serde_json::json!({"error":error}),
        };
        let mut expected = case["expected"].clone();
        expected.as_object_mut().unwrap().remove("result");
        expected
            .as_object_mut()
            .unwrap()
            .remove("upstreamModelSha256");
        if expected.get("modelProjectionLimit") == Some(&Value::Bool(true)) {
            expected = serde_json::json!({"model":{"projectionLimit":true}});
        }
        if expected.get("model").is_some() {
            expected.as_object_mut().unwrap().remove("error");
        }
        if actual != expected {
            if failures.len() < 10 {
                eprintln!(
                    "{} source={:?}\nactual={}\nexpected={}",
                    case["id"], case["source"], actual, expected
                )
            }
            failures.push(case["id"].clone())
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} mismatches: {:?}",
        failures.len(),
        cases.len(),
        failures.get(..10.min(failures.len())).unwrap()
    );
}

#[test]
fn pinned_treemap_complete_svg_corpus() {
    use pele::compat::{diagrams::treemap::Treemap, Engine, RenderOptions};
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/treemap.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Treemap);
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match engine.render_source(
            case["source"].as_str().unwrap(),
            &options,
            &mut Host(&case["host"]),
        ) {
            Ok(result) => {
                let mut v = serde_json::to_value(result).unwrap();
                if case["expected"]["result"].get("svgSha256").is_some() {
                    use sha2::{Digest, Sha256};
                    let svg = v.as_object_mut().unwrap().remove("svg").unwrap();
                    v["svgSha256"] =
                        format!("{:x}", Sha256::digest(svg.as_str().unwrap().as_bytes())).into();
                }
                v
            }
            Err(error) => serde_json::json!({"error":error}),
        };
        let expected = if case["expected"].get("error").is_some() {
            serde_json::json!({"error":case["expected"]["error"]})
        } else {
            case["expected"]["result"].clone()
        };
        if actual != expected {
            if failures.len() < 6 {
                eprintln!(
                    "{} source={:?}\nactual={}\nexpected={}",
                    case["id"], case["source"], actual, expected
                )
            }
            failures.push(case["id"].clone());
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} SVG mismatches: {:?}",
        failures.len(),
        cases.len(),
        failures.get(..10.min(failures.len())).unwrap()
    );
}

#[test]
fn retained_treemap_hierarchy_styles_reflow_native_pixels_and_recovery() {
    use pele::compat::{
        diagrams::treemap::{Treemap, TreemapModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Treemap);
    let source="---\ntitle: Budget\nconfig:\n  treemap:\n    valueFormat: '$0,0'\n---\ntreemap-beta\naccTitle: Spending map\naccDescr: Planned spending\n\"Operations\":::frame\n  \"Salaries\":700000:::hot\n  \"Equipment\":200000\n\"Reserve\":100000\nclassDef frame fill:#eee,stroke-width:2px\nclassDef hot fill:#123456,color:white\n";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = doc.model::<TreemapModel>().unwrap();
    assert_eq!(model.roots.len(), 2);
    assert_eq!(model.classes.len(), 2);
    assert_eq!(model.roots[0].children.as_ref().unwrap().len(), 2);
    assert_eq!(
        model.roots[0].children.as_ref().unwrap()[0].value,
        Some(700000.)
    );
    let before = doc.model_view().unwrap();
    let wide = engine
        .render_document(&doc, &RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut options = RenderOptions::default();
    options.render.max_width = Some(320.);
    options.render.native_theme = Some(NativeTheme::default());
    let narrow = engine
        .render_document(&doc, &options, &mut MetricsResources)
        .unwrap();
    assert!(narrow.rendered.width < wide.rendered.width);
    assert_eq!(narrow.rendered.width, 320.);
    assert_eq!(doc.model_view().unwrap(), before);
    assert!(!narrow.rendered.svg.contains("var("));
    assert!(narrow.rendered.links.is_empty());
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    assert_eq!(
        xml.descendants().filter(|n| n.has_tag_name("rect")).count(),
        4
    );
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Spending map")));
    assert!(narrow.rendered.svg.contains("$700,000"));
    assert!(narrow.rendered.svg.contains("fill:#123456;"));
    let tree = resvg::usvg::Tree::from_str(&narrow.rendered.svg, &Default::default()).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(narrow.rendered.width as u32, narrow.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 0).count() > 1000);
    assert!(engine
        .render_source(
            "treemap\n\"a\":-1",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .is_err());
    let recovered=engine.render_source("treemap\n\"__proto__\":::__proto__\n \"constructor\":1:::constructor\nclassDef __proto__ fill:red\nclassDef constructor fill:blue",&RenderOptions::default(),&mut MetricsResources).unwrap();
    assert!(recovered.rendered.svg.contains("pele-leaf constructor"));
    assert!(!recovered.rendered.svg.contains("NaN"));
}
#[test]
fn deep_treemap_keeps_typed_hierarchy_and_render_beyond_json_budget() {
    use pele::compat::{
        diagrams::treemap::{Treemap, TreemapModel},
        text::MetricsResources,
        Engine, ErrorCode, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Treemap);
    let mut source = "treemap\n".to_string();
    for i in 0..300 {
        source.push_str(&format!("{}\"s{i}\"\n", " ".repeat(i)));
    }
    source.push_str(&format!("{}\"leaf\":1", " ".repeat(300)));
    let doc = engine.parse(&source, &ParseOptions::default()).unwrap();
    let model = doc.model::<TreemapModel>().unwrap();
    let mut node = &model.roots[0];
    let mut depth = 0;
    while let Some(children) = &node.children {
        node = &children[0];
        depth += 1;
    }
    assert_eq!(depth, 300);
    assert_eq!(node.value, Some(1.));
    assert_eq!(doc.model_view().unwrap_err().code, ErrorCode::Limit);
    let rendered = engine
        .render_document(&doc, &RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    assert!(rendered.rendered.svg.contains("data-id=\"s0\""));
    assert!(!rendered.rendered.svg.contains("NaN"));
}
