//! Complete public architecture parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::architecture::Architecture, text::MetricsResources, *};
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
fn all_pinned_architecture_scenarios_match_model_svg_and_errors() {
    let fixture: Json = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/architecture.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut engine = Engine::empty();
    engine.register(Architecture);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if std::env::var("ARCHITECTURE_FILTER")
            .ok()
            .is_some_and(|filter| !case["id"].as_str().unwrap().contains(&filter))
        {
            continue;
        }
        if std::env::var("ARCHITECTURE_SKIP_STRESS").is_ok()
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
fn architecture_nested_boundary_ports_icons_narrow_reuse_and_native_pixels() {
    use pele::compat::diagrams::architecture::ArchitectureModel;
    let mut engine = Engine::empty();
    engine.register(Architecture);
    let source="---\ntitle: Deployment\n---\narchitecture-beta\naccTitle: Production architecture\naccDescr {\n A database and client\n}\ngroup cloud(cloud)[**Cloud**]\ngroup inner(server)[Private] in cloud\nservice db(database)[Database] in inner\nservice api \"API\"[Gateway] in cloud\nservice client(internet)[Client]\njunction hub\ndb{group}:R <-[query]-> L:api\napi{group}:B --> T:client\nclient:R -- L:hub\nalign column api client";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<ArchitectureModel>().unwrap();
    assert_eq!(model.groups.len(), 2);
    assert_eq!(model.nodes.len(), 4);
    assert_eq!(model.edges.len(), 3);
    assert_eq!(model.layout_hints.len(), 1);
    assert_eq!(model.nodes[0].1.parent.as_deref(), Some("inner"));
    assert!(model.edges[0].lhs_group);
    let before = serde_json::to_value(document.model_view().unwrap()).unwrap();
    let mut resources = Resources {
        host: serde_json::json!({"icons":{"cloud":"<circle cx=\"12\" cy=\"12\" r=\"8\" fill=\"#d13535\"/>"}}),
        metrics: MetricsResources,
    };
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let wide = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    options.render.max_width = Some(160.);
    options.parse.preparation.limit = 0;
    let narrow = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    assert!(narrow.rendered.width <= wide.rendered.width);
    assert!(narrow.rendered.links.is_empty());
    assert_eq!(
        before,
        serde_json::to_value(document.model_view().unwrap()).unwrap()
    );
    let xml = roxmltree::Document::parse(&narrow.rendered.svg).unwrap();
    let count = |class: &str| {
        xml.descendants()
            .filter(|n| {
                n.attribute("class")
                    .is_some_and(|s| s.split(' ').any(|s| s == class))
            })
            .count()
    };
    assert_eq!(count("pele-cluster"), 2);
    assert_eq!(count("pele-service"), 3);
    assert_eq!(count("pele-junction"), 1);
    assert_eq!(count("pele-edge"), 3);
    assert_eq!(count("pele-edge-label"), 1);
    assert!(xml
        .descendants()
        .any(|n| n.tag_name().name() == "title" && n.text() == Some("Production architecture")));
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
            > 40
    );
    options.render.output_limit = Some(100);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut resources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert!(engine
        .render_document(&document, &RenderOptions::default(), &mut resources)
        .is_ok());
    assert_eq!(
        engine
            .parse(
                "architecture-beta\ngroup g\nservice a in g\nservice b in g\na{group}:R -- L:b",
                &ParseOptions::default()
            )
            .err().unwrap()
            .code,
        ErrorCode::Semantic
    );
}
