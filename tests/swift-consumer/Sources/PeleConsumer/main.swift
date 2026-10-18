import Pele
let diagram = try Pele.render("flowchart LR; A[Consumer] --> B{Linked?} --> C([Yes])")
precondition(diagram.geometry.nodes.count == 3)
precondition(diagram.geometry.edges.count == 2)
precondition(diagram.svg.contains("<path"))
print("Independent SwiftPM consumer passed: \(diagram.width) × \(diagram.height)")
