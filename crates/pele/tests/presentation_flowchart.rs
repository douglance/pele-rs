//! Public native presentation feature: retained typed/mutable models share final SVG geometry.
use pele::compat::{
    diagrams::{
        flowchart::{Flowchart, FlowchartModel},
        swimlane::Swimlane,
    },
    public::{
        Atom, GraphWriter, ModelGraph, Node, PublicDiagramModule, PublicDocument, PublicEngine,
    },
    *,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Default)]
struct Host {
    calls: Vec<(String, u64, u8)>,
    fail: bool,
}
impl RenderResources for Host {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        self.calls
            .push((request.text.into(), request.size.to_bits(), request.style));
        if self.fail {
            return Err(PeleError::new("measurement stopped", ErrorCode::Semantic));
        }
        Ok(request.text.encode_utf16().count() as f64 * request.size * 0.6)
    }
}
fn artifact(capture: &PresentationCapture<'_>) -> PresentationArtifact {
    match capture.outcome().expect("final presentation") {
        PresentationOutcome::Available(frame) => frame.clone(),
        other => panic!("{other:?}"),
    }
}
const SOURCE: &str = "flowchart LR\naccTitle: Orders &amp; links\naccDescr: Native accessible description\nA[Start] -->|next| B[Finish]\nclick A \"https://example.test/orders\" \"Order details\" _blank\nclick B \"https://example.test/finish\"";
fn number(node: roxmltree::Node<'_, '_>, name: &str) -> f64 {
    node.attribute(name).unwrap().parse().unwrap()
}
fn assert_geometry(result: &RenderResult, frame: &PresentationArtifact) {
    assert_eq!(frame.version, PRESENTATION_VERSION);
    assert_eq!(
        (frame.width, frame.height),
        (result.rendered.width, result.rendered.height)
    );
    let xml = roxmltree::Document::parse(&result.rendered.svg).unwrap();
    let viewbox: Vec<f64> = xml
        .root_element()
        .attribute("viewBox")
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect();
    assert_eq!(viewbox, [0., 0., frame.width, frame.height]);
    for link in &frame.links {
        let node = xml
            .descendants()
            .find(|n| {
                n.attribute("data-id") == Some(link.id.as_str())
                    && n.attribute("class")
                        .is_some_and(|c| c.contains("pele-node"))
            })
            .unwrap();
        assert!(node
            .descendants()
            .any(|n| n.has_tag_name("a") && n.attribute("href") == Some(link.href.as_str())));
        let translate = node
            .attribute("transform")
            .unwrap()
            .strip_prefix("translate(")
            .unwrap()
            .strip_suffix(')')
            .unwrap();
        let center: Vec<f64> = translate.split(',').map(|v| v.parse().unwrap()).collect();
        let rect = node.descendants().find(|n| n.has_tag_name("rect")).unwrap();
        // Public regions are layout frames; SVG emits each scalar to 0.01 units.
        for (actual, expected) in [
            (link.bounds.x, center[0] + number(rect, "x")),
            (link.bounds.y, center[1] + number(rect, "y")),
            (link.bounds.width, number(rect, "width")),
            (link.bounds.height, number(rect, "height")),
        ] {
            assert!((actual - expected).abs() <= 0.011, "{actual} != {expected}");
        }
    }
}
#[test]
fn typed_accessibility_links_and_selected_svg_share_geometry_without_extra_callbacks() {
    let engine = Engine::with_builtins();
    let document = engine.parse(SOURCE, &ParseOptions::default()).unwrap();
    let mut options = RenderOptions::default();
    options.render.native_theme = Some(NativeTheme::default());
    let mut ordinary = Host::default();
    let expected = engine
        .render_document(&document, &options, &mut ordinary)
        .unwrap();
    let mut host = Host::default();
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    let actual = engine
        .render_document(&document, &options, &mut capture)
        .unwrap();
    assert_eq!(actual, expected);
    let frame = artifact(&capture);
    assert_eq!(frame.title.as_deref(), Some("Orders & links"));
    assert_eq!(
        frame.description.as_deref(),
        Some("Native accessible description")
    );
    assert_eq!(
        frame
            .text
            .iter()
            .map(|v| v.text.as_str())
            .collect::<Vec<_>>(),
        ["Start", "Finish", "next"]
    );
    assert_eq!(frame.links.len(), 2);
    assert_geometry(&actual, &frame);
    drop(capture);
    assert_eq!(host.calls, ordinary.calls);
}
#[test]
fn adaptive_flowchart_and_swimlane_publish_selected_frame_not_last_attempt() {
    let source = "flowchart LR\nsubgraph One\nA[A long first label] --> B[A long second label]\nend\nsubgraph Two\nC[A long third label] --> D[A long fourth label]\nend\nB --> C\nclick A \"https://example.test/a\"\nclick D \"https://example.test/d\"";
    let mut engine = Engine::empty();
    engine.register(Flowchart);
    engine.register(Swimlane);
    for (kind, source) in [
        (DiagramType::Flowchart, source.to_string()),
        (
            DiagramType::Swimlane,
            source.replacen("flowchart", "swimlane-beta", 1),
        ),
    ] {
        let doc = engine.parse(&source, &ParseOptions::default()).unwrap();
        assert_eq!(doc.diagram_type().as_str(), kind.as_str());
        let mut frames = vec![];
        for (width, auto, responsive) in [
            (1200., true, true),
            (180., true, true),
            (180., false, true),
            (1., true, false),
        ] {
            let mut options = RenderOptions::default();
            options.render.max_width = Some(width);
            options.render.auto_direction = Some(auto);
            options.render.responsive = Some(responsive);
            let mut plain = Host::default();
            let expected = engine.render_document(&doc, &options, &mut plain).unwrap();
            let mut host = Host::default();
            let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
            let actual = engine
                .render_document(&doc, &options, &mut capture)
                .unwrap();
            let frame = artifact(&capture);
            assert_eq!(actual, expected);
            assert_geometry(&actual, &frame);
            drop(capture);
            assert_eq!(host.calls, plain.calls);
            frames.push(frame);
        }
        assert_ne!(
            frames[0].links, frames[1].links,
            "narrow layout changed real regions"
        );
    }
}
#[test]
fn discarded_downward_candidate_never_replaces_the_selected_across_frame() {
    let engine = Engine::with_builtins();
    let source="flowchart LR\nA[Start] --> B[One] & C[Two] & D[Three] & E[Four]\nclick A \"https://example.test/a\"";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let mut options = RenderOptions::default();
    options.render.max_width = Some(1.);
    options.render.auto_direction = Some(false);
    let mut host = Host::default();
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    let across = engine
        .render_document(&doc, &options, &mut capture)
        .unwrap();
    let across_frame = artifact(&capture);
    drop(capture);
    let across_calls = host.calls.len();
    options.render.auto_direction = Some(true);
    let mut host = Host::default();
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    let automatic = engine
        .render_document(&doc, &options, &mut capture)
        .unwrap();
    let automatic_frame = artifact(&capture);
    drop(capture);
    assert_eq!(
        automatic, across,
        "wide downward candidate must lose to narrower across"
    );
    assert_eq!(automatic_frame, across_frame);
    assert!(
        host.calls.len() > across_calls,
        "downward candidate was actually evaluated"
    );
}

