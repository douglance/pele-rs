import Foundation
import XCTest
import Pele
import AppKit
import CryptoKit

private final class MutableHostFailure: Error {}
final class MutableDocumentTests: XCTestCase {
    private func reference(_ value: Pele.PublicGraph.Atom?) throws -> UInt32 {
        guard case .ref(let id) = value else { throw PeleError(code: "test", message: "Expected object reference") }; return id
    }
    private func text(_ graph: Pele.PublicGraph, _ atom: Pele.PublicGraph.Atom?) throws -> String {
        guard case .string(let id) = atom, let text = graph.strings[Int(id)].stringIfWellFormed else {
            throw PeleError(code: "test", message: "Expected well-formed string")
        }; return text
    }
    private func setText(_ graph: inout Pele.PublicGraph, _ key: String, _ value: String, on object: UInt32? = nil) throws {
        let id = try graph.intern(value); try graph.setField(key, value: .string(id), on: object)
    }
    func testPieMutableMapOrderRevisionsAndIndependentLifetime() throws {
        let document = try Pele.Engine().parseMutable("pie title Before\n\"A\":1\n\"B\":2")
        var snapshot = try document.snapshot()
        XCTAssertEqual(snapshot.revision, 0)
        let mapID = try reference(snapshot.graph.field("sections"))
        guard case .map(let entries, let properties) = snapshot.graph.nodes[Int(mapID)] else { return XCTFail("sections Map") }
        XCTAssertEqual(entries.count, 2)
        snapshot.graph.nodes[Int(mapID)] = .map(entries: [
            .init(key: entries[1].key, value: .number(4)),
            .init(key: entries[0].key, value: .number(1))
        ], properties: properties)
        try setText(&snapshot.graph, "title", "Edited")
        try setText(&snapshot.graph, "type", "application-type")
        XCTAssertEqual(try document.apply(snapshot.graph, expectedRevision: snapshot.revision), 1)
        let first = try document.render()
        XCTAssertEqual(first.type, "pie")
        XCTAssertEqual(first.svg, try Pele.Engine().render("pie title Edited\n\"B\":4\n\"A\":1").svg)
        let saved = try document.snapshot()
        XCTAssertEqual(try reference(saved.graph.field("sections")), mapID)
        XCTAssertEqual(try text(saved.graph, saved.graph.field("type")), "application-type")
        XCTAssertThrowsError(try document.apply(snapshot.graph, expectedRevision: 0)) {
            XCTAssertEqual(($0 as? CompatibilityError)?.message, "Stale public model revision")
        }
        XCTAssertEqual(document.revision, saved.revision)
        XCTAssertEqual(saved.revision, 2)
        XCTAssertEqual(try document.render().svg, first.svg)
        var limited = Pele.Engine.Options(); limited.outputLimit = 1
        XCTAssertThrowsError(try document.render(options: limited)) { XCTAssertEqual(($0 as? CompatibilityError)?.code, "limit") }
        XCTAssertEqual(try document.render().svg, first.svg)
        try document.close(); try document.close()
        XCTAssertThrowsError(try document.snapshot())
        XCTAssertThrowsError(try document.render())
    }
    func testSankeyAliasesAndLosslessFlatGraphSurviveNativeEditBoundary() throws {
        let document = try Pele.Engine().parseMutable("sankey-beta\nA,B,2\nA,C,3")
        var graph = try document.snapshot().graph
        let nodes = try reference(graph.field("nodes")), links = try reference(graph.field("links")), lookup = try reference(graph.field("lookup"))
        let a = try reference(graph.field("0", on: nodes)), link0 = try reference(graph.field("0", on: links)), link1 = try reference(graph.field("1", on: links))
        XCTAssertEqual(try reference(graph.field("source", on: link0)), a)
        XCTAssertEqual(try reference(graph.field("source", on: link1)), a)
        guard case .map(let entries, _) = graph.nodes[Int(lookup)] else { return XCTFail("lookup Map") }
        XCTAssertEqual(try reference(entries[0].value), a)
        let sharedSource = try reference(graph.field("source", on: link0))
        try setText(&graph, "id", "Renamed", on: sharedSource)
        let raw = try graph.intern(Pele.JSString(codeUnits: [0xD800, 0x41, 0xDC00]))
        try graph.setField("applicationUTF16", value: .string(raw))
        try graph.setField("applicationNaN", value: .number(Double(bitPattern: 0x7FF8000000000042)))
        try graph.setField("applicationZero", value: .number(-0.0))
        try graph.setField("applicationInfinity", value: .number(.infinity))
        let date = UInt32(graph.nodes.count)
        graph.nodes.append(.date(milliseconds: .nan, properties: []))
        try graph.setField("applicationDate", value: .ref(date))
        var tail = graph.root
        let next = try graph.intern("next")
        for _ in 0..<2000 {
            let id = UInt32(graph.nodes.count)
            graph.nodes.append(.object(properties: [.init(key: next, value: tail)])); tail = .ref(id)
        }
        try graph.setField("applicationCycle", value: tail)
        let roundTrip = try JSONDecoder().decode(Pele.PublicGraph.self, from: JSONEncoder().encode(graph))
        try document.apply(roundTrip, expectedRevision: 0)
        let kept = try document.snapshot().graph
        XCTAssertEqual(try reference(kept.field("source", on: link0)), a)
        XCTAssertEqual(try text(kept, kept.field("id", on: a)), "Renamed")
        XCTAssertEqual(kept.strings[Int(raw)].codeUnits, [0xD800, 0x41, 0xDC00])
        XCTAssertNil(kept.strings[Int(raw)].stringIfWellFormed)
        if case .number(let n) = try kept.field("applicationNaN") { XCTAssertEqual(n.bitPattern, 0x7FF8000000000042) } else { XCTFail("NaN") }
        if case .number(let n) = try kept.field("applicationZero") { XCTAssertEqual(n.bitPattern, (-0.0).bitPattern) } else { XCTFail("negative zero") }
        if case .number(let n) = try kept.field("applicationInfinity") { XCTAssertEqual(n, .infinity) } else { XCTFail("infinity") }
        if case .date(let ms, _) = kept.nodes[Int(date)] { XCTAssertTrue(ms.isNaN) } else { XCTFail("Date") }
        XCTAssertEqual(try document.render().svg, try Pele.Engine().render("sankey-beta\nRenamed,B,2\nRenamed,C,3").svg)
    }
    func testInfoMetadataNativeDecodeHostExceptionIdentityAndRecovery() throws {
        let document = try Pele.Engine().parseMutable("info")
        var graph = try document.snapshot().graph
        try setText(&graph, "title", "Mutable native Info")
        try setText(&graph, "accTitle", "Accessible Info")
        try setText(&graph, "accDescr", "Owned mutable graph")
        try document.apply(graph, expectedRevision: 0)
        let failure = MutableHostFailure()
        var resources = Pele.HostResources(); resources.textWidth = { _ in throw failure }
        XCTAssertThrowsError(try document.render(resources: resources)) { XCTAssertTrue(($0 as? MutableHostFailure) === failure) }
        let expected = try document.render()
        XCTAssertEqual(expected.svg, try Pele.Engine().render("info title Mutable native Info\naccTitle: Accessible Info\naccDescr: Owned mutable graph").svg)
        var options = Pele.Engine.Options(); options.nativeTheme = Pele.NativeTheme()
        let native = try document.render(options: options)
        XCTAssertFalse(native.svg.contains("var("))
        let xml = try XMLDocument(data: Data(native.svg.utf8))
        XCTAssertEqual(try xml.nodes(forXPath: "//*[local-name()='title']").first?.stringValue, "Accessible Info")
        XCTAssertEqual(try xml.nodes(forXPath: "//*[local-name()='desc']").first?.stringValue, "Owned mutable graph")
        XCTAssertNotNil(NSImage(data: Data(native.svg.utf8)), "Actual macOS SVG image decoder")
        var edited = try document.snapshot().graph
        let raw = try edited.intern(Pele.JSString(codeUnits: [0xD800]))
        try edited.setField("title", value: .string(raw)); try document.apply(edited, expectedRevision: document.revision)
        XCTAssertThrowsError(try document.render()) { XCTAssertEqual(($0 as? CompatibilityError)?.message, "UTF-16 Info text rendering is not implemented") }
        edited = try document.snapshot().graph; try setText(&edited, "title", "Recovered")
        try document.apply(edited, expectedRevision: document.revision)
        XCTAssertTrue(try document.render().svg.contains("Recovered"))
    }
    func testStatefulRenderOutcomesPublishRevisionsAcrossFailureAndRecovery() throws {
        let document = try Pele.Engine().parseMutable("info title Retained state")
        defer { try? document.close() }
        let first = try document.renderOperation()
        XCTAssertEqual(first.revision, 1)
        guard case .rendered(let initial) = first.result else { return XCTFail("Initial render") }
        XCTAssertEqual(initial.type, "info")
        XCTAssertEqual(try text(first.graph, first.graph.field("title")), "Retained state")
        var limited = Pele.Engine.Options(); limited.outputLimit = 1
        let failed = try document.renderOperation(options: limited)
        XCTAssertEqual(failed.revision, 2)
        guard case .thrown(.pele(let error)) = failed.result else { return XCTFail("Output limit outcome") }
        XCTAssertEqual(error.code, "limit")
        let sentinel = MutableHostFailure()
        var resources = Pele.HostResources(); resources.textWidth = { _ in throw sentinel }
        XCTAssertThrowsError(try document.renderOperation(resources: resources)) {
            XCTAssertTrue(($0 as? MutableHostFailure) === sentinel)
        }
        XCTAssertEqual(document.revision, 3)
        XCTAssertEqual(try document.snapshot().revision, 3)
        var graph = try document.snapshot().graph
        try setText(&graph, "title", "Recovered state")
        try document.apply(graph, expectedRevision: document.revision)
        let recovered = try document.renderOperation()
        XCTAssertEqual(recovered.revision, 5)
        guard case .rendered(let result) = recovered.result else { return XCTFail("Recovered render") }
        XCTAssertTrue(result.svg.contains("Recovered state"))
    }
    func testLargeClassModelsUseBinaryExchangeWithoutRaisingBudgets() throws {
        var root = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 { root.deleteLastPathComponent() }
        let fixture = try JSONSerialization.jsonObject(with: Data(contentsOf: root.appendingPathComponent("fixtures/pele/class.json"))) as! [String: Any]
        let cases = fixture["cases"] as! [[String: Any]]
        let ids = ["stress/1,600 classDefs applied to 1,600 classes", "stress/a classDef of 5,000 declarations on 800 classes, each with a style of its own"]
        for id in ids {
            let item = try XCTUnwrap(cases.first { $0["id"] as? String == id })
            let source = try XCTUnwrap(item["source"] as? String)
            let supplied = item["options"] as! [String: Any]
            var parseFields = try JSONSerialization.jsonObject(with: JSONEncoder().encode(Pele.Engine.ParseOptions())) as! [String: Any]
            var renderFields = try JSONSerialization.jsonObject(with: JSONEncoder().encode(Pele.Engine.Options())) as! [String: Any]
            for (key, value) in supplied { parseFields[key] = value; renderFields[key] = value }
            let parse = try JSONDecoder().decode(Pele.Engine.ParseOptions.self, from: JSONSerialization.data(withJSONObject: parseFields))
            let options = try JSONDecoder().decode(Pele.Engine.Options.self, from: JSONSerialization.data(withJSONObject: renderFields))
            let document = try Pele.Engine().parseMutable(source, options: parse)
            defer { try? document.close() }
            let snapshot = try document.snapshot()
            XCTAssertGreaterThan(snapshot.graph.nodes.count, 1000)
            try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
            let rendered = try document.render(options: options)
            let expected = (item["expected"] as! [String: Any])["result"] as! [String: Any]
            let digest = SHA256.hash(data: Data(rendered.svg.utf8)).map { String(format: "%02x", $0) }.joined()
            XCTAssertEqual(digest, expected["svgSha256"] as? String, id)
        }
    }

