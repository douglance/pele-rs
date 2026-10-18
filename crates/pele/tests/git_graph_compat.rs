//! Complete public GitGraph parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::git_graph::GitGraph, text::MetricsResources, *};
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
fn all_pinned_git_graph_scenarios_match_model_svg_and_errors() {
    use std::io::Read;
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/git-graph.json"
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
    engine.register(GitGraph);
    // Eight official mutations change their detected family to flowchart.
    engine.register(pele::compat::diagrams::flowchart::Flowchart);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if std::env::var("GIT_GRAPH_FILTER")
            .ok()
            .is_some_and(|filter| !case["id"].as_str().unwrap().contains(&filter))
        {
            continue;
        }
        if std::env::var("GIT_GRAPH_SKIP_STRESS").is_ok()
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
fn retained_git_document_commit_semantics_symbols_and_native_pixels() {
    use pele::compat::diagrams::git_graph::GitModel;
    let mut engine = Engine::empty();
    engine.register(GitGraph);
    let source="---\ntitle: Release history\n---\ngitGraph\naccTitle: Version history\naccDescr: Feature merge and cherry pick\ncommit id:\"initial\" msg:\"Initial commit\"\nbranch feature\ncommit id:\"feature\" type:REVERSE\ncheckout main\ncommit id:\"release\" type:HIGHLIGHT\nmerge feature id:\"merged\" tag:\"v1.0\"\nbranch fixes\ncommit id:\"fix\"\ncheckout main\ncherry-pick id:\"fix\"\n";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<GitModel>().unwrap();
    let direct =
        pele::compat::diagrams::git_graph::parse(source, &ParseOptions::default()).unwrap();
    assert_eq!(
        serde_json::to_value(direct).unwrap(),
        serde_json::to_value(model).unwrap()
    );
    assert_eq!(model.commits.len(), 6);
    assert_eq!(model.head.as_ref().unwrap().commit_type, 4);
    assert_eq!(model.lanes, vec!["main", "feature", "fixes"]);
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
    let xml = roxmltree::Document::parse(&retained.rendered.svg).unwrap();
    for kind in ["normal", "reverse", "highlight", "merge", "cherry-pick"] {
        assert!(xml
            .descendants()
            .any(|n| n.attribute("class")
                == Some(format!("pele-commit pele-commit-{kind}").as_str())))
    }
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Version history")));
    let tree = resvg::usvg::Tree::from_str(&retained.rendered.svg, &Default::default()).unwrap();
    let mut pixels = resvg::tiny_skia::Pixmap::new(
        retained.rendered.width as u32,
        retained.rendered.height as u32,
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
            .filter(|p| p.alpha() > 200 && p.blue() > 130 && p.red() < 100)
            .count()
            > 500
    );
    options.render.max_width = Some(200.);
    let narrow = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert!(narrow.rendered.width < retained.rendered.width);
}
