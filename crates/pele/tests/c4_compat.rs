//! Complete public C4 parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::c4::C4, text::MetricsResources, *};
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
fn all_pinned_c4_scenarios_match_model_svg_and_errors() {
    use std::io::Read;
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/c4.json"
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
    engine.register(C4);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if std::env::var("C4_FILTER")
            .ok()
            .is_some_and(|filter| !case["id"].as_str().unwrap().contains(&filter))
        {
            continue;
        }
        if std::env::var("C4_SKIP_STRESS").is_ok()
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
fn retained_c4_all_headers_symbols_links_and_native_pixels() {
    use pele::compat::diagrams::c4::C4Model;
    let source="C4Context\ntitle Banking context\naccDescr: A nested context with a retained link\nPerson(user, \"Customer\")\nEnterprise_Boundary(bank, \"Bank\") {\nSystem(app, \"Banking\", $link=\"https://example.com/bank\")\nSystemDb_Ext(store, \"Ledger\")\nContainerQueue(queue, \"Queue\", \"AMQP\")\n}\nRel(user, app, \"Uses\", \"HTTPS\")\nBiRel(app, store, \"Syncs\")\nRel(app, queue, \"Enqueues\")\nUpdateElementStyle(app, $bgColor=\"#247acd\", $fontColor=\"#ffffff\")\n";
    let mut engine = Engine::empty();
    engine.register(C4);
    for header in [
        "C4Context",
        "C4Container",
        "C4Component",
        "C4Dynamic",
        "C4Deployment",
    ] {
        let source = source.replacen("C4Context", header, 1);
        let document = engine.parse(&source, &ParseOptions::default()).unwrap();
        let model = document.model::<C4Model>().unwrap();
        assert_eq!(model.c4_type.as_deref(), Some(header));
        assert_eq!(model.shapes.len(), 4);
        assert_eq!(model.boundaries.len(), 2);
        let mut options = RenderOptions::default();
        options.render.native_theme = Some(NativeTheme::default());
        let initial = engine
            .render_source(&source, &options, &mut MetricsResources)
            .unwrap();
        options.parse.preparation.limit = 0;
        options.parse.max_edges = 0;
        let retained = engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap();
        assert_eq!(initial, retained);
        assert_eq!(
            retained.rendered.links,
            vec![LinkInfo {
                id: "app".into(),
                href: "https://example.com/bank".into(),
                internal: false
            }]
        );
        let xml = roxmltree::Document::parse(&retained.rendered.svg).unwrap();
        for form in ["person", "rect", "cyl", "h-cyl"] {
            assert!(xml.descendants().any(|n| n
                .attribute("class")
                .is_some_and(|c| c.starts_with(&format!("pele-node pele-shape-{form} ")))))
        }
        let tree =
            resvg::usvg::Tree::from_str(&retained.rendered.svg, &Default::default()).unwrap();
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
                .filter(|p| p.alpha() > 200 && p.blue() > 150 && p.red() < 80)
                .count()
                > 1000
        );
        options.render.max_width = Some(320.);
        let narrow = engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap();
        assert!(narrow.rendered.width < retained.rendered.width);
        assert_eq!(narrow.rendered.links, retained.rendered.links);
    }
}