    func testBinaryRenderMetadataKeepsNonFiniteDimensions() throws {
        let document = try Pele.Engine().parseMutable("sankey\na,b,5e-324")
        defer { try? document.close() }
        let outcome = try document.renderOperation()
        guard case .rendered(let rendered) = outcome.result else { return XCTFail("Non-finite drawing is an outcome") }
        XCTAssertTrue(rendered.height.isNaN)
        XCTAssertEqual(rendered.type, "sankey")
        XCTAssertEqual(document.revision, outcome.revision)
        XCTAssertTrue(try document.render().height.isNaN)
    }

    func testRepeatedCreateCloseKeepsIndependentOwners() throws {
        for _ in 0..<100 {
            let document = try Pele.Engine().parseMutable("pie\n\"Owned\":1")
            XCTAssertEqual(try document.render().type, "pie")
            try document.close()
        }
    }
    func testNativeJourneyMethodsPublishReturnedAliasesAndPartialErrors() throws {
        let document = try Pele.Engine().parseMutable("journey\nsection Start\nTask:3:Alice")
        let schema = try document.schema()
        XCTAssertEqual(schema.id, "pele/journey@1")
        XCTAssertEqual(schema.methods.count, 6)
        func method(_ name: String) throws -> UInt32 {
            try XCTUnwrap(schema.methods.first { $0.name == name }?.id)
        }
        var graph = try document.snapshot().graph
        let root = graph.root, title = try graph.intern("Swift native methods")
        try document.apply(graph, expectedRevision: document.revision)
        let titled = try document.invoke(method: method("setDiagramTitle"), receiver: root, arguments: [.string(title)])
        guard case .returned(.undefined) = titled.result else { return XCTFail("Setter result") }
        XCTAssertEqual(try text(titled.graph, titled.graph.field("title")), "Swift native methods")
        let actors = try document.invoke(method: method("getActors"), receiver: root)
        guard case .returned(.ref(let array)) = actors.result,
              case .array(let count, let people) = actors.graph.nodes[Int(array)] else { return XCTFail("Native actor array") }
        XCTAssertEqual(count, 1)
        XCTAssertEqual(try text(actors.graph, people.first?.value), "Alice")
        graph = actors.graph
        let section = try graph.intern("Published before failure")
        try graph.setField("sections", value: .null)
        try document.apply(graph, expectedRevision: document.revision)
        let before = document.revision
        let failed = try document.invoke(method: method("addSection"), receiver: root, arguments: [.string(section)])
        guard case .thrown(.javaScript(let name, let message)) = failed.result else { return XCTFail("Expected JavaScript operation error") }
        XCTAssertEqual(name, "TypeError")
        XCTAssertTrue(message.stringIfWellFormed?.contains("push") == true)
        XCTAssertEqual(failed.revision, before + 1)
        XCTAssertEqual(try text(failed.graph, failed.graph.field("currentSection")), "Published before failure")
        graph = failed.graph
        graph.nodes.append(.array(length: 0, properties: []))
        try graph.setField("sections", value: .ref(UInt32(graph.nodes.count - 1)))
        try document.apply(graph, expectedRevision: document.revision)
        var options = Pele.Engine.Options(); options.nativeTheme = Pele.NativeTheme()
        let rendered = try document.render(options: options)
        XCTAssertTrue(rendered.svg.contains("Swift native methods"))
        XCTAssertNotNil(NSImage(data: Data(rendered.svg.utf8)))
        try document.close()
    }
    func testSixteenMutableFamiliesKeepNativeModelsAndRenderMetadataEdits() throws {
        let sources = ["pie\n\"A\":1", "info", "sankey\nA,B,5", "packet-beta\n0-7: \"Data\"", "radar-beta\naxis a[\"One\"], b[\"Two\"], c[\"Three\"]\ncurve team[\"Team\"]{4,3,5}", "quadrantChart\nA:[0.2,0.3]", "treemap-beta\n\"Group\"\n  \"Leaf\":10", "journey\nTask:3:Alice", "xychart-beta\nx-axis [A,B]\nline [1,2]", "kanban\n  Todo\n    task[Build]", "mindmap\n  root((Rust))\n    Swift", "timeline\nsection Start\n2025: Rust", "flowchart LR\nA-->B", "swimlane-beta LR\nsubgraph group[Group]\nA-->B\nend", "erDiagram\nUSER ||--o{ DOCUMENT : owns", "architecture-beta\nservice app(server)[App]"]
        for source in sources {
            let document = try Pele.Engine().parseMutable(source)
            var snapshot = try document.snapshot()
            try setText(&snapshot.graph, "title", "Native mutable title")
            try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
            var options = Pele.Engine.Options(); options.nativeTheme = Pele.NativeTheme()
            let rendered = try document.render(options: options)
            XCTAssertEqual(rendered.type, document.type)
            XCTAssertTrue(rendered.svg.contains("Native mutable title"), document.type)
            XCTAssertNotNil(NSImage(data: Data(rendered.svg.utf8)), document.type)
            try document.close()
        }
    }

