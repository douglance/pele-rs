import Pele
let diagram = try Pele.render("flowchart LR; A[Consumer] --> B{Linked?} --> C([Yes])")
precondition(diagram.geometry.nodes.count == 3)
precondition(diagram.geometry.edges.count == 2)
precondition(diagram.svg.contains("<path"))
print("Independent SwiftPM consumer passed: \(diagram.width) × \(diagram.height)")

#if PELE_SOURCE_CORE
let engine = Pele.Engine()
for (type, source) in [
    ("state", "stateDiagram-v2\n[*] --> Ready\nReady --> [*]"),
    ("gantt", "gantt\ndateFormat YYYY-MM-DD\nA :2024-01-01,2d"),
    ("sequence", "sequenceDiagram\nApp->>Core: Render\nCore-->>App: SVG"),
    ("class", "classDiagram\nEngine --> Document"),
    ("er", "erDiagram\nUSER ||--o{ DOCUMENT : owns"),
    ("xychart", "xychart-beta\nx-axis [A,B]\nbar [1,2]"),
    ("flowchart", "flowchart TD\nsubgraph Core\nA --> B\nend"),
    ("kanban", "kanban\n  Todo\n    Task[Build]"),
    ("mindmap", "mindmap\n  root((Rust))\n    Swift\n    Web"),
    ("pie", "pie\n\"One\":1"),
    ("packet", "packet-beta\n0-7: \"Header\""),
    ("radar", "radar-beta\naxis A,B,C\ncurve C{1,2,3}"),
    ("quadrantChart", "quadrantChart\nA: [0.5,0.5]"),
    ("timeline", "timeline\n2026: Rust: Swift: Web"),
    ("journey", "journey\nBuild: 4: Rust\nVerify: 5: Web")
] {
    let document = try engine.parse(source)
    let result = try engine.render(document)
    precondition(result.type == type && result.width > 0 && result.height > 0)
    precondition(result.svg.hasPrefix("<svg"))
}
print("Independent source-built SwiftPM consumer passed: all 15 families")
#endif
