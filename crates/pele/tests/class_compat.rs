//! Complete public class parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::class_diagram::Class, text::MetricsResources, *};
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
fn all_pinned_class_scenarios_match_model_svg_and_errors() {
    let fixture: Json = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/class.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut engine = Engine::empty();
    engine.register(Class);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if std::env::var("CLASS_FILTER")
            .ok()
            .is_some_and(|filter| !case["id"].as_str().unwrap().contains(&filter))
        {
            continue;
        }
        if std::env::var("CLASS_SKIP_STRESS").is_ok()
            && case["id"].as_str().unwrap().starts_with("stress/")
        {
            continue;
        }
        let options: RenderOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let source = case["source"].as_str().unwrap();
        let mut resources = Resources {
            host: case["host"].clone(),
            metrics: MetricsResources,
        };
        let mut model_projection = None;
        let actual = (|| -> Result<Json, PeleError> {
            let document = engine.parse(source, &options.parse)?;
            let model = if case["host"]["modelProjection"] == "excluded" {
                assert_eq!(document.model_view().unwrap_err().code, ErrorCode::Limit);
                None
            } else {
                Some(serde_json::to_value(document.model_view()?).unwrap())
            };
            model_projection = model.clone();
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
            Ok(if let Some(model) = model {
                serde_json::json!({"model":model,"result":result})
            } else {
                serde_json::json!({"result":result})
            })
        })();
        let actual = match actual {
            Ok(v) => v,
            Err(e) => {
                if let Some(model) = model_projection {
                    serde_json::json!({"model":model,"error":e})
                } else {
                    serde_json::json!({"error":e})
                }
            }
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
fn class_document_reuse_namespaces_generic_members_links_native_pixels_and_limits() {
    use pele::compat::diagrams::class_diagram::ClassModel;
    let mut engine = Engine::empty();
    engine.register(Class);
    let source="---\ntitle: API model\n---\nclassDiagram\naccTitle: API accessibility\naccDescr: Two related types\ndirection LR\nnamespace Product.API {\n class Client~T~:::accent {\n  <<service>>\n  +List~T~ items$\n  +fetch(Map~K,V~ input) List~T~*\n }\n note for Client \"**Remote** API fa:fa-cloud\"\n}\nclass Result\nClient \"1\" o-- \"many\" Result : returns\nResult --> Result : next\nRemote ()-- Result\nclassDef accent fill:#d13535,color:#fff\nlink Client \"https://example.test/client\" \"Open documentation\" _self";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<ClassModel>().unwrap();
    assert_eq!(model.classes.len(), 2);
    assert_eq!(model.namespaces.len(), 2);
    assert_eq!(model.classes[0].1.generic, "T");
    assert_eq!(
        model.classes[0].1.methods[0].display(),
        "+fetch(Map<K,V> input) : List<T>"
    );
    assert_eq!(model.classes[0].1.members[0].classifier, "$");
    assert_eq!(model.classes[0].1.parent.as_deref(), Some("Product.API"));
    assert_eq!(model.interfaces.len(), 1);
    let mut resources = Resources {
        host: serde_json::json!({"icons":{"fa:fa-cloud":"<path d=\"M0 0H24V24H0Z\"/>"}}),
        metrics: MetricsResources,
    };
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let wide = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    options.parse.preparation.limit = 0;
    options.render.max_width = Some(250.);
    let narrow = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    assert!(narrow.rendered.width <= wide.rendered.width);
    assert_eq!(
        narrow.rendered.links,
        vec![LinkInfo {
            id: "Client".into(),
            href: "https://example.test/client".into(),
            internal: false
        }]
    );
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    let count = |class: &str| {
        xml.descendants()
            .filter(|n| {
                n.attribute("class")
                    .is_some_and(|v| v.split(' ').any(|v| v == class))
            })
            .count()
    };
    assert_eq!(count("pele-class"), 3); // root plus the two declared types.
    assert_eq!(count("pele-namespace"), 2);
    assert_eq!(count("pele-interface"), 1);
    assert_eq!(count("pele-icon"), 1);
    assert_eq!(count("pele-static"), 1);
    assert_eq!(count("pele-abstract"), 1);
    assert!(xml
        .descendants()
        .any(|n| n.tag_name().name() == "title" && n.text() == Some("API accessibility")));
    for node in xml.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            if matches!(attr.name(), "fill" | "stroke" | "style") {
                assert!(!attr.value().contains("var("))
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
            .filter(|p| p.alpha() > 100 && p.red() > 150 && p.green() < 100 && p.blue() < 100)
            .count()
            > 200,
        "authored class fill paints red pixels"
    );
    let mut options = RenderOptions::default();
    options.render.output_limit = Some(100);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut resources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let blocked = engine
        .render_source(
            "classDiagram\nclass A\nlink A \"javascript:alert(1)\"",
            &RenderOptions::default(),
            &mut resources,
        )
        .unwrap();
    assert!(blocked.rendered.links.is_empty());
    assert!(!blocked.rendered.svg.contains("<a "));
    assert!(engine
        .render_document(&document, &RenderOptions::default(), &mut resources)
        .is_ok());
}