    func testSevenMoreMutableFamiliesDecodeAndRenderThroughC() throws {
        let sources = ["gitGraph\ncommit", "agentflow-beta LR\nA[Agent] --> B[Tool]", "classDiagram\nclass A {\n+run() void\n}", "eventmodeling\ntf 01 ui Screen\ntf 02 cmd Submit\ntf 03 evt Saved", "ishikawa-beta\nRoot\n  Cause", "stateDiagram-v2\nA --> B", "treeView-beta\nsrc/\n  main.rs"]
        for source in sources {
            let document = try Pele.Engine().parseMutable(source)
            defer { try? document.close() }
            var snapshot = try document.snapshot()
            try setText(&snapshot.graph, "applicationTitle", "Host extension")
            try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
            let schema = try document.schema()
            XCTAssertNotEqual(schema.id, "pele/opaque@1", document.type)
            var options = Pele.Engine.Options(); options.nativeTheme = Pele.NativeTheme()
            let outcome = try document.renderOperation(options: options)
            guard case .rendered(let result) = outcome.result else { return XCTFail(document.type) }
            XCTAssertEqual(result.type, document.type)
            XCTAssertNotNil(NSImage(data: Data(result.svg.utf8)), document.type)
            XCTAssertEqual(try text(outcome.graph, outcome.graph.field("applicationTitle")), "Host extension")
            if document.type == "class" {
                var graph = outcome.graph
                let className = try graph.intern("HostClass")
                try document.apply(graph, expectedRevision: document.revision)
                let id = try XCTUnwrap(schema.methods.first { $0.name == "addClass" }?.id)
                let added = try document.invoke(method: id, receiver: graph.root, arguments: [.string(className)])
                guard case .returned = added.result else { return XCTFail("Native class builder") }
                XCTAssertTrue(try document.render().svg.contains("HostClass"))
            }
        }
    }

