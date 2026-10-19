//! Complete public Cynefin parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::cynefin::Cynefin, text::MetricsResources, *};
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
fn all_pinned_cynefin_scenarios_match_model_svg_and_errors() {
    use std::io::Read;
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/cynefin.json"
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
    engine.register(Cynefin);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if std::env::var("CYNEFIN_FILTER")
            .ok()
            .is_some_and(|filter| !case["id"].as_str().unwrap().contains(&filter))
        {
            continue;
        }
        if std::env::var("CYNEFIN_SKIP_STRESS").is_ok()
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
fn retained_cynefin_seeded_regions_overflow_transitions_icons_and_native_pixels() {
    use pele::compat::diagrams::cynefin::{CynefinModel, DomainName};
    let source="cynefin-beta\ntitle Incident response\naccTitle: Decisions\naccDescr: Five domains\ncomplex\n\"old\"\ncomplex\n\"fa:fa-user Investigate\"\ncomplicated\n\"Expert review\"\nclear\n\"Restart service\"\nchaotic\n\"Page support\"\nconfusion\n\"a\"\n\"b\"\n\"c\"\n\"d\"\n\"e\"\ncomplex --> clear : \"Pattern found\"\nclear --> chaotic\nconfusion --> complicated : \"Triage\"\nclear --> clear : \"ignored\"\n";
    let mut engine = Engine::empty();
    engine.register(Cynefin);
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<CynefinModel>().unwrap();
    assert_eq!(model.domains.len(), 5);
    assert_eq!(
        model.domain(DomainName::Complex).unwrap().items[0].label,
        "fa:fa-user Investigate"
    );
    assert_eq!(model.transitions.len(), 3);
    let mut resources = Resources {
        host: serde_json::json!({"icons":{"fa:fa-user":"<path d=\"M0 0L16 0L8 16Z\"/>"}}),
        metrics: MetricsResources,
    };
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let initial = engine
        .render_source(source, &options, &mut resources)
        .unwrap();
    options.parse.preparation.limit = 0;
    options.parse.max_edges = 0;
    let retained = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    assert_eq!(initial, retained);
    assert!(retained.rendered.links.is_empty());
    assert!(!retained.rendered.svg.contains("var(--"));
    let xml = roxmltree::Document::parse(&retained.rendered.svg).unwrap();
    for (class, count) in [
        ("pele-cynefin-region", 5),
        ("pele-cynefin-domain", 5),
        ("pele-cynefin-cliff", 1),
        ("pele-edge", 3),
        ("pele-marker", 3),
        ("pele-node pele-cynefin-overflow", 1),
    ] {
        assert_eq!(
            xml.descendants()
                .filter(|n| n.attribute("class") == Some(class))
                .count(),
            count
        );
    }
    assert!(retained.rendered.svg.contains("+2 more"));
    assert!(retained.rendered.svg.contains("M0 0L16 0L8 16Z"));
    assert!(!retained.rendered.svg.contains("ignored"));
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
            .filter(|p| p.alpha() > 0 && p.red() < 160 && p.green() < 160 && p.blue() < 160)
            .count()
            > 1000
    );
    options.render.id_prefix = Some("another-seed".into());
    let changed = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    let changed_xml = roxmltree::Document::parse(&changed.rendered.svg).unwrap();
    let boundary = |xml: &roxmltree::Document<'_>| {
        xml.descendants()
            .find(|n| n.attribute("class") == Some("pele-cynefin-boundaries"))
            .unwrap()
            .children()
            .find(|n| n.is_element())
            .unwrap()
            .attribute("d")
            .unwrap()
            .to_string()
    };
    assert_ne!(boundary(&xml), boundary(&changed_xml));
    options.render.id_prefix = None;
    options.render.max_width = Some(320.);
    let narrow = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    assert_eq!(narrow.rendered.width, initial.rendered.width);
    assert_eq!(narrow.rendered.height, initial.rendered.height);
}
