//! Public feature-slice integration: sankey source preparation, parser and ordered models.
use pele::compat::{
    diagrams::sankey, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
fn fixture() -> &'static Value {
    static FIXTURE: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    FIXTURE.get_or_init(|| {
        use std::io::Read;
        let file = std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/sankey.json.gz"
        ))
        .unwrap();
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(file)
            .read_to_end(&mut bytes)
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    })
}
#[test]
fn pinned_sankey_parser_corpus() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(sankey::Sankey);
            engine
                .parse(case["source"].as_str().unwrap(), &options)
                .map(|d| serde_json::to_value(d.model_view().unwrap()).unwrap())
        } {
            Ok(model) => serde_json::json!({"model":model}),
            Err(error) => serde_json::json!({"error":error}),
        };
        let mut expected = case["expected"].clone();
        expected.as_object_mut().unwrap().remove("result");
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
fn pinned_sankey_complete_svg_corpus() {
    use pele::compat::{diagrams::sankey::Sankey, Engine, RenderOptions};
    let fixture = fixture();
    let mut engine = Engine::empty();
    engine.register(Sankey);
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
fn retained_sankey_reflow_colors_native_pixels_and_cycle_recovery() {
    use pele::compat::{
        diagrams::sankey::{Sankey, SankeyModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Sankey);
    let source="---\ntitle: Energy flows\nconfig:\n  sankey:\n    nodeAlignment: center\n    labelStyle: outlined\n    prefix: '$'\n    suffix: ' M'\n    linkColor: target\n    nodeColors:\n      Solar: '#123456'\n      Grid: '#ab3456'\n---\nsankey-beta\nSolar,Grid,25\nWind,Grid,35\nGrid,Homes,40\nGrid,Industry,20\n";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = doc.model::<SankeyModel>().unwrap();
    assert_eq!(model.nodes.len(), 5);
    assert_eq!(model.links.len(), 4);
    assert_eq!(model.title.as_deref(), Some("Energy flows"));
    assert_eq!(model.links[0].source.id, "Solar");
    assert_eq!(model.links[0].target.index, 1);
    assert_eq!(model.links[0].value, 25.);
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
    assert_eq!(doc.model_view().unwrap(), before);
    assert!(!narrow.rendered.svg.contains("var("));
    assert!(narrow.rendered.links.is_empty());
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    let named = |class: &str| {
        xml.descendants()
            .filter(|n| n.attribute("class") == Some(class))
            .count()
    };
    assert_eq!(named("pele-sankey-node"), 5);
    assert_eq!(named("pele-sankey-link"), 4);
    assert_eq!(named("pele-sankey-value"), 5);
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Solar → Grid: $25 M")));
    assert!(narrow.rendered.svg.contains("fill:#ab3456;"));
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
    let cycle = engine
        .parse("sankey\na,b,1\nb,c,1\nc,a,1", &ParseOptions::default())
        .unwrap();
    assert_eq!(cycle.model::<SankeyModel>().unwrap().links.len(), 3);
    let error = engine
        .render_document(&cycle, &RenderOptions::default(), &mut MetricsResources)
        .unwrap_err();
    assert!(error.message.contains("circular links"));
    let recovered = engine
        .render_source(
            "sankey\n__proto__,constructor,abc\nconstructor,toString,-5\n",
            &RenderOptions::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert!(recovered
        .rendered
        .svg
        .contains("__proto__ → constructor: 0"));
    assert!(!recovered.rendered.svg.contains("NaN"));
}
