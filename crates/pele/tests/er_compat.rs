//! Public feature-slice integration: er source preparation, parser and ordered models.
use pele::compat::{
    diagrams::er, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
fn pinned_er_parser_corpus() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/er.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(er::Er);
            engine
                .parse(case["source"].as_str().unwrap(), &options)
                .map(|d| serde_json::to_value(d.model::<er::ErModel>().unwrap()).unwrap())
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
fn pinned_er_complete_svg_corpus() {
    use pele::compat::{diagrams::er::Er, Engine, RenderOptions};
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/er.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(Er);
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
fn retained_er_documents_native_tables_links_and_recovery() {
    use pele::compat::{
        diagrams::er::{Er, ErModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Er);
    let source="---\ntitle: Customer records\n---\nerDiagram\naccTitle: Customer schema\naccDescr: Accounts and their orders\nsubgraph accounts[Accounts]\nCustomer:::internal-link {\n int id PK\n string displayName \"Customer name\"\n List~string~ tags\n}\nend\nCustomer ||--o{ Order : places\nCustomer ||--|| Customer : refers";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<ErModel>().unwrap();
    assert_eq!(model.entities.len(), 2);
    assert_eq!(model.entities[0].1.attributes.len(), 3);
    assert_eq!(model.subgraphs[0].nodes, vec!["Customer"]);
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    let result = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert_eq!(result.rendered.links.len(), 1);
    assert_eq!(result.rendered.links[0].href, "Customer");
    let xml = roxmltree::Document::parse(&result.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Customer schema")));
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("a") && n.attribute("href") == Some("Customer")));
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-er-stripes"))
            .count(),
        1
    );
    assert_eq!(
        xml.descendants()
            .filter(|n| n.attribute("class") == Some("pele-marker"))
            .count(),
        5
    );
    assert!(!result.rendered.svg.contains("var("));
    let tree = resvg::usvg::Tree::from_str(&result.rendered.svg, &Default::default()).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(result.rendered.width as u32, result.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 0).count() > 1000);
    assert!(engine
        .render_source(
            "erDiagram\nA { int }",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .is_err());
    let recovered = engine
        .render_source(
            "erDiagram\nA ||--o{ B : recovers",
            &RenderOptions::default(),
            &mut MetricsResources,
        )
        .unwrap();
    assert!(recovered.rendered.svg.contains(">recovers</text>"));
}