#[test]
fn approved_link_frames_follow_external_and_internal_svg_gates() {
    let engine = Engine::with_builtins();
    let source="flowchart TD\nA[Allowed] --> B[Blocked] --> C[\"//evil.test\"] --> D[Local note]\nclick A \"https://example.test/a\"\nclick B \"javascript:alert(1)\"\nclass C,D internal-link";
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let mut host = Host::default();
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    let result = engine
        .render_document(&doc, &RenderOptions::default(), &mut capture)
        .unwrap();
    let frame = artifact(&capture);
    assert_eq!(
        frame
            .links
            .iter()
            .map(|l| (l.id.as_str(), l.href.as_str(), l.internal))
            .collect::<Vec<_>>(),
        [
            ("A", "https://example.test/a", false),
            ("D", "Local note", true)
        ]
    );
    assert!(
        result.rendered.links.iter().any(|l| l.id == "C"),
        "legacy unapproved internal LinkInfo preserved"
    );
    let xml = roxmltree::Document::parse(&result.rendered.svg).unwrap();
    for link in &frame.links {
        assert!(xml
            .descendants()
            .any(|n| n.has_tag_name("a") && n.attribute("href") == Some(link.href.as_str())));
    }
    let internal = &frame.links[1];
    assert!(internal.bounds.width > 0. && internal.bounds.height > 0.);
    let mut options = RenderOptions::default();
    options.render.links = Some(false);
    engine
        .render_document(&doc, &options, &mut capture)
        .unwrap();
    assert!(artifact(&capture).links.is_empty());
    options.render.links = None;
    options.render.link_schemes = Some(vec!["mailto".into()]);
    engine
        .render_document(&doc, &options, &mut capture)
        .unwrap();
    assert_eq!(artifact(&capture).links.len(), 1);
}
#[test]
fn mutable_edits_and_failed_render_policy_retain_last_good_presentation_then_recover() {
    let public = PublicEngine::with_builtins();
    let mut doc = public.parse(SOURCE, &ParseOptions::default()).unwrap();
    let mut host = Host::default();
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    let before = doc.render(&RenderOptions::default(), &mut capture).unwrap();
    let good = artifact(&capture);
    let mut graph = doc.snapshot().unwrap();
    let key = graph.strings.len() as u32;
    graph.strings.push("Edited accessible title".into());
    let root = match graph.root {
        Atom::Ref(id) => id,
        _ => panic!("root"),
    };
    let property = graph.nodes[root as usize]
        .properties_mut()
        .iter_mut()
        .find(|p| graph.strings[p.key as usize].is("accTitle"))
        .unwrap();
    property.value = Atom::String(key);
    doc.apply(doc.revision(), graph).unwrap();
    let revision = doc.revision();
    let failed = doc
        .render_operation_with_output_limit(
            revision,
            &RenderOptions::default(),
            OutputLimit::Number(1.),
            &mut capture,
        )
        .unwrap();
    assert!(failed.result.is_err());
    assert_eq!(failed.revision, revision + 1);
    assert_eq!(artifact(&capture), good);
    let mut options = RenderOptions::default();
    let mut theme = NativeTheme::default();
    theme.accent = "invalid".into();
    options.render.native_theme = Some(theme);
    assert!(doc.render(&options, &mut capture).is_err());
    assert_eq!(artifact(&capture), good);
    let recovered = doc
        .render_operation(doc.revision(), &RenderOptions::default(), &mut capture)
        .unwrap();
    let recovered_result = recovered.result.unwrap();
    let edited = artifact(&capture);
    assert_eq!(edited.title.as_deref(), Some("Edited accessible title"));
    assert_ne!(before.rendered.svg, recovered_result.rendered.svg);
    assert_geometry(&recovered_result, &edited);
    capture.take();
    drop(capture);
    host.fail = true;
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    assert!(doc.render(&RenderOptions::default(), &mut capture).is_err());
    assert!(capture.outcome().is_none());
    drop(capture);
    host.fail = false;
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    doc.render(&RenderOptions::default(), &mut capture).unwrap();
    assert_eq!(artifact(&capture), edited);
}
#[test]
fn budgets_unsupported_and_nonfinite_geometry_are_explicit_without_changing_render_result() {
    let engine = Engine::with_builtins();
    let doc = engine.parse(SOURCE, &ParseOptions::default()).unwrap();
    let mut host = Host::default();
    let plain = engine
        .render_document(&doc, &RenderOptions::default(), &mut host)
        .unwrap();
    for budget in [
        PresentationBudget {
            max_elements: 0,
            max_text_bytes: 1024,
        },
        PresentationBudget {
            max_elements: 100,
            max_text_bytes: 1,
        },
    ] {
        let mut capture = PresentationCapture::new(&mut host, budget);
        assert_eq!(
            engine
                .render_document(&doc, &RenderOptions::default(), &mut capture)
                .unwrap(),
            plain
        );
        assert_eq!(
            capture.outcome(),
            Some(&PresentationOutcome::Unavailable(
                PresentationUnavailable::BudgetExceeded
            ))
        );
    }
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    engine
        .render_source("pie\n\"A\": 1", &RenderOptions::default(), &mut capture)
        .unwrap();
    assert_eq!(
        capture.outcome(),
        Some(&PresentationOutcome::Unavailable(
            PresentationUnavailable::UnsupportedDiagram
        ))
    );
    let mut options = RenderOptions::default();
    options.render.padding = Some(f64::NAN);
    engine
        .render_document(&doc, &options, &mut capture)
        .unwrap();
    assert_eq!(
        capture.outcome(),
        Some(&PresentationOutcome::Unavailable(
            PresentationUnavailable::InvalidGeometry
        ))
    );
}

