import Foundation
import XCTest
import Pele

final class RegistryTests: XCTestCase {
    func testAllBuiltinsRegisterSelectivelyAndDocumentsSurviveClose() throws {
        let examples: [(String, String)] = [
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
]
        XCTAssertEqual(examples.count, 33)
        let registry = try Pele.Registry.empty()
        XCTAssertEqual(try registry.capabilities().count, 33)
        XCTAssertTrue(try registry.capabilities().allSatisfy { $0.known && !$0.registered })
        var documents: [(String, Pele.MutableDocument, String)] = []
        for (kind, source) in examples {
            XCTAssertFalse(try registry.registered(kind), kind)
            XCTAssertEqual(try registry.detectType(source), kind, kind)
            XCTAssertFalse(try registry.supports(source), kind)
            try registry.registerBuiltin(kind)
            XCTAssertTrue(try registry.registered(kind), kind)
            XCTAssertTrue(try registry.supports(source), kind)
            let document = try registry.parseMutable(source)
            XCTAssertEqual(document.type, kind)
            documents.append((kind, document, source))
        }
        XCTAssertEqual(try registry.capabilities().filter(\.registered).count, 33)
        for (kind, _) in examples { try registry.registerBuiltin(kind) }
        XCTAssertEqual(try registry.capabilities().count, 33)
        try registry.close(); try registry.close()
        var resources = Pele.HostResources(); resources.timeZone = TimeZone(secondsFromGMT: 0)!
        resources.nowMillis = { 1_704_067_200_000 }
        for (kind, document, source) in documents {
            let snapshot = try document.snapshot()
            XCTAssertEqual(snapshot.type, kind)
            let rendered = try document.render(resources: resources)
            let expected = try Pele.Engine().render(source, resources: resources)
            XCTAssertEqual(rendered.type, kind); XCTAssertEqual(rendered.svg, expected.svg, kind)
            var graph = try document.snapshot().graph
            let raw = try graph.intern(Pele.JSString(codeUnits: [0xd800, 0, 0xdc00]))
            try graph.setField("retainedRegistryText", value: .string(raw))
            try graph.setField("retainedRegistrySelf", value: graph.root)
            try document.apply(graph, expectedRevision: document.revision)
            let retained = try document.snapshot().graph
            guard case .string(let string) = try retained.field("retainedRegistryText"),
                  case .ref(let alias) = try retained.field("retainedRegistrySelf"), case .ref(let root) = retained.root else { return XCTFail("Lost retained graph values: \(kind)") }
            XCTAssertEqual(retained.strings[Int(string)].codeUnits, [0xd800, 0, 0xdc00]); XCTAssertEqual(alias, root)
            try document.close()
        }
        XCTAssertThrowsError(try registry.capabilities()) { XCTAssertEqual(($0 as? PeleError)?.message, "Registry is closed") }
    }
    func testDiscoveryHasSeparateGrammarSourceLimitAndRegistrationBoundaries() throws {
        let empty = try Pele.Registry.empty(), selective = try Pele.Registry.empty(), all = try Pele.Registry.allBuiltins()
        defer { try? empty.close(); try? selective.close(); try? all.close() }
        XCTAssertEqual(try all.capabilities().filter(\.registered).count, 33)
        try selective.registerBuiltin("info")
        XCTAssertFalse(try empty.registered("info")); XCTAssertTrue(try selective.registered("info"))
        for id in ["Info", " flowchart", "custom/diagram", ""] {
            XCTAssertFalse(try selective.registered(id), id)
            XCTAssertThrowsError(try selective.registerBuiltin(id), id)
        }
        XCTAssertThrowsError(try empty.parseMutable("info")) {
            let error = $0 as? CompatibilityError
            XCTAssertEqual(error?.code, "unsupported-diagram"); XCTAssertEqual(error?.type, "info")
            XCTAssertEqual(error?.message, "Diagram type \"info\" is not supported yet.")
        }
        let large = "info\n%%" + String(repeating: "x", count: 60_000)
        XCTAssertEqual(try selective.detectType(large), "info"); XCTAssertTrue(try selective.supports(large))
        XCTAssertThrowsError(try selective.parseMutable(large)) { XCTAssertEqual(($0 as? CompatibilityError)?.code, "limit") }
        let accepted = try selective.parseMutable(large, limits: .init(source: .number(.infinity)))
        try selective.close(); XCTAssertEqual(try accepted.render().type, "info"); try accepted.close()
        let badGrammar = "flowchart TD\nA --> ["
        XCTAssertEqual(try all.detectType(badGrammar), "flowchart"); XCTAssertTrue(try all.supports(badGrammar))
        XCTAssertThrowsError(try all.parseMutable(badGrammar))
        let badYAML = "---\ntitle: \"\\q\"\n---\npie\n\"A\": 1"
        XCTAssertThrowsError(try all.detectType(badYAML)) { XCTAssertEqual(($0 as? CompatibilityError)?.code, "syntax") }
        XCTAssertFalse(try all.supports(badYAML))
        XCTAssertNil(try all.detectType("unrecognized diagram")); XCTAssertFalse(try all.supports("unrecognized diagram"))
        let transport = String(repeating: "x", count: 16 * 1024 * 1024 + 1)
        XCTAssertThrowsError(try all.supports(transport)) { XCTAssertEqual(($0 as? CompatibilityError)?.message, "Native transport: declared input exceeds budget") }
        XCTAssertThrowsError(try all.detectType(transport))
        XCTAssertThrowsError(try all.registerBuiltin(String(repeating: "x", count: 4097)))
        XCTAssertThrowsError(try all.registered(String(repeating: "x", count: 4097)))
    }
    func testRegistryParseKeepsExactPoliciesOptionsAndOwnedDocumentState() throws {
        let registry = try Pele.Registry.empty(); try registry.registerBuiltin("flowchart")
        let source = "flowchart TD\nA --> B --> C"
        var options = Pele.Engine.ParseOptions(); options.limit = -1; options.maxEdges = -1
        XCTAssertThrowsError(try registry.parseMutable(source, limits: .init(source: .undefined), options: options))
        XCTAssertThrowsError(try registry.parseMutable(source, limits: .init(maxEdges: .undefined), options: options))
        let nan = Double(bitPattern: 0x7ff8000000000042)
        options.config = ["flowchart": .object(["inheritDir": .bool(true)])]
        let document = try registry.parseMutable(source, limits: .init(source: .undefined, maxEdges: .number(nan)), options: options)
        var graph = try document.snapshot().graph
        guard case .ref(let settings) = try graph.field("options"), case .number(let stored) = try graph.field("maxEdges", on: settings) else { return XCTFail("Missing exact edge option") }
        XCTAssertEqual(stored.bitPattern, nan.bitPattern)
        guard case .bool(true) = try graph.field("inheritDir", on: settings) else { return XCTFail("Lost config") }
        try registry.registerBuiltin("flowchart"); try registry.close()
        XCTAssertEqual(try document.render().type, "flowchart")
        let title = try graph.intern("Independent document")
        try graph.setField("title", value: .string(title)); try document.apply(graph, expectedRevision: document.revision)
        XCTAssertTrue(try document.render(outputLimit: .undefined).svg.contains("Independent document")); try document.close()
        let all = try Pele.Registry.allBuiltins(); defer { try? all.close() }
        options = .init()
        XCTAssertThrowsError(try all.parseMutable(source, limits: .init(source: .null, maxEdges: .number(-1)), options: options)) {
            XCTAssertEqual(($0 as? CompatibilityError)?.message, "Diagram source is longer than the limit of null characters.")
        }
        XCTAssertThrowsError(try all.parseMutable(source, limits: .init(maxEdges: .null))) { XCTAssertEqual(($0 as? CompatibilityError)?.code, "limit") }
        let fractional = try all.parseMutable(source, limits: .init(maxEdges: .number(1.5)))
        XCTAssertEqual(try fractional.render().type, "flowchart"); try fractional.close()
    }
}