    func testTimelineReceiverAliasesAndPartialFailuresThroughC() throws {
        let document = try Pele.Engine().parseMutable("timeline\nsection Start\n2025: Rust")
        defer { try? document.close() }
        let schema = try document.schema()
        func method(_ name: String) throws -> UInt32 { try XCTUnwrap(schema.methods.first { $0.name == name }?.id) }
        var graph = try document.snapshot().graph
        let root = graph.root
        let returned = try document.invoke(method: method("getCommonDb"), receiver: root)
        guard case .returned(let receiver) = returned.result else { return XCTFail("Returned receiver") }
        XCTAssertEqual(try reference(receiver), try reference(root))
        graph = returned.graph
        let title = try graph.intern("Swift timeline"), next = try graph.intern("Before failure")
        try graph.setField("sections", value: .null)
        try document.apply(graph, expectedRevision: document.revision)
        let failed = try document.invoke(method: method("addSection"), receiver: root, arguments: [.string(next)])
        guard case .thrown(.javaScript(let name, _)) = failed.result else { return XCTFail("Partial operation error") }
        XCTAssertEqual(name, "TypeError")
        XCTAssertEqual(try text(failed.graph, failed.graph.field("currentSection")), "Before failure")
        graph = failed.graph
        graph.nodes.append(.array(length: 0, properties: []))
        try graph.setField("sections", value: .ref(UInt32(graph.nodes.count - 1)))
        try document.apply(graph, expectedRevision: document.revision)
        let titled = try document.invoke(method: method("setDiagramTitle"), receiver: root, arguments: [.string(title)])
        guard case .returned(.undefined) = titled.result else { return XCTFail("Setter result") }
        XCTAssertTrue(try document.render().svg.contains("Swift timeline"))
    }
    func testSharedOutlineDescriptorAndSwimlaneRetainedRenderer() throws {
        let kanban = try Pele.Engine().parseMutable("kanban\n  Todo"), mindmap = try Pele.Engine().parseMutable("mindmap\n  root((Rust))")
        defer { try? kanban.close(); try? mindmap.close() }
        let first = try kanban.schema(), second = try mindmap.schema()
        XCTAssertEqual(first.sharedFunctions?.first?.identity, "pele/common/outline/getType@1")
        XCTAssertEqual(first.sharedFunctions?.first?.identity, second.sharedFunctions?.first?.identity)
        XCTAssertEqual(first.functionShapes?.first?.constructible, true)
        XCTAssertEqual(second.functionShapes?.first?.constructible, true)
        for document in [kanban, mindmap] {
            var graph = try document.snapshot().graph
            let start = try graph.intern("["), end = try graph.intern("]")
            try document.apply(graph, expectedRevision: document.revision)
            let outcome = try document.invoke(method: try XCTUnwrap(document.schema().methods.first { $0.name == "getType" }?.id), receiver: .null, arguments: [.string(start), .string(end)])
            guard case .returned(.number(let value)) = outcome.result else { return XCTFail("Shared helper result") }
            XCTAssertEqual(value, 2)
        }
        let flowchart = try Pele.Engine().parseMutable("flowchart LR\nA-->B"), swimlane = try Pele.Engine().parseMutable("swimlane-beta LR\nsubgraph lane[Lane]\nA-->B\nend")
        defer { try? flowchart.close(); try? swimlane.close() }
        XCTAssertEqual(try flowchart.schema().id, try swimlane.schema().id)
        XCTAssertEqual(try text(swimlane.snapshot().graph, swimlane.snapshot().graph.field("type")), "flowchart")
        XCTAssertEqual(try swimlane.render().type, "swimlane")
    }

}