struct ExportFailure {
    fail_after_render: Arc<AtomicBool>,
}
struct ExportState {
    model: FlowchartModel,
    graph: ModelGraph,
    fail: bool,
}
impl DiagramModule for ExportFailure {
    type Model = FlowchartModel;
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor::new(DiagramId::extension("presentation-export"))
    }
    fn parse(&self, c: ParseContext<'_>) -> Result<FlowchartModel, PeleError> {
        Flowchart.parse(c)
    }
    fn render(
        &self,
        m: &FlowchartModel,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        Flowchart.render(m, c, o, h)
    }
}
impl PublicDiagramModule for ExportFailure {
    type PublicState = ExportState;
    fn create_public(&self, m: &FlowchartModel) -> Result<ExportState, PeleError> {
        let mut w = GraphWriter::default();
        let root = w.node(Node::Object { properties: vec![] });
        Ok(ExportState {
            model: m.clone(),
            graph: w.finish(Atom::Ref(root))?,
            fail: false,
        })
    }
    fn export_public(&self, s: &ExportState) -> Result<ModelGraph, PeleError> {
        if s.fail {
            Err(PeleError::new(
                "post-render export failed",
                ErrorCode::Limit,
            ))
        } else {
            Ok(s.graph.clone())
        }
    }
    fn apply_public(&self, s: &mut ExportState, g: ModelGraph) -> Result<(), PeleError> {
        s.graph = g;
        s.fail = false;
        Ok(())
    }
    fn render_public(
        &self,
        s: &mut ExportState,
        c: &Object,
        o: &RenderSettings,
        h: &mut dyn RenderResources,
    ) -> Result<Rendered, PeleError> {
        let result = self.render(&s.model, c, o, h)?;
        s.fail = self.fail_after_render.load(Ordering::Relaxed);
        Ok(result)
    }
}
#[test]
fn failed_mutable_export_does_not_publish_provisional_presentation_and_recovery_is_owned() {
    let fail = Arc::new(AtomicBool::new(false));
    let module = ExportFailure {
        fail_after_render: fail.clone(),
    };
    let mut engine = Engine::empty();
    engine.register(Flowchart);
    let typed = engine.parse(SOURCE, &ParseOptions::default()).unwrap();
    let mut doc = PublicDocument::from_module(
        module,
        typed.model::<FlowchartModel>().unwrap(),
        Object::default(),
    )
    .unwrap();
    drop(engine);
    let recovery_graph = doc.snapshot().unwrap();
    let mut host = Host::default();
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    doc.render_operation(0, &RenderOptions::default(), &mut capture)
        .unwrap()
        .result
        .unwrap();
    let good = artifact(&capture);
    fail.store(true, Ordering::Relaxed);
    let mut options = RenderOptions::default();
    options.render.padding = Some(100.);
    assert_eq!(
        doc.render_operation(doc.revision(), &options, &mut capture)
            .unwrap_err()
            .message,
        "post-render export failed"
    );
    assert_eq!(doc.revision(), 2);
    assert_eq!(artifact(&capture), good);
    fail.store(false, Ordering::Relaxed);
    doc.apply(doc.revision(), recovery_graph).unwrap();
    doc.render_operation(doc.revision(), &options, &mut capture)
        .unwrap()
        .result
        .unwrap();
    let recovered = artifact(&capture);
    assert_ne!(recovered.links, good.links);
    assert!(recovered.width > good.width);
}
