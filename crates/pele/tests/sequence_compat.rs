//! Public feature-slice integration: sequence source preparation, parser and ordered models.
use pele::compat::{
    diagrams::sequence, ParseOptions, PeleError, RenderResources, TextRequest, TrustedSvg,
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
        serde_json::from_reader(flate2::read::GzDecoder::new(
            include_bytes!("../../../fixtures/pele/sequence.json.gz").as_slice(),
        ))
        .unwrap()
    })
}
#[test]
fn pinned_sequence_parser_corpus() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let options: ParseOptions = serde_json::from_value(case["options"].clone()).unwrap();
        let actual = match {
            let mut engine = pele::compat::Engine::empty();
            engine.register(sequence::Sequence);
            engine
                .parse(case["source"].as_str().unwrap(), &options)
                .map(|d| match d.model_view() {
                    Ok(model) => serde_json::json!({"model":model}),
                    Err(e) => {
                        if case["expected"]["modelProjectionLimit"] == true {
                            serde_json::json!({"modelProjectionLimit":true})
                        } else {
                            serde_json::json!({"error":e})
                        }
                    }
                })
        } {
            Ok(model) => model,
            Err(error) => serde_json::json!({"error":error}),
        };
        let mut expected = case["expected"].clone();
        expected.as_object_mut().unwrap().remove("result");
        if expected.get("model").is_some() || expected.get("modelProjectionLimit").is_some() {
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
fn pinned_sequence_complete_svg_corpus() {
    use pele::compat::{diagrams::sequence::Sequence, Engine, RenderOptions};
    let fixture = fixture();
    let mut engine = Engine::empty();
    engine.register(Sequence);
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
fn retained_sequence_reflow_lifecycle_frames_and_native_pixels() {
    use pele::compat::{
        diagrams::sequence::{Sequence, SequenceModel},
        text::MetricsResources,
        Engine, NativeTheme, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Sequence);
    let source="sequenceDiagram\naccTitle: Durable request\naccDescr: Client stores an order and receives confirmation\ntitle Order processing\nbox Aqua Client devices\nparticipant A as Client application\nend\nparticipant B@{type: database, alias: Durable storage}\nautonumber\nA->>+B: wrap: Submit this order for durable storage\nNote over A,B: wrap: Acknowledgement follows persistence\nalt Accepted\nB-->>A: Stored\nelse Retry\nloop Every second\nB->>B: Retry write\nend\nend\ncreate actor C as Delivery agent\nB->>C: Deliver\ndestroy C\nC-xB: Complete\nB-->>-A: Success";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let model = document.model::<SequenceModel>().unwrap();
    assert_eq!(model.actors.len(), 3);
    assert_eq!(model.boxes.len(), 1);
    assert_eq!(model.notes.len(), 1);
    assert_eq!(model.created_actors[0].0, "C");
    assert_eq!(model.destroyed_actors[0].0, "C");
    let before = document.model_view().unwrap();
    let wide = engine
        .render_document(&document, &RenderOptions::default(), &mut MetricsResources)
        .unwrap();
    let mut options = RenderOptions::default();
    options.render.max_width = Some(320.);
    options.render.native_theme = Some(NativeTheme::default());
    options.parse.preparation.limit = 0;
    let native = engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap();
    assert!(native.rendered.width < wide.rendered.width);
    assert_eq!(before, document.model_view().unwrap());
    assert!(!native.rendered.svg.contains("var("));
    assert!(native.rendered.links.is_empty());
    let xml = roxmltree::Document::parse(&native.rendered.svg).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.has_tag_name("title") && n.text() == Some("Durable request")));
    for class in [
        "pele-activation",
        "pele-sequence-number",
        "pele-note",
        "pele-frame pele-frame-alt",
        "pele-frame pele-frame-loop",
        "pele-destroy",
    ] {
        assert!(
            xml.descendants()
                .any(|n| n.attribute("class") == Some(class)),
            "missing {class}"
        )
    }
    assert_eq!(
        xml.descendants()
            .filter(|n| n
                .attribute("class")
                .is_some_and(|c| c.contains("pele-actor-footer")))
            .count(),
        2
    );
    let tree = resvg::usvg::Tree::from_str(&native.rendered.svg, &Default::default()).unwrap();
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(native.rendered.width as u32, native.rendered.height as u32)
            .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 0).count() > 1000);
    assert!(engine
        .render_source(
            "sequenceDiagram\ndeactivate A",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .is_err());
    assert!(engine
        .render_source(
            "sequenceDiagram\nA->>B: Recovered",
            &RenderOptions::default(),
            &mut MetricsResources
        )
        .unwrap()
        .rendered
        .svg
        .contains("Recovered"));
}

#[test]
fn complete_deep_menus_and_shared_boxes_render_beyond_model_transport_budget() {
    use pele::compat::{
        diagrams::sequence::{Sequence, SequenceModel},
        text::MetricsResources,
        Engine, RenderOptions,
    };
    let mut engine = Engine::empty();
    engine.register(Sequence);
    let mut options = RenderOptions::default();
    options.parse.preparation.limit = 1_000_000;
    let source = format!(
        "sequenceDiagram\nparticipant A\nlinks A: {}{}\nA->>B: Complete",
        "[".repeat(25000),
        "]".repeat(25000)
    );
    let document = engine.parse(&source, &options.parse).unwrap();
    let model = document.model::<SequenceModel>().unwrap();
    assert_eq!(model.actors[0].1.links.len(), 1);
    assert_eq!(model.actors[0].1.links[0].1.json().len(), 49998);
    assert!(document.model_view().is_err());
    assert!(engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap()
        .rendered
        .svg
        .contains("Complete"));
    let mut boxed = "sequenceDiagram\nbox Shared group\n".to_owned();
    for i in 0..2000 {
        boxed.push_str(&format!("participant p{i}\n"))
    }
    boxed.push_str("end\np0->>p1999: Complete");
    let document = engine.parse(&boxed, &options.parse).unwrap();
    let model = document.model::<SequenceModel>().unwrap();
    assert_eq!(model.boxes[0].actor_keys.len(), 2000);
    assert_eq!(
        model.actors[0].1.r#box.as_ref().unwrap().actor_keys.len(),
        2000
    );
    assert_eq!(
        document.model_view().unwrap_err().code,
        pele::compat::ErrorCode::Limit
    );
    assert!(engine
        .render_document(&document, &options, &mut MetricsResources)
        .unwrap()
        .rendered
        .svg
        .contains("Complete"));
}
