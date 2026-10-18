//! Public native presentation producer batch: real AgentFlow, ER and Kanban layouts.
use pele::compat::{
    public::{Atom, ModelGraph, Property, PublicEngine},
    *,
};
#[derive(Default)]
struct Host {
    trace: Vec<String>,
    fail: bool,
}
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        self.trace.push(format!(
            "text:{:?}:{:016x}:{}",
            r.text,
            r.size.to_bits(),
            r.style
        ));
        if self.fail {
            return Err(PeleError::new("producer host stopped", ErrorCode::Semantic));
        }
        Ok(r.text.encode_utf16().count() as f64
            * r.size
            * if r.style & 1 != 0 { 0.71 } else { 0.6 })
    }
    fn icon(&mut self, name: &str) -> Result<Option<TrustedSvg>, PeleError> {
        self.trace.push(format!("icon:{name}"));
        Ok(Some(TrustedSvg::from_trusted_markup(
            "<path d=\"M0 0L24 24\"/>".into(),
        )))
    }
}
const AGENT:&str="---\ntitle: Agent board\n---\nagentflow-beta LR\naccTitle: Accessible workflow\naccDescr: Workflow description\na[\"First task\"] --> b[\"Local note\"]\nclick a href \"https://example.test/task\"\nclass b internal-link";
const ER:&str="---\ntitle: Entity board\n---\nerDiagram\ndirection LR\naccTitle: Accessible schema\naccDescr: Schema description\nCustomer:::internal-link {\n int id PK\n string name \"Display name\"\n}\nCustomer ||--o{ Order : places\nOrder:::internal-link {\n int id PK\n}";
const KANBAN:&str="---\ntitle: Ticket board\nconfig:\n  kanban:\n    ticketBaseUrl: 'https://example.test/#TICKET#'\n---\nkanban\n todo[Todo]\n  a[First card]@{ ticket: T-1, assigned: Person, icon: star }\n done[Done]\n  b[Second card]@{ ticket: T-2 }\n later[Later]\n  c[Third card]@{ ticket: T-3 }";
fn artifact(c: &PresentationCapture<'_>) -> PresentationArtifact {
    match c.outcome().unwrap() {
        PresentationOutcome::Available(f) => f.clone(),
        v => panic!("{v:?}"),
    }
}
fn n(node: roxmltree::Node<'_, '_>, key: &str) -> f64 {
    node.attribute(key).unwrap().parse().unwrap()
}
fn translated(node: roxmltree::Node<'_, '_>) -> (f64, f64) {
    node.ancestors()
        .filter_map(|n| n.attribute("transform"))
        .fold((0., 0.), |(x, y), t| {
            let values: Vec<f64> = t
                .strip_prefix("translate(")
                .unwrap()
                .strip_suffix(')')
                .unwrap()
                .split(',')
                .map(|v| v.parse().unwrap())
                .collect();
            (x + values[0], y + values[1])
        })
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 0.021, "{a} != {b}");
}
fn geometry(result: &RenderResult, frame: &PresentationArtifact) {
    assert_eq!(
        (frame.width, frame.height),
        (result.rendered.width, result.rendered.height)
    );
    assert_eq!(frame.version, 1);
    let xml = roxmltree::Document::parse(&result.rendered.svg).unwrap();
    for action in &frame.links {
        let node = xml
            .descendants()
            .find(|n| {
                n.attribute("data-id") == Some(action.id.as_str())
                    && n.attribute("class")
                        .is_some_and(|v| v.contains("pele-node"))
            })
            .unwrap();
        let anchor = node
            .descendants()
            .find(|n| n.has_tag_name("a") && n.attribute("href") == Some(action.href.as_str()))
            .unwrap();
        let (x, y) = translated(node);
        if result.diagram_type.as_str() == "agentflow" && !action.internal {
            let rect = node.descendants().find(|n| n.has_tag_name("rect")).unwrap();
            close(action.bounds.x, x + n(rect, "x"));
            close(action.bounds.y, y + n(rect, "y"));
            close(action.bounds.width, n(rect, "width"));
            close(action.bounds.height, n(rect, "height"));
        } else {
            let text = anchor
                .descendants()
                .find(|n| n.has_tag_name("text"))
                .unwrap();
            let at = if text.has_attribute("x") {
                text
            } else {
                text.descendants()
                    .find(|n| n.has_tag_name("tspan") && n.has_attribute("x"))
                    .unwrap()
            };
            let size = if result.diagram_type.as_str() == "kanban" {
                n(node, "font-size")
            } else {
                16.
            };
            let expected_left = x + n(at, "x")
                - if result.diagram_type.as_str() == "kanban" {
                    0.
                } else {
                    action.bounds.width / 2.
                };
            close(action.bounds.x, expected_left);
            close(
                action.bounds.y,
                y + n(at, "y") - size * 0.35 - action.bounds.height / 2.,
            );
            let content = anchor
                .descendants()
                .filter(|n| n.is_text())
                .map(|n| n.text().unwrap())
                .collect::<String>();
            close(
                action.bounds.width,
                content.encode_utf16().count() as f64
                    * size
                    * if text.attribute("font-weight").is_some() {
                        0.71
                    } else {
                        0.6
                    },
            );
        }
    }
}
#[test]
fn three_real_producers_match_ordinary_svg_callbacks_accessibility_and_selected_geometry() {
    let engine = Engine::with_builtins();
    for (source, title, descr, count) in [
        (
            AGENT,
            "Accessible workflow",
            Some("Workflow description"),
            2,
        ),
        (ER, "Accessible schema", Some("Schema description"), 2),
        (KANBAN, "Ticket board", None, 3),
    ] {
        let doc = engine.parse(source, &ParseOptions::default()).unwrap();
        let mut frames = vec![];
        for (width, auto, responsive) in [
            (1400., true, true),
            (190., true, true),
            (190., false, false),
        ] {
            let mut options = RenderOptions::default();
            options.render.max_width = Some(width);
            options.render.auto_direction = Some(auto);
            options.render.responsive = Some(responsive);
            options.render.native_theme = Some(NativeTheme::default());
            let mut ordinary = Host::default();
            let expected = engine
                .render_document(&doc, &options, &mut ordinary)
                .unwrap();
            let mut host = Host::default();
            let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
            let actual = engine
                .render_document(&doc, &options, &mut capture)
                .unwrap();
            let frame = artifact(&capture);
            assert_eq!(actual, expected);
            assert_eq!(frame.title.as_deref(), Some(title));
            assert_eq!(frame.description.as_deref(), descr);
            assert_eq!(frame.links.len(), count);
            assert!(!frame.text.is_empty());
            geometry(&actual, &frame);
            frames.push(frame);
            drop(capture);
            assert_eq!(host.trace, ordinary.trace);
        }
        assert_ne!(
            frames[0].links,
            frames[1].links,
            "responsive regions changed: {}",
            doc.diagram_type()
        );
        let frame = &frames[0];
        if doc.diagram_type().as_str() == "er" {
            for text in [
                "int",
                "id",
                "PK",
                "string",
                "name",
                "Display name",
                "places",
            ] {
                assert!(
                    frame.text.iter().any(|v| v.text == text),
                    "ER cell/relationship text {text}"
                );
            }
        }
        if doc.diagram_type().as_str() == "kanban" {
            for text in ["Todo", "First card", "T-1", "Person"] {
                assert!(
                    frame.text.iter().any(|v| v.text == text),
                    "Kanban text {text}"
                );
            }
        }
    }
}
#[test]
fn new_producers_keep_last_good_on_mutable_output_theme_and_host_failures_and_recover() {
    let public = PublicEngine::with_builtins();
    for source in [AGENT, ER, KANBAN] {
        let mut doc = public.parse(source, &ParseOptions::default()).unwrap();
        let mut host = Host::default();
        let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
        doc.render_operation(0, &RenderOptions::default(), &mut capture)
            .unwrap()
            .result
            .unwrap();
        let good = artifact(&capture);
        let failed = doc
            .render_operation_with_output_limit(
                doc.revision(),
                &RenderOptions::default(),
                OutputLimit::Number(1.),
                &mut capture,
            )
            .unwrap();
        assert!(failed.result.is_err());
        assert_eq!(artifact(&capture), good);
        let mut bad = RenderOptions::default();
        let mut theme = NativeTheme::default();
        theme.border = "invalid".into();
        bad.render.native_theme = Some(theme);
        assert!(doc.render(&bad, &mut capture).is_err());
        assert_eq!(artifact(&capture), good);
        drop(capture);
        host.fail = true;
        let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
        assert!(doc.render(&RenderOptions::default(), &mut capture).is_err());
        assert!(capture.outcome().is_none());
        drop(capture);
        host.fail = false;
        let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
        let recovered = doc
            .render_operation(doc.revision(), &RenderOptions::default(), &mut capture)
            .unwrap();
        geometry(&recovered.result.unwrap(), &artifact(&capture));
        assert_eq!(artifact(&capture), good);
    }
}
#[test]
fn link_gates_metadata_budgets_and_invalid_geometry_remain_explicit() {
    let engine = Engine::with_builtins();
    for source in [AGENT, ER, KANBAN] {
        let doc = engine.parse(source, &ParseOptions::default()).unwrap();
        let mut host = Host::default();
        let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
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
        assert!(artifact(&capture).links.iter().all(|l| l.internal));
        drop(capture);
        let mut capture = PresentationCapture::new(
            &mut host,
            PresentationBudget {
                max_elements: 1,
                max_text_bytes: 1,
            },
        );
        engine
            .render_document(&doc, &RenderOptions::default(), &mut capture)
            .unwrap();
        assert_eq!(
            capture.outcome(),
            Some(&PresentationOutcome::Unavailable(
                PresentationUnavailable::BudgetExceeded
            ))
        );
        drop(capture);
        let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
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
    for source in [
        AGENT.replace("https://example.test/task", "javascript:alert(1)"),
        KANBAN.replace(
            "https://example.test/#TICKET#",
            "javascript:alert(#TICKET#)",
        ),
    ] {
        let doc = engine.parse(&source, &ParseOptions::default()).unwrap();
        let mut host = Host::default();
        let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
        engine
            .render_document(&doc, &RenderOptions::default(), &mut capture)
            .unwrap();
        assert!(artifact(&capture).links.iter().all(|l| l.internal));
    }
}

fn set_string(graph: &mut ModelGraph, node: u32, key: &str, value: &str) {
    let value_id = graph.strings.len() as u32;
    graph.strings.push(value.into());
    if let Some(property) = graph.nodes[node as usize]
        .properties_mut()
        .iter_mut()
        .find(|p| graph.strings[p.key as usize].is(key))
    {
        property.value = Atom::String(value_id);
    } else {
        let key_id = graph.strings.len() as u32;
        graph.strings.push(key.into());
        graph.nodes[node as usize].properties_mut().push(Property {
            key: key_id,
            value: Atom::String(value_id),
            flags: 7,
        });
    }
}
fn record(graph: &ModelGraph, key: &str, value: &str) -> u32 {
    graph
        .nodes
        .iter()
        .position(|n| {
            n.properties().iter().any(|p| {
                graph.strings[p.key as usize].is(key)
                    && matches!(p.value,Atom::String(i) if graph.strings[i as usize].is(value))
            })
        })
        .expect("mutable record") as u32
}
#[test]
fn arena_edits_follow_cached_agent_graph_and_live_er_kanban_geometry_without_reparsing() {
    let public = PublicEngine::with_builtins();
    for (source, find_key, find_value, label_key, new_label) in [
        (AGENT, "id", "a", "text", "Edited task"),
        (ER, "label", "Customer", "alias", "Edited entity"),
        (KANBAN, "id", "a", "label", "Edited card"),
    ] {
        let mut doc = public.parse(source, &ParseOptions::default()).unwrap();
        let mut host = Host::default();
        let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
        doc.render(&RenderOptions::default(), &mut capture).unwrap();
        let old = artifact(&capture);
        let mut graph = doc.snapshot().unwrap();
        let root = match graph.root {
            Atom::Ref(i) => i,
            _ => panic!("root"),
        };
        let node = record(&graph, find_key, find_value);
        set_string(
            &mut graph,
            root,
            if doc.diagram_type().as_str() == "kanban" {
                "title"
            } else {
                "accTitle"
            },
            "Edited title",
        );
        set_string(&mut graph, node, label_key, new_label);
        if doc.diagram_type().as_str() == "agentflow" {
            set_string(&mut graph, node, "link", "https://example.test/edited");
        }
        if doc.diagram_type().as_str() == "kanban" {
            set_string(&mut graph, node, "ticket", "T-9");
            set_string(&mut graph, root, "accTitle", "ignored extra");
        }
        doc.apply(doc.revision(), graph).unwrap();
        let revision = doc.revision();
        let outcome = doc
            .render_operation(revision, &RenderOptions::default(), &mut capture)
            .unwrap();
        let result = outcome.result.unwrap();
        let frame = artifact(&capture);
        assert_eq!(outcome.revision, revision + 1);
        assert_eq!(frame.title.as_deref(), Some("Edited title"));
        if doc.diagram_type().as_str() == "agentflow" {
            // Pinned agentflow/render.ts graphs WeakMap caches labels/links at parse;
            // public metadata stays live. The artifact must follow the rendered
            // private graph, not invent regions for edited semantic-only records.
            assert_eq!(frame.text, old.text);
            assert_eq!(frame.links, old.links);
            let expected_source = source.replace("Accessible workflow", "Edited title");
            let expected = Engine::with_builtins()
                .render_source(
                    &expected_source,
                    &RenderOptions::default(),
                    &mut Host::default(),
                )
                .unwrap();
            assert_eq!(result, expected);
        } else {
            assert!(frame.text.iter().any(|t| t.text == new_label));
            assert!(result.rendered.svg.contains(new_label));
        }
        assert_ne!(frame, old);
        geometry(&result, &frame);
        if doc.diagram_type().as_str() == "kanban" {
            assert!(frame
                .links
                .iter()
                .any(|l| l.href == "https://example.test/T-9"));
        }
        if doc.diagram_type().as_str() == "er" {
            let mut graph = doc.snapshot().unwrap();
            let node = record(&graph, "label", "Customer");
            set_string(&mut graph, node, "alias", "javascript:alert(1)");
            doc.apply(doc.revision(), graph).unwrap();
            let result = doc.render(&RenderOptions::default(), &mut capture).unwrap();
            assert!(!artifact(&capture).links.iter().any(|l| l.id == "Customer"));
            assert!(result.rendered.links.iter().any(|l| l.id == "Customer"));
        }
    }
}
#[test]
fn discarded_direction_attempts_never_publish_the_last_unselected_frame() {
    let engine = Engine::with_builtins();
    for source in ["agentflow-beta LR\na --> b & c & d & e\nclick a href \"https://example.test/a\"", "erDiagram\ndirection LR\nA ||--|| B : owns\nA ||--|| C : creates\nA ||--|| D : places\nA ||--|| E : uses\nclass A internal-link"]{
        let doc=engine.parse(source,&ParseOptions::default()).unwrap();let mut options=RenderOptions::default();options.render.max_width=Some(1.);options.render.auto_direction=Some(false);
        let mut host=Host::default();let mut capture=PresentationCapture::new(&mut host,PresentationBudget::default());let across=engine.render_document(&doc,&options,&mut capture).unwrap();let first=artifact(&capture);drop(capture);let first_calls=host.trace.len();
        options.render.auto_direction=Some(true);let mut host=Host::default();let mut capture=PresentationCapture::new(&mut host,PresentationBudget::default());let automatic=engine.render_document(&doc,&options,&mut capture).unwrap();let selected=artifact(&capture);drop(capture);
        assert_eq!(across,automatic,"{} retains narrower across candidate",doc.diagram_type());assert_eq!(first,selected);assert!(host.trace.len()>first_calls);
    }
}
#[test]
fn agent_collapsed_and_border_held_paths_capture_only_actual_visible_text() {
    let source="agentflow-beta LR\nflow f[Agent]\na[Plan] --> b[Execute]\nb -->|return| f\nend\nflow hidden[Collapsed]@{ view: collapsed }\nx[Invisible one] --> y[Invisible two]\nend\ny --> a\nclick a href \"https://example.test/a\"";
    let engine = Engine::with_builtins();
    let doc = engine.parse(source, &ParseOptions::default()).unwrap();
    let mut host = Host::default();
    let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
    let result = engine
        .render_document(&doc, &RenderOptions::default(), &mut capture)
        .unwrap();
    let frame = artifact(&capture);
    for label in ["Agent", "Plan", "Execute", "Collapsed", "return"] {
        assert!(
            frame.text.iter().any(|t| t.text == label),
            "visible label {label}"
        );
        assert!(result.rendered.svg.contains(label));
    }
    assert!(!frame.text.iter().any(|t| t.text.contains("Invisible")));
    assert!(!result.rendered.svg.contains("Invisible"));
    geometry(&result, &frame);
}
