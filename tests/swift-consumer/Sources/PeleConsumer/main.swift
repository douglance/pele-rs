import Foundation
import Pele
let engine = Pele.Engine()
for (type, source) in [
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
    FileHandle.standardError.write(Data("Swift consumer: \(type)\n".utf8))
    let document = try engine.parse(source)
    var resources = Pele.HostResources(); resources.timeZone = TimeZone(secondsFromGMT: 0)!
    resources.nowMillis = { 1_704_067_200_000 }
    let result = try engine.render(document, resources: resources)
    precondition(result.type == type && result.width > 0 && result.height > 0)
    precondition(result.svg.hasPrefix("<svg"))
    let mutable = try engine.parseMutable(source)
    let schema = try mutable.schema()
    var snapshot = try mutable.snapshot()
    let raw = try snapshot.graph.intern(Pele.JSString(codeUnits: [0xd800, 65, 0xdc00]))
    try snapshot.graph.setField("consumerText", value: .string(raw))
    try snapshot.graph.setField("consumerSelf", value: snapshot.graph.root)
    try snapshot.graph.setField("consumerNumber", value: .number(Double(bitPattern: 0x7ff8000000000042)))
    try mutable.apply(snapshot.graph, expectedRevision: snapshot.revision)
    let unchanged = try mutable.render(resources: resources)
    precondition(unchanged.svg == result.svg)
    let saved = try mutable.snapshot().graph
    guard case .string(let rawID) = try saved.field("consumerText"),
          case .ref(let cycle) = try saved.field("consumerSelf"), case .ref(let root) = saved.root,
          case .number(let number) = try saved.field("consumerNumber") else { fatalError("Lost owned graph values") }
    precondition(saved.strings[Int(rawID)].codeUnits == [0xd800, 65, 0xdc00] && cycle == root)
    precondition(number.bitPattern == 0x7ff8000000000042)
    if let setter = schema.methods.first(where: { ["setDiagramTitle", "setTitle"].contains($0.name) }) {
        var graph = saved; let title = try graph.intern("Independent Swift consumer")
        try mutable.apply(graph, expectedRevision: mutable.revision)
        let outcome = try mutable.invoke(method: setter.id, receiver: graph.root, arguments: [.string(title)], resources: resources)
        guard case .returned = outcome.result else { fatalError("Native setter failed") }
        let edited = try mutable.render(resources: resources)
        precondition(edited.svg.contains("Independent Swift consumer"))
    } else {
        var graph = saved; let title = try graph.intern("Independent Swift consumer")
        try graph.setField("title", value: .string(title))
        try mutable.apply(graph, expectedRevision: mutable.revision)
        let edited = try mutable.render(resources: resources)
        var body = source
        if body.hasPrefix("---\n"), let header = body.range(of: "\n---\n") { body = String(body[header.upperBound...]) }
        // State treats a title-looking DSL line as an ordinary state statement.
        if type != "state" {
            body = body.components(separatedBy: "\n").map { $0.hasPrefix("title ") ? "title Independent Swift consumer" : $0 }.joined(separator: "\n")
        }
        let equivalent = try engine.render("---\ntitle: Independent Swift consumer\n---\n" + body, resources: resources)
        precondition(edited.svg == equivalent.svg && edited.svg != result.svg, "Edited/source mismatch: \(type)")
    }
    try mutable.close()
    do { _ = try mutable.snapshot(); fatalError("Closed document accepted") } catch { }

}
print("Independent current-API SwiftPM consumer passed: all 33 immutable and mutable families")
// C13: construct a real model graph without parsing, retaining aliases/cycles.
let preparedSchema = try engine.preparedSchema(for: "pie")
precondition(!preparedSchema.id.isEmpty && preparedSchema.version > 0)
var callerGraph = Pele.PublicGraph(root: .ref(0), strings: [], nodes: [
    .object(properties: []), .map(entries: [], properties: [])
])
let pieType = try callerGraph.intern("pie")
try callerGraph.setField("type", value: .string(pieType))
for name in ["title", "accTitle", "accDescr"] { try callerGraph.setField(name, value: .undefined) }
try callerGraph.setField("showData", value: .bool(false))
try callerGraph.setField("sections", value: .ref(1))
try callerGraph.setField("consumerAlias", value: .ref(1))
try callerGraph.setField("consumerCycle", value: .ref(0))
let one = try callerGraph.intern("One"), two = try callerGraph.intern("Two")
callerGraph.nodes[1] = .map(entries: [
    .init(key: .string(one), value: .number(1)), .init(key: .string(two), value: .number(2))
], properties: [])
let constructed = try engine.createPrepared("pie", graph: callerGraph)
let prepared = try engine.parsePrepared("pie", source: "pie\n\"One\":1\n\"Two\":2\n")
let constructedSVG = try constructed.render().svg
let preparedSVG = try prepared.render().svg
precondition(constructedSVG == preparedSVG)
let constructedGraph = try constructed.snapshot().graph
if case .ref(let alias) = try constructedGraph.field("consumerAlias"),
   case .ref(let cycle) = try constructedGraph.field("consumerCycle") {
    precondition(alias == 1 && cycle == 0 && constructed.revision == 1)
} else { fatalError("Caller-owned graph lost alias or cycle") }
try constructed.close(); try prepared.close()

// C14: installed-font measurement and strict image approval, without a window.
@MainActor func checkStrictNativeResources() throws {
    let owner = Pele.NativeResources()
    let source = "flowchart LR\nA[Native fonts] --> B[Approved resources]"
    let first = try engine.renderNative(source, nativeResources: owner)
    precondition(first.type == "flowchart" && first.width > 0 && first.height > 0)
    precondition(first.svg.contains("Native fonts"))
    do {
        _ = try engine.renderNative("flowchart LR\nA@{img: \"https://example.test/picture.png\",label: \"Picture\",w:80,h:60}", nativeResources: owner)
        fatalError("Strict native renderer accepted an unapproved image")
    } catch let error as PeleError {
        precondition(error.code == "resource" && error.message == "Native image requires explicit host approval")
    }
    let recovered = try engine.renderNative(source, nativeResources: owner)
    precondition(recovered.svg == first.svg)
}
try checkStrictNativeResources()
print("Independent SwiftPM consumer passed: C13 caller-owned graph + C14 strict resources/recovery")
