//! A downstream module exercises preparation, registry replacement, model ownership and resources.
use pele::compat::*;
use serde::Serialize;
#[derive(Serialize)]
struct Document {
    source: String,
    title: Option<String>,
    line_offset: usize,
    max_edges: usize,
}
struct ConsumerModule(&'static str);
impl DiagramModule for ConsumerModule {
    type Model = Document;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: DiagramType::Info.into(),
            section: Some("consumer".into()),
            keep_comments: true,
        }
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Document, PeleError> {
        Ok(Document {
            source: c.source.into(),
            title: c.title.map(str::to_owned),
            line_offset: c.line_offset,
            max_edges: c.max_edges,
        })
    }
    fn render(
        &self,
        model: &Document,
        config: &Object,
        options: &RenderSettings,
        resources: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        let width = resources.text_width(TextRequest {
            text: model.title.as_deref().unwrap_or(""),
            size: 16.,
            style: 1,
            font_family: options.font_family.as_deref(),
            font_family_mono: None,
        })?;
        Ok(Rendered {
            svg: format!(
                "{}:{}:{}:{:?}",
                self.0,
                model.title.as_deref().unwrap_or(""),
                options.responsive.unwrap(),
                config.get("consumer")
            ),
            width,
            height: 1.,
            links: vec![],
        })
    }
}
struct Host;
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        Ok(r.text.encode_utf16().count() as f64 * r.size)
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        Ok(0.)
    }
}
#[test]
fn consumer_module_keeps_its_parse_contract_after_replacement() {
    let mut engine = Engine::empty();
    assert!(!engine.supports("info"));
    engine.register(ConsumerModule("original"));
    assert!(engine.registered(DiagramType::Info));
    assert!(engine.supports("info invalid syntax"));
    let options:RenderOptions=serde_json::from_str(r#"{"limit":1000,"maxEdges":19,"config":{"consumer":{"useMaxWidth":false}},"fontFamily":"Arial","padding":12,"outputLimit":1000}"#).unwrap();
    let document = engine
        .parse(
            "---\ntitle: 🙂 title\n---\ninfo\n%% keep this\n#35;",
            &options.parse,
        )
        .unwrap();
    let model = document.model::<Document>().unwrap();
    assert_eq!(model.line_offset, 3);
    assert_eq!(model.max_edges, 19);
    assert!(model.source.contains("%% keep this"));
    assert!(model.source.ends_with("ﬂ°°35¶ß\n"));
    let first = engine
        .render_document(&document, &options, &mut Host)
        .unwrap();
    assert!(first.rendered.svg.starts_with("original:🙂 title:false"));
    engine.register(ConsumerModule("replacement"));
    assert_eq!(
        engine
            .render_document(&document, &options, &mut Host)
            .unwrap(),
        first
    );
    assert!(engine
        .render_source("info", &options, &mut Host)
        .unwrap()
        .rendered
        .svg
        .starts_with("replacement:"));
    let mut changed = options.clone();
    changed.parse.preparation.limit = 0;
    changed.parse.preparation.config = Object::default();
    changed.parse.max_edges = 0;
    assert_eq!(
        engine
            .render_document(&document, &changed, &mut Host)
            .unwrap(),
        first,
        "parse options are not reapplied to owned documents"
    );
    changed.render.output_limit = Some(0);
    let err = engine
        .render_document(&document, &changed, &mut Host)
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::Limit);
    let cap = engine
        .capabilities()
        .into_iter()
        .find(|c| c.diagram_type == DiagramType::Info.into())
        .unwrap();
    assert!(cap.registered);
    assert!(
        !cap.render_parity,
        "a registered custom module is not proof of builtin parity"
    );
    drop(engine);
    assert!(
        document.model_view().unwrap().as_object().is_some(),
        "document owns its module/model"
    );
}
#[test]
fn core_rejects_absent_handlers_and_keeps_config_numbers() {
    let engine = Engine::empty();
    let err = engine.parse("pie", &ParseOptions::default()).err().unwrap();
    assert_eq!(err.code, ErrorCode::UnsupportedDiagram);
    assert_eq!(err.diagram_type.as_deref(), Some("pie"));
    let output = engine
        .inspect(
            "---\nconfig: {nan: .nan, inf: .inf, 10: ten, 2: two, 1: one, z: last}\n---\ninfo",
            &PrepareOptions::default(),
        )
        .unwrap();
    assert!(matches!(output.prepared.config.get("nan"),Some(Value::Number(n)) if n.is_nan()));
    assert!(matches!(output.prepared.config.get("inf"),Some(Value::Number(n)) if n.is_infinite()));
    let json = inspect_envelope(
        "---\nconfig: {10: ten, 2: two, 1: one, z: last}\n---\ninfo",
        "",
    )
    .1;
    assert!(
        json.contains(r#""config":{"1":"one","2":"two","10":"ten","z":"last"}"#),
        "JSON preserves JS enumeration order"
    );
}

struct WorkflowModule(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl DiagramModule for WorkflowModule {
    type Model = Object;
    fn descriptor(&self) -> ModuleDescriptor {
        let changed = self.0.load(std::sync::atomic::Ordering::Relaxed);
        ModuleDescriptor {
            id: DiagramId::extension(if changed { "mutated" } else { "workflow" }),
            section: Some(if changed { "other" } else { "consumer" }.into()),
            keep_comments: false,
        }
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<Object, PeleError> {
        Ok(c.config.clone())
    }
    fn render(
        &self,
        _model: &Object,
        _config: &Object,
        options: &RenderSettings,
        resources: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        let a = resources.icon("ﬂ°amp¶ß")?;
        let b = resources.icon("ﬂ°amp¶ß")?;
        assert_eq!(
            a.as_ref().map(TrustedSvg::markup),
            b.as_ref().map(TrustedSvg::markup)
        );
        let time = resources.now_millis()?;
        assert_eq!(time, resources.now_millis()?);
        Ok(Rendered {
            svg: format!("responsive={}", options.responsive.unwrap()),
            width: 1.,
            height: time,
            links: vec![LinkInfo {
                id: "ﬂ°amp¶ß ﬂ°°35¶ß &lt;".into(),
                href: "#local".into(),
                internal: true,
            }],
        })
    }
}
#[derive(Default)]
struct ResourceHost {
    icons: Vec<String>,
    clocks: usize,
}
impl RenderResources for ResourceHost {
    fn text_width(&mut self, _: TextRequest<'_>) -> Result<f64, PeleError> {
        Ok(1.)
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        self.icons.push(name.into());
        Ok(None)
    }
    fn now_millis(&mut self) -> Result<f64, PeleError> {
        self.clocks += 1;
        Ok(self.clocks as f64)
    }
}
#[test]
fn new_language_retains_metadata_order_and_per_render_resource_lifetimes() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let changed = Arc::new(AtomicBool::new(false));
    let mut engine = Engine::empty();
    engine.register(WorkflowModule(changed.clone()));
    changed.store(true, Ordering::Relaxed);
    let id = DiagramId::extension("workflow");
    assert!(engine.registered(id.clone()));
    assert!(!engine.registered(DiagramId::extension("mutated")));
    let source="---\nconfig: {consumer: {useMaxWidth: false}, nested: {z: last, a: first, 10: ten, 2: two}}\n---\nworkflow one";
    let doc = engine
        .parse_as(&id, source, &ParseOptions::default())
        .unwrap();
    assert_eq!(doc.diagram_type(), &id);
    let projected = serde_json::to_string(&doc.model_view().unwrap()).unwrap();
    assert!(
        projected.contains(r#""nested":{"2":"two","10":"ten","z":"last","a":"first"}"#),
        "nested projection must preserve JS map enumeration"
    );
    let mut host = ResourceHost::default();
    let result = engine
        .render_document(&doc, &RenderOptions::default(), &mut host)
        .unwrap();
    assert_eq!(result.rendered.svg, "responsive=false");
    assert_eq!(result.rendered.links[0].id, "& # <");
    assert_eq!(host.icons, vec!["&"]);
    assert_eq!(host.clocks, 1);
    engine
        .render_document(&doc, &RenderOptions::default(), &mut host)
        .unwrap();
    assert_eq!(
        host.icons.len(),
        2,
        "negative icon cache must end with the render"
    );
    assert_eq!(host.clocks, 2);
    let mut explicit = RenderOptions::default();
    explicit.render.now = Some(42.);
    assert_eq!(
        engine
            .render_document(&doc, &explicit, &mut host)
            .unwrap()
            .rendered
            .height,
        42.
    );
    assert_eq!(host.clocks, 2, "explicit time must not read host clock");
    let caps = engine.capabilities();
    assert_eq!(caps.len(), 34);
    let extension = caps.last().unwrap();
    assert!(!extension.known && extension.registered && !extension.render_parity);
}

struct LinkModule;
impl DiagramModule for LinkModule {
    type Model = String;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("links"))
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<String, PeleError> {
        Ok(c.source.into())
    }
    fn render(
        &self,
        source: &String,
        _: &Object,
        _: &RenderSettings,
        _: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Ok(Rendered {
            svg: "<svg/>".into(),
            width: 1.,
            height: 1.,
            links: vec![LinkInfo {
                id: source.clone(),
                href: "#local".into(),
                internal: true,
            }],
        })
    }
}
#[test]
fn consumer_links_decode_all_named_numeric_and_malformed_entity_scenarios() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/pele/entity-links.json")).unwrap();
    let mut engine = Engine::empty();
    engine.register(LinkModule);
    let id = DiagramId::extension("links");
    for case in fixture["cases"].as_array().unwrap() {
        let document = engine
            .parse_as(
                &id,
                case["source"].as_str().unwrap(),
                &ParseOptions::default(),
            )
            .unwrap();
        let result = engine.render_document(&document, &RenderOptions::default(), &mut Host);
        if case["expected"].get("error").is_some() {
            assert_eq!(result.unwrap_err().code, ErrorCode::Limit, "{}", case["id"])
        } else {
            assert_eq!(
                result.unwrap().rendered.links[0].id,
                case["expected"]["value"],
                "{}",
                case["id"]
            )
        }
    }
}
