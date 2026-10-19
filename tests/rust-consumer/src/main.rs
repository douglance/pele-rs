fn main() -> Result<(), Box<dyn std::error::Error>> {
    let diagram = pele::render(
        "graph TD; A[Packaged crate] --> B{Usable?} --> C([Yes])",
        &Default::default(),
    )?;
    assert_eq!(diagram.geometry.nodes.len(), 3);
    assert_eq!(diagram.geometry.edges.len(), 2);
    assert!(diagram.svg.contains("<path"));
    println!(
        "Packaged Rust crate consumer passed: {} × {}",
        diagram.width, diagram.height
    );
    let engine = pele::compat::Engine::with_builtins();
    for (kind, source) in [
        (
            pele::compat::DiagramType::State,
            "stateDiagram-v2\n[*] --> Ready\nReady --> [*]",
        ),
        (
            pele::compat::DiagramType::Gantt,
            "gantt\ndateFormat YYYY-MM-DD\nA :2024-01-01,2d",
        ),
        (
            pele::compat::DiagramType::Sequence,
            "sequenceDiagram\nApp->>Core: Render\nCore-->>App: SVG",
        ),
        (
            pele::compat::DiagramType::Class,
            "classDiagram\nclass Engine\nEngine --> Document",
        ),
        (
            pele::compat::DiagramType::Er,
            "erDiagram\nUSER ||--o{ DOCUMENT : owns",
        ),
        (
            pele::compat::DiagramType::Xychart,
            "xychart-beta\nx-axis [A,B]\nbar [1,2]",
        ),
        (
            pele::compat::DiagramType::Flowchart,
            "flowchart TD\nsubgraph Core\nA --> B\nend\nB --> C",
        ),
        (
            pele::compat::DiagramType::Kanban,
            "kanban\n  Todo\n    Task[Build Rust]",
        ),
        (
            pele::compat::DiagramType::Mindmap,
            "mindmap\n  root((Rust))\n    Swift\n    Web",
        ),
        (pele::compat::DiagramType::Pie, "pie\n\"One\":1"),
        (
            pele::compat::DiagramType::Packet,
            "packet-beta\n0-7: \"Header\"",
        ),
        (
            pele::compat::DiagramType::Radar,
            "radar-beta\naxis A,B,C\ncurve C{1,2,3}",
        ),
        (
            pele::compat::DiagramType::QuadrantChart,
            "quadrantChart\nA: [0.5,0.5]",
        ),
        (
            pele::compat::DiagramType::Timeline,
            "timeline\n2026: Rust: Swift: Web",
        ),
        (
            pele::compat::DiagramType::Journey,
            "journey\nImplement: 4: Rust\nVerify: 5: Web",
        ),
    ] {
        let document = engine.parse(source, &Default::default())?;
        assert_eq!(*document.diagram_type(), kind.into());
        let mut options = pele::compat::RenderOptions::default();
        options.render.now = Some(1_704_067_200_000.0);
        let rendered = engine.render_document(
            &document,
            &options,
            &mut pele::compat::text::MetricsResources,
        )?;
        assert!(rendered.rendered.width > 0. && rendered.rendered.height > 0.);
        assert!(rendered.rendered.svg.starts_with("<svg"));
    }
    println!("Packaged Rust compatibility consumer passed: state, gantt, sequence, class, er, xychart, flowchart, kanban, mindmap, pie, packet, radar, quadrantChart, timeline, journey");
    let public = pele::compat::public::PublicEngine::with_builtins();
    for (kind, source) in [
        ("pie", "pie\n\"One\":1"),
        ("venn", "---\ntitle: Shared sets\n---\nvenn-beta\nset A[\"Swift\"]:20\nset B[\"Web\"]:14\nunion A,B[\"Rust\"]:4"),
        ("ishikawa", "---\ntitle: Delivery quality\n---\nishikawa-beta\n    Release ready\n        Implementation\n            Rust engine\n            Host adapters\n        Verification\n            Swift and web\n            Integration tests"),
        ("swimlane", "---\ntitle: Shared workflow\n---\nswimlane-beta LR\nsubgraph Swift\nA[Native app]\nend\nsubgraph Rust\nB[Core engine]\nend\nsubgraph Web\nC[Browser]\nend\nA --> B --> C"),
        ("wardley", "wardley-beta\ntitle Dependency map\nevolution Genesis -> Custom -> Product -> Commodity\ncomponent App [0.8, 0.3]\ncomponent Core [0.4, 0.7]\nApp -> Core"),
        ("eventmodeling", "---\ntitle: Command lifecycle\n---\neventmodeling\n    tf 01 ui UI\n    tf 02 cmd RunAction\n    tf 03 evt ActionExecuted"),
        ("usecase", "---\ntitle: User journeys\n---\nusecase-beta\ndirection LR\nactor Customer(\"Customer\")\nsystemBoundary \"Order system\"\n  Checkout(\"Place order\")\nend\nCustomer --> Checkout"),
        ("cynefin", "---\ntitle: Decision domains\n---\ncynefin-beta\n  complex\n    \"Emergent practice\"\n  clear\n    \"Best practice\""),
        ("treeView", "---\ntitle: Source tree\n---\ntreeView-beta\nroot/\n    src/\n        index.js\n        App.tsx\n    tests/\n        RendererTests.swift\n    Package.swift\n    README.md"),
        ("agentflow", "---\ntitle: Writing workflow\n---\nagentflow-beta LR\nbrief[\"Brief\"]@{ shape: input }\ndraft[\"Draft\"]@{ shape: task }\ncheck[\"Check\"]@{ shape: tool }\nbrief --> draft --> check"),
        ("requirement", "---\ntitle: Shared requirements\n---\nrequirementDiagram\nrequirement core {\n id: 1\n text: Shared renderer\n risk: high\n verifymethod: test\n}\nelement app {\n type: service\n}\napp - satisfies -> core"),
        ("block", "block-beta\ncolumns 2\napp[\"Swift\"] core[\"Rust\"]\napp --> core"),
        ("railroad", "railroad-abnf-beta\nrule = \"test\" / \"more\" ;"),
        ("treemap", "treemap-beta\n\"Category A\"\n  \"Swift\": 10\n  \"Web\": 20\n\"Category B\"\n  \"Rust\": 30"),
        ("c4", "C4Context\ntitle Shared system\nPerson(user, \"User\", \"Author\")\nSystem(core, \"Rust\", \"Renderer\")\nRel(user, core, \"Uses\")"),
        ("info", "info\ntitle Shared engine"),
        ("gitGraph", "gitGraph\ntitle Shared branches\ncommit id: \"base\"\nbranch feature\ncommit\ncheckout main\nmerge feature"),
        ("architecture", "architecture-beta\ntitle Application layout\nservice app(server)[App]\nservice db(database)[Data]\napp:R --> L:db"),
        ("sankey", "sankey\nCapture,Store,10\nStore,Swift,6\nStore,Web,4"),
        ("state", "stateDiagram-v2\ntitle Lifecycle\n[*] --> Ready\nstate Active {\n [*] --> Working\n Working --> [*]\n}\nReady --> Active"),
        ("gantt", "gantt\ntitle Release plan\ndateFormat YYYY-MM-DD\nsection Core\nBuild :done,a,2024-03-01,2d\nVerify :crit,b,after a,2d"),
        ("sequence", "sequenceDiagram\ntitle Request lifecycle\nparticipant App\nparticipant Core\nApp->>Core: Render\nactivate Core\nCore-->>App: SVG\ndeactivate Core"),
        ("xychart", "xychart-beta\ntitle Revenue\nx-axis [Jan, Feb, Mar]\ny-axis Sales 0 --> 100\nbar [20, 40, 70]\nline [25, 45, 80]"),
        ("class", "classDiagram\nclass Engine {\n +render() SVG\n}\nEngine --> Document : renders"),
        ("er", "erDiagram\nUSER ||--o{ DOCUMENT : owns\nUSER {\n int id PK\n string name\n}\nDOCUMENT {\n int id PK\n}"),
        ("flowchart", "flowchart TD\nsubgraph one[My Group]\nA --> B\nend\nC --> A"),
        ("kanban", "kanban\n  Todo\n    task[Build Rust]\n  Done\n    ship[Ship Swift]"),
        ("mindmap", "mindmap\n  root((Rust))\n    Swift\n    Web"),
        ("packet", "packet-beta\ntitle Network header\n0-7: \"Version\"\n8-15: \"Flags\"\n16-31: \"Length\""),
        ("radar", "radar-beta\ntitle Skills\naxis a[\"Speed\"], b[\"Quality\"], c[\"Cost\"]\ncurve team[\"Team\"]{4, 3, 5}"),
        ("timeline", "timeline\ntitle Releases\nsection Core\n2025: Rust\n2026: Swift: Web"),
        ("journey", "journey\ntitle Build and ship\nsection Core\nImplement: 4: Rust\nVerify: 5: Swift, Web"),
        ("quadrantChart", "quadrantChart\ntitle Work priorities\nx-axis Low --> High\ny-axis Low --> High\nFast: [0.8, 0.7]\nCareful: [0.3, 0.4]")
    ] {
        use pele::compat::public::{Atom, JsString, Property};
        let mut document = public.parse(source, &Default::default())?;
        assert_eq!(document.diagram_type().as_str(), kind);
        let mut graph = document.snapshot()?;
        let Atom::Ref(root) = graph.root else { panic!("Object root") };
        let key = graph.strings.len() as u32; graph.strings.push("consumerText".into());
        let text = graph.strings.len() as u32; graph.strings.push(JsString(vec![0xd800, 65, 0xdc00]));
        graph.nodes[root as usize].properties_mut().push(Property { key, value: Atom::String(text), flags: 7 });
        document.apply(0, graph)?;
        let mut options = pele::compat::RenderOptions::default(); options.render.now = Some(1_704_067_200_000.0);
        let rendered = document.render_operation(document.revision(), &options, &mut pele::compat::text::MetricsResources)?.result.map_err(|e| format!("{kind}: {e:?}"))?;
        assert_eq!(rendered.diagram_type.as_str(), kind);
        assert!(rendered.rendered.svg.starts_with("<svg"));
        let saved = document.snapshot()?;
        let Some(Atom::String(id)) = saved.field("consumerText")? else { panic!("Retained field") };
        assert_eq!(saved.strings[*id as usize].0, [0xd800, 65, 0xdc00]);
    }
    println!("Packaged mutable Rust consumer passed: all33 families");
    Ok(())
}
