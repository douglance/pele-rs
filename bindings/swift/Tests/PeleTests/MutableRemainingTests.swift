import Foundation
import XCTest
import AppKit
import Pele

/// Consumer features cross the actual C archive; graph identity and binary values
/// are checked after mutations, native methods, rendering, and owner disposal.
private final class ScheduleHostFailure: Error {}

final class MutableRemainingTests: XCTestCase {
    private func reference(_ atom: Pele.PublicGraph.Atom?) throws -> UInt32 {
        guard case .ref(let id) = atom else { throw PeleError(code: "test", message: "Expected graph object") }; return id
    }
    private func method(_ document: Pele.MutableDocument, _ name: String) throws -> UInt32 {
        try XCTUnwrap(document.schema().methods.first { $0.name == name }?.id)
    }
    private func text(_ graph: Pele.PublicGraph, _ atom: Pele.PublicGraph.Atom?) throws -> String {
        guard case .string(let id) = atom else { throw PeleError(code: "test", message: "Expected graph string") }
        return try XCTUnwrap(graph.strings[Int(id)].stringIfWellFormed)
    }
    func testSixMoreNativeModelsKeepMutationsValuesAndRenderedPixels() throws {
        let sources = [
            ("venn", "venn-beta\nset A[\"Alpha\"]:20\nset B[\"Beta\"]:14\nunion A,B[\"Both\"]:3", 8),
            ("sequence", "sequenceDiagram\nAlice->>Bob:Hello", 17),
            ("gantt", "gantt\ndateFormat YYYY-MM-DD\nsection Work\nTask:a,2024-01-01,2d", 31),
            ("c4", "C4Context\nPerson(user,\"Customer\")\nSystem(app,\"Banking\")\nRel(user,app,\"Uses\")", 19),
            ("requirement", "requirementDiagram\nrequirement A {\n text: first\n}\nelement E {\n type: Thing\n}\nA - contains -> E", 17),
            ("usecase", "usecase-beta\nactor A(\"Client\")\nU(\"Login\")\nA --> U", 0)
        ]
        var resources = Pele.HostResources(); resources.timeZone = TimeZone(secondsFromGMT: 0)!
        resources.nowMillis = { 1_704_067_200_000 }
        var options = Pele.Engine.Options(); options.nativeTheme = Pele.NativeTheme()
        for (family, source, count) in sources {
            let engine = Pele.Engine(), document = try engine.parseMutable(source)
            let schema = try document.schema()
            XCTAssertEqual(schema.methods.count, count, family)
            XCTAssertEqual(schema.id, "pele/\(family)@1")
            var graph = try document.snapshot().graph
            let raw: [UInt16] = [0xd800, 65, 0xdc00], stringID = try graph.intern(Pele.JSString(codeUnits: raw))
            try graph.setField("hostText", value: .string(stringID))
            try graph.setField("hostNumber", value: .number(Double(bitPattern: 0x7ff8000000000042)))
            try graph.setField("hostSelf", value: graph.root)
            try document.apply(graph, expectedRevision: document.revision)
            let rendered = try document.render(options: options, resources: resources)
            XCTAssertEqual(rendered.svg, try engine.render(source, options: options, resources: resources).svg, family)
            XCTAssertNotNil(NSImage(data: Data(rendered.svg.utf8)), family)
            let saved = try document.snapshot().graph
            XCTAssertEqual(try reference(saved.field("hostSelf")), try reference(saved.root))
            guard case .string(let id) = try saved.field("hostText"), case .number(let number) = try saved.field("hostNumber") else { return XCTFail("Exact host values") }
            XCTAssertEqual(saved.strings[Int(id)].codeUnits, raw)
            XCTAssertEqual(number.bitPattern, 0x7ff8000000000042)
            if let title = schema.methods.first(where: { ["setDiagramTitle", "setTitle"].contains($0.name) }) {
                var edited = saved; let value = try edited.intern("Swift API title")
                try document.apply(edited, expectedRevision: document.revision)
                let outcome = try document.invoke(method: title.id, receiver: edited.root, arguments: [.string(value)], resources: resources)
                guard case .returned = outcome.result else { return XCTFail("Title method \(family)") }
                XCTAssertTrue(try document.render(options: options, resources: resources).svg.contains("Swift API title"), family)
            }
            try document.close(); XCTAssertThrowsError(try document.snapshot())
        }
    }
    func testReturnedPeleErrorIsAValueWithLosslessFieldsAndRetainedIdentity() throws {
        let document = try Pele.Engine().parseMutable("agentflow-beta\nA --> B")
        defer { try? document.close() }
        var graph = try document.snapshot().graph
        try graph.setField("lineOffset", value: .number(-1.5))
        let location = UInt32(graph.nodes.count)
        graph.nodes.append(.object(properties: []))
        try graph.setField("first_line", value: .number(4), on: location)
        try graph.setField("first_column", value: .number(-1.25), on: location)
        let raw: [UInt16] = [0xd800, 65, 0xdc00], message = try graph.intern(Pele.JSString(codeUnits: raw))
        try document.apply(graph, expectedRevision: document.revision)
        let outcome = try document.invoke(method: method(document, "shapeError"), receiver: graph.root, arguments: [.string(message), .ref(location)])
        guard case .returned(let atom) = outcome.result else { return XCTFail("Error must be returned") }
        let id = try reference(atom)
        guard case .error(let brand, let properties) = outcome.graph.nodes[Int(id)] else { return XCTFail("Native Error node") }
        XCTAssertEqual(brand, .pele)
        XCTAssertEqual(properties.first?.flags, 6)
        XCTAssertEqual(try text(outcome.graph, outcome.graph.field("code", on: id)), "semantic")
        guard case .number(let line) = try outcome.graph.field("line", on: id), case .number(let column) = try outcome.graph.field("column", on: id), case .string(let returned) = try outcome.graph.field("message", on: id) else { return XCTFail("Error fields") }
        XCTAssertEqual(line, 2.5); XCTAssertEqual(column, -0.25)
        XCTAssertEqual(outcome.graph.strings[Int(returned)].codeUnits, raw)
        graph = outcome.graph; try graph.setField("savedError", value: atom)
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try reference(document.snapshot().graph.field("savedError")), id)
        XCTAssertEqual(try document.render().type, "agentflow")
    }
    func testGanttSchedulingReturnsSharedDateObjectsThroughNativeCalls() throws {
        let document = try Pele.Engine().parseMutable("gantt\ndateFormat YYYY-MM-DD\nA:a,2024-01-01,1d\nB:b,after a,2d")
        defer { try? document.close() }
        var resources = Pele.HostResources(); resources.timeZone = TimeZone(secondsFromGMT: 0)!
        resources.nowMillis = { 1_704_067_200_000 }
        let graph = try document.snapshot().graph
        let first = try document.invoke(method: method(document, "getTasks"), receiver: graph.root, resources: resources)
        guard case .returned(let tasks) = first.result else { return XCTFail("Scheduled task array") }
        let array = try reference(tasks)
        guard case .array(let length, let entries) = first.graph.nodes[Int(array)] else { return XCTFail("Task array") }
        XCTAssertEqual(length, 2)
        let a = try reference(entries.first { first.graph.strings[Int($0.key)].stringIfWellFormed == "0" }?.value)
        let b = try reference(entries.first { first.graph.strings[Int($0.key)].stringIfWellFormed == "1" }?.value)
        let finish = try reference(first.graph.field("endTime", on: a)), start = try reference(first.graph.field("startTime", on: b))
        guard case .date(let end, _) = first.graph.nodes[Int(finish)], case .date(let beginning, _) = first.graph.nodes[Int(start)] else { return XCTFail("Scheduled Date values") }
        XCTAssertEqual(end, beginning)
        XCTAssertEqual(beginning, 1_704_153_600_000)
        let second = try document.invoke(method: method(document, "getTasks"), receiver: graph.root, resources: resources)
        guard case .returned(let again) = second.result else { return XCTFail("Retained tasks") }
        XCTAssertEqual(try reference(again), array)
        XCTAssertEqual(try document.render(resources: resources).type, "gantt")
    }
    func testScheduledDatesPublishBeforeSwiftHostFailureAndRecover() throws {
        let document = try Pele.Engine().parseMutable("gantt\ndateFormat YYYY-MM-DD\nA:a,2024-01-01,1d")
        defer { try? document.close() }
        let failure = ScheduleHostFailure()
        var resources = Pele.HostResources(); resources.timeZone = TimeZone(secondsFromGMT: 0)!
        resources.nowMillis = { 1_704_067_200_000 }
        resources.textWidth = { _ in throw failure }
        let before = document.revision
        XCTAssertThrowsError(try document.render(resources: resources)) { XCTAssertTrue(($0 as? ScheduleHostFailure) === failure) }
        XCTAssertEqual(document.revision, before + 1)
        let graph = try document.snapshot().graph
        XCTAssertTrue(graph.nodes.contains { if case .date(let milliseconds, _) = $0 { return milliseconds.isFinite }; return false })
        resources.textWidth = nil
        XCTAssertEqual(try document.render(resources: resources).type, "gantt")
    }
    func testEveryErrorBrandRemainsDistinctInSwiftOwnedGraphs() throws {
        let document = try Pele.Engine().parseMutable("info")
        defer { try? document.close() }
        var graph = try document.snapshot().graph
        var ids: [UInt32] = []
        for brand in Pele.ErrorObjectKind.allCases {
            let id = UInt32(graph.nodes.count); ids.append(id)
            graph.nodes.append(.error(errorType: brand, properties: []))
            try graph.setField("error-" + brand.rawValue, value: .ref(id))
            let value = try graph.intern(brand.rawValue)
            try graph.setField("message", value: .string(value), on: id)
        }
        try document.apply(graph, expectedRevision: document.revision)
        let saved = try document.snapshot().graph
        for (brand, id) in zip(Pele.ErrorObjectKind.allCases, ids) {
            guard case .error(let returned, _) = saved.nodes[Int(id)] else { return XCTFail("Error brand lost") }
            XCTAssertEqual(returned, brand)
            XCTAssertEqual(try reference(saved.field("error-" + brand.rawValue)), id)
            XCTAssertEqual(try text(saved, saved.field("message", on: id)), brand.rawValue)
        }
        XCTAssertEqual(try document.render().type, "info")
    }

}
