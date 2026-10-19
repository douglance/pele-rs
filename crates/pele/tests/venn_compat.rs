//! Complete public Venn parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::venn::Venn, text::MetricsResources, *};
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
fn all_pinned_venn_scenarios_match_model_svg_and_errors() {
    use std::io::Read;
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/venn.json"
    ));
    let bytes = if path.exists() {
        std::fs::read(path).unwrap()
    } else {
        let file = std::fs::File::open(path.with_extension("json.gz")).unwrap();
        let mut bytes = Vec::new();
        flate2::read::GzDecoder::new(file)
            .read_to_end(&mut bytes)
            .unwrap();
        bytes
    };
    let fixture: Json = serde_json::from_slice(&bytes).unwrap();
    let mut engine = Engine::empty();
    engine.register(Venn);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
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
            let differing =
                if actual.get("error").is_some() || case["expected"].get("error").is_some() {
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
                        "models equal {}\nSVG first diff byte {p}: actual {:?} expected {:?}",
                        actual["model"] == case["expected"]["model"],
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
fn retained_venn_circle_areas_styled_regions_text_and_native_pixels() {
    use pele::compat::diagrams::venn::VennModel;
    let source="venn-beta\ntitle Intersections\nset A[\"Alpha\"]:20\n  text a[\"Content A\"]\nset B[\"Beta\"]:14\nset C[\"Gamma\"]:9\nunion A,B[\"AB\"]:4\nunion B,C[\"BC\"]:2\nunion A,C[\"AC\"]:3\nunion A,B,C[\"All\"]:1\n  text center[\"Shared\"]\nstyle A stroke:black\nstyle B stroke:black\nstyle C stroke:black\nstyle A,B fill:#247acd,stroke:black,stroke-width:2\nstyle A,B,C fill:green\nstyle center color:purple\n";
    let mut engine = Engine::empty();
    engine.register(Venn);
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<VennModel>().unwrap();
    assert_eq!(model.subsets.len(), 7);
    assert_eq!(model.text_nodes.len(), 2);
    assert_eq!(model.text_nodes[1].sets, vec!["A", "B", "C"]);
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let initial = engine
        .render_source(source, &options, &mut MetricsResources)
        .unwrap();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    let retained = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert_eq!(initial, retained);
    assert!(retained.rendered.links.is_empty());
    assert!(!retained.rendered.svg.contains("var(--"));
    let xml = roxmltree::Document::parse(&retained.rendered.svg).unwrap();
    let circles: Vec<_> = xml
        .descendants()
        .filter(|n| n.attribute("class") == Some("pele-venn-set"))
        .collect();
    assert_eq!(circles.len(), 3);
    let radius = |id: &str| {
        circles
            .iter()
            .find(|c| c.attribute("data-id") == Some(id))
            .unwrap()
            .attribute("r")
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    assert!((radius("A").powi(2) / radius("B").powi(2) - 20. / 14.).abs() < 0.001);
    assert!((radius("A").powi(2) / radius("C").powi(2) - 20. / 9.).abs() < 0.001);
    let paths: Vec<_> = xml
        .descendants()
        .filter(|n| n.attribute("class") == Some("pele-venn-overlap"))
        .collect();
    assert_eq!(paths.len(), 2);
    assert!(paths
        .iter()
        .all(|n| n.attribute("d").unwrap().contains('A')));
    assert!(xml
        .descendants()
        .any(|n| n.attribute("class") == Some("pele-venn-text")
            && n.attribute("data-id") == Some("center")));
    let tree =
        resvg::usvg::Tree::from_str(&retained.rendered.svg, &resvg::usvg::Options::default())
            .unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(
        tree.size().width().ceil() as u32,
        tree.size().height().ceil() as u32,
    )
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
            .filter(|p| p.alpha() > 0
                && p.red() < 80
                && p.green() > 100
                && p.green() < 150
                && p.blue() > 150)
            .count()
            > 1000
    );
    options.render.max_width = Some(320.);
    let narrow = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert_eq!(narrow.rendered.width, initial.rendered.width);
    assert_eq!(narrow.rendered.height, initial.rendered.height);
}
