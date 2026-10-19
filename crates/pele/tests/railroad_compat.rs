//! Complete public Railroad parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::railroad::Railroad, text::MetricsResources, *};
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
fn all_pinned_railroad_scenarios_match_model_svg_and_errors() {
    use std::io::Read;
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/pele/railroad.json"
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
    engine.register(Railroad);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if std::env::var("RAILROAD_FILTER")
            .ok()
            .is_some_and(|filter| !case["id"].as_str().unwrap().contains(&filter))
        {
            continue;
        }
        if std::env::var("RAILROAD_SKIP_STRESS").is_ok()
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
fn retained_railroad_all_notations_tracks_literals_and_native_pixels() {
    use pele::compat::diagrams::railroad::RailroadModel;
    let sources = [
        "railroad-beta\ntitle Grammar\naccTitle: Language\naccDescr: A grammar\nr = sequence(terminal(\"a\"),nonterminal(\"b\"),choice(optional(special(\"any\")),zeroOrMore(terminal(\"x\")))) ;\nr = oneOrMore(terminal(\"z\")) ;",
        "railroad-ebnf-beta\ntitle Grammar\naccTitle: Language\naccDescr: A grammar\nr = \"a\" b ( [ ? any ? ] | { \"x\" } ) ;\nr = \"z\"+ ;",
        "railroad-abnf-beta\ntitle Grammar\naccTitle: Language\naccDescr: A grammar\nr = \"a\" b ( [ c ] / *\"x\" ) ;\nr = 2*4\"z\" ;",
        "railroad-peg-beta\ntitle Grammar\naccTitle: Language\naccDescr: A grammar\nr <- \"a\" b ( [c] / \"x\"* ) ;\nr <- \"z\"+ ;"
    ];
    let mut engine = Engine::empty();
    engine.register(Railroad);
    for (i, source) in sources.iter().enumerate() {
        let source = if i == 3 {
            source.replace("[c]", "c?")
        } else {
            source.to_string()
        };
        let document = engine.parse(&source, &ParseOptions::default()).unwrap();
        let model = document.model::<RailroadModel>().unwrap();
        assert_eq!(model.rules.len(), 2);
        assert_eq!(model.rules[0].name, model.rules[1].name);
        assert!(!model.nodes.is_empty());
        assert_eq!(
            model.get_rule("r").unwrap().definition,
            model.rules[1].definition
        );
        let mut malformed = model.clone();
        malformed.rules[0].definition = malformed.nodes.len();
        assert_eq!(
            Railroad
                .render(
                    &malformed,
                    &Object::default(),
                    &RenderSettings::default(),
                    &mut MetricsResources
                )
                .unwrap_err()
                .code,
            ErrorCode::Semantic
        );
        let mut cyclic = model.clone();
        let root = cyclic.rules[0].definition;
        cyclic.nodes[root] = pele::compat::diagrams::railroad::RailroadNode::Optional(root);
        assert_eq!(
            Railroad
                .render(
                    &cyclic,
                    &Object::default(),
                    &RenderSettings::default(),
                    &mut MetricsResources
                )
                .unwrap_err()
                .code,
            ErrorCode::Semantic
        );
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
        assert!(retained.rendered.links.is_empty());
        let xml = roxmltree::Document::parse(&retained.rendered.svg).unwrap();
        assert_eq!(
            xml.descendants()
                .filter(|n| n.attribute("class") == Some("pele-rule"))
                .count(),
            2
        );
        assert!(xml
            .descendants()
            .any(|n| n.attribute("class") == Some("pele-edge pele-track")));
        assert!(xml
            .descendants()
            .any(|n| n.attribute("class") == Some("pele-node pele-terminal")));
        assert!(xml
            .descendants()
            .any(|n| n.attribute("class") == Some("pele-node pele-nonterminal")));
        assert!(!retained.rendered.svg.contains("var(--"));
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
        options.render.max_width = Some(180.);
        let narrow = engine
            .render_document(&document, &options, &mut MetricsResources)
            .unwrap();
        assert!(narrow.rendered.width < initial.rendered.width);
        assert!(narrow.rendered.height > initial.rendered.height);
    }
}
