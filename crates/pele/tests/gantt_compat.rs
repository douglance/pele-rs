//! Complete public gantt parse/model/render comparisons against pinned Pele.
use pele::compat::{diagrams::gantt::Gantt, text::MetricsResources, *};
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
    fn parse_date(&mut self, input: &str) -> Result<Option<f64>, PeleError> {
        self.metrics.parse_date(input)
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        Ok(self.host["icons"][name]
            .as_str()
            .map(|s| TrustedSvg::from_trusted_markup(s.into())))
    }
}
#[test]
fn all_pinned_gantt_scenarios_match_model_svg_and_errors() {
    let fixture: Json = serde_json::from_reader(flate2::read::GzDecoder::new(
        std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/pele/gantt.json.gz"
        ))
        .unwrap(),
    ))
    .unwrap();
    let mut engine = Engine::empty();
    engine.register(Gantt);
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        if std::env::var("GANTT_FILTER")
            .ok()
            .is_some_and(|filter| !case["id"].as_str().unwrap().contains(&filter))
        {
            continue;
        }
        if std::env::var("GANTT_SKIP_STRESS").is_ok()
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
                format!(
                    "actual error {} expected error {} modelEqual {}",
                    actual["error"],
                    case["expected"]["error"],
                    actual["model"] == case["expected"]["model"]
                )
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
                    actual["model"] == case["expected"]["model"],
                    actual["result"]["width"],
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
            .take(200)
            .collect::<Vec<_>>()
            .join("\n\n")
    );
}

#[test]
fn gantt_schedules_dependencies_calendar_exclusions_and_renders_native_task_states() {
    use pele::compat::diagrams::gantt::GanttModel;
    let mut engine = Engine::empty();
    engine.register(Gantt);
    let source="---\ntitle: Release plan\nconfig:\n gantt:\n  displayMode: compact\n---\ngantt\ndateFormat YYYY-MM-DD\naxisFormat %a %d\ntickInterval 1day\naccTitle: Release schedule\nexcludes weekends\nincludes 2024-01-06\nsection Planning\nFirst :done,a,2024-01-04,2d\nSecond :active,crit,b,after a,3d\nsection Shipping\nRelease :milestone,r,after b,0d\nMarker :vert,v,2024-01-12,0d\nclick a href \"https://example.test/task\"";
    let document = engine.parse(source, &ParseOptions::default()).unwrap();
    let raw = document.model::<GanttModel>().unwrap();
    assert_eq!(raw.tasks.len(), 4);
    assert!(raw.tasks.iter().all(|t| t.start_time.is_none()));
    let mut resources = Resources {
        host: Json::Null,
        metrics: MetricsResources,
    };
    let now = 1704844800000.; // 2024-01-10T00:00Z
    let scheduled = raw.scheduled(now, &mut resources).unwrap();
    assert_eq!(scheduled.tasks[0].start_time, Some(1704326400000.)); // Jan4
    assert_eq!(scheduled.tasks[0].end_time, Some(1704499200000.)); // included Jan6
    assert_eq!(scheduled.tasks[1].start_time, scheduled.tasks[0].end_time);
    assert_eq!(scheduled.tasks[1].end_time, Some(1704844800000.)); // Jan10, Sunday excluded
    assert!(scheduled.tasks.iter().all(|t| t.processed));
    assert_eq!(scheduled.tasks[3].order, -1);
    let mut options = RenderOptions::default();
    options.render.now = Some(now);
    options.render.native_theme = Some(NativeTheme::default());
    options.render.max_width = Some(320.);
    let result = engine
        .render_document(&document, &options, &mut resources)
        .unwrap();
    assert_eq!(result.rendered.links.len(), 1);
    assert_eq!(result.rendered.links[0].id, "a");
    let xml = roxmltree::Document::parse(&result.rendered.svg).unwrap();
    let count = |class: &str| {
        xml.descendants()
            .filter(|n| {
                n.attribute("class")
                    .is_some_and(|c| c.split(' ').any(|c| c == class))
            })
            .count()
    };
    for (class, want) in [
        ("pele-task", 3),
        ("pele-milestone", 1),
        ("pele-active", 1),
        ("pele-crit", 1),
        ("pele-done", 1),
        ("pele-vert", 1),
        ("pele-today", 1),
    ] {
        assert_eq!(count(class), want, "{class}")
    }
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
    assert!(pixels.pixels().iter().filter(|p| p.alpha() > 100).count() > 500);
    options.render.output_limit = Some(100);
    assert_eq!(
        engine
            .render_document(&document, &options, &mut resources)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    options.render.output_limit = None;
    assert!(engine
        .render_document(&document, &options, &mut resources)
        .is_ok());
    assert!(
        raw.tasks.iter().all(|t| t.start_time.is_none()),
        "rendering retains the immutable parsed source model"
    );
}
