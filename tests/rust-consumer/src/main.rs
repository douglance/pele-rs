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
        (pele::compat::DiagramType::Class, "classDiagram\nclass Engine\nEngine --> Document"),
        (pele::compat::DiagramType::Er, "erDiagram\nUSER ||--o{ DOCUMENT : owns"),
        (pele::compat::DiagramType::Xychart, "xychart-beta\nx-axis [A,B]\nbar [1,2]"),
        (pele::compat::DiagramType::Flowchart, "flowchart TD\nsubgraph Core\nA --> B\nend\nB --> C"),
        (pele::compat::DiagramType::Kanban, "kanban\n  Todo\n    Task[Build Rust]"),
        (pele::compat::DiagramType::Mindmap, "mindmap\n  root((Rust))\n    Swift\n    Web"),
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
        let rendered = engine.render_document(
            &document,
            &Default::default(),
            &mut pele::compat::text::MetricsResources,
        )?;
        assert!(rendered.rendered.width > 0. && rendered.rendered.height > 0.);
        assert!(rendered.rendered.svg.starts_with("<svg"));
    }
    println!("Packaged Rust compatibility consumer passed: class, er, xychart, flowchart, kanban, mindmap, pie, packet, radar, quadrantChart, timeline, journey");
    Ok(())
}
