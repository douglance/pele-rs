import Foundation
import XCTest
import Pele

private final class PreparedFailure: Error {}
private final class PreparedTrace: @unchecked Sendable {
    private let lock = NSLock(); private var events: [ConfigValue] = []
    func add(_ value: ConfigValue) { lock.lock(); events.append(value); lock.unlock() }
    var value: ConfigValue { lock.lock(); defer { lock.unlock() }; return .array(events) }
}
final class PreparedDocumentTests: XCTestCase {
    private func canonical(_ graph: Pele.PublicGraph, schema: Pele.PublicModelSchema) -> ConfigValue {
        var ids: [UInt32: UInt32] = [:], pending: [UInt32] = [], nodes: [ConfigValue] = [], functions: [UInt32] = []
        func units(_ value: Pele.JSString) -> ConfigValue { .array(value.codeUnits.map { .number(Double($0)) }) }
        func atom(_ value: Pele.PublicGraph.Atom) -> ConfigValue {
            switch value {
            case .undefined: return .array([.string("undefined")])
            case .null: return .array([.string("null")])
            case .bool(let v): return .array([.string("bool"), .bool(v)])
            case .number(let v): return .array([.string("number"), .string(String(format: "%016llx", v.bitPattern))])
            case .string(let id): return .array([.string("string"), units(graph.strings[Int(id)])])
            case .builtinFunction(let id):
                if !functions.contains(id) { functions.append(id) }
                return .array([.string("functionObservation"), .number(Double(functions.firstIndex(of: id)!))])
            case .hostReference(let id): return .array([.string("host"), .number(Double(id))])
            case .ref(let id):
                if ids[id] == nil { ids[id] = UInt32(pending.count); pending.append(id) }
                return .array([.string("ref"), .number(Double(ids[id]!))])
            }
        }
        let root = atom(graph.root)
        var cursor = 0
        while cursor < pending.count {
            let node = graph.nodes[Int(pending[cursor])]
            var value: [String: ConfigValue] = [:]
            switch node {
            case .error(let kind, _): value = ["kind": .string("error"), "errorType": .string(kind.rawValue)]
            case .instance(let prototype, _): value = ["kind": .string("instance"), "prototype": .number(Double(prototype))]
            case .object: value = ["kind": .string("object")]
            case .array(let length, _): value = ["kind": .string("array"), "length": .number(Double(length))]
            case .map(let entries, _): value = ["kind": .string("map"), "entries": .array(entries.map { .array([atom($0.key), atom($0.value)]) })]
            case .set(let values, _): value = ["kind": .string("set"), "values": .array(values.map(atom))]
            case .date(let ms, _): value = ["kind": .string("date"), "milliseconds": .string(String(format: "%016llx", ms.bitPattern))]
            }
            value["properties"] = .array(node.properties.map { .array([units(graph.strings[Int($0.key)]), atom($0.value), .number(Double($0.flags))]) })
            nodes.append(.object(value)); cursor += 1
        }
        var value: [String: ConfigValue] = ["root": root, "nodes": .array(nodes)]
        if !functions.isEmpty {
            value["functionObservations"] = .array(functions.map { id in
                let arity = schema.methods.first { $0.id == id }!.arity
                let constructible = schema.functionShapes?.contains { $0.method == id && $0.constructible } == true
                return .object(["arity": .number(Double(arity)), "ownKeys": .array((constructible ? ["length","name","prototype"] : ["length","name"]).map { .string($0) })])
            })
        }
        return .object(value)
    }

    private struct Fixture: Decodable {
        struct Case: Decodable {
            struct Expected: Decodable {
                struct Drawing: Decodable { let svg: String; let links: [DiagramLink] }
                let before: ConfigValue; let after: ConfigValue; let calls: ConfigValue
                let result: Drawing; let widthBits: String; let heightBits: String
            }
            let id: String; let family: String; let source: String
            let config: [String: ConfigValue]; let title: String?; let lineOffset: UInt64
            let edit: [String: String]?; let options: [String: ConfigValue]
            let expected: Expected
        }
        let cases: [Case]
    }
    func testAllThirtyThreeRawEntriesPreservePinnedModelsSVGAndCallbacks() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let fixture = try JSONDecoder().decode(Fixture.self, from: Data(contentsOf: root.appendingPathComponent("fixtures/pele/diagram-entries.json")))
        XCTAssertEqual(fixture.cases.count, 99); XCTAssertEqual(Set(fixture.cases.map(\.family)).count, 33)
        for item in fixture.cases {
            try autoreleasepool {
                var parse = Pele.PreparedParseOptions(); parse.config = item.config; parse.title = item.title; parse.lineOffset = item.lineOffset
                let document = try Pele.Engine().parsePrepared(item.family, source: item.source, options: parse)
                defer { try? document.close() }
                let schema = try document.schema()
                XCTAssertEqual(canonical(try document.snapshot().graph, schema: schema), item.expected.before, item.id)
                if let edit = item.edit {
                    var snapshot = try document.snapshot()
                    for (key, text) in edit { let value = try snapshot.graph.intern(text); try snapshot.graph.setField(key, value: .string(value)) }
                    try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
                }
                let trace = PreparedTrace(); var host = Pele.HostResources(); host.timeZone = TimeZone(secondsFromGMT: 0)!
                host.textWidth = { request in
                    let value = Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6)
                    trace.add(.array([.string("width"), .array(request.text.utf16.map { .number(Double($0)) }), .string(String(format: "%016llx",request.size.bitPattern)), .number(Double(request.style)), .string(String(format: "%016llx",value.bitPattern))])); return value
                }
                host.icon = { name in trace.add(.array([.string("icon"), .array(name.utf16.map { .number(Double($0)) })])); return .init(trustedMarkup: "<path d=\"M0,0\"/>") }
                host.nowMillis = { trace.add(.array([.string("clock"), .string(String(format: "%016llx",Double(1_704_067_200_000).bitPattern))])); return 1_704_067_200_000 }
                var fields = item.options; fields["config"] = .object([:])
                let settings = try JSONDecoder().decode(Pele.Engine.Options.self, from: JSONEncoder().encode(fields))
                let drawing = try document.render(config: item.config, options: settings, resources: host)
                XCTAssertEqual(drawing.type, item.family, item.id); XCTAssertEqual(drawing.svg, item.expected.result.svg, item.id)
                XCTAssertEqual(String(format: "%016llx",drawing.width.bitPattern), item.expected.widthBits, item.id)
                XCTAssertEqual(String(format: "%016llx",drawing.height.bitPattern), item.expected.heightBits, item.id)
                XCTAssertEqual(drawing.links.map { [$0.id,$0.href,String($0.internal)] }, item.expected.result.links.map { [$0.id,$0.href,String($0.internal)] }, item.id)
                XCTAssertEqual(canonical(try document.snapshot().graph, schema: schema), item.expected.after, item.id)
                XCTAssertEqual(trace.value, item.expected.calls, item.id)
            }
        }
    }
    func testRawEdgeUndefinedAndExplicitContextHaveIndependentPolicies() throws {
        let engine = Pele.Engine(), source = "flowchart LR\n" + String(repeating: "A --> B\n", count: 5001)
        XCTAssertThrowsError(try engine.parseMutable(source))
        let document = try engine.parsePrepared("flowchart", source: source)
        defer { try? document.close() }
        let graph = try document.snapshot().graph
        guard case .ref(let options) = try graph.field("options"), case .undefined = try graph.field("maxEdges", on: options) else { return XCTFail("Raw undefined edge policy was defaulted") }
        var strict = Pele.PreparedParseOptions(); strict.maxEdges = .number(1)
        XCTAssertThrowsError(try engine.parsePrepared("flowchart", source: source, options: strict))
        var context = Pele.PreparedParseOptions(); context.title = "Prepared title"
        let titled = try engine.parsePrepared("pie", source: "pie\n\"One\":1\n", options: context)
        defer { try? titled.close() }
        let titledGraph = try titled.snapshot().graph
        guard case .string(let title) = try titledGraph.field("title") else { return XCTFail("Missing explicit title") }
        XCTAssertEqual(titledGraph.strings[Int(title)].stringIfWellFormed, "Prepared title")
        var overflow = Pele.PreparedParseOptions(); overflow.lineOffset = .max
        XCTAssertThrowsError(try engine.parsePrepared("pie", source: "pie\n\"One\":1\n", options: overflow))
    }
    func testRawBoundMethodUsesSameOwnerAndPreservesOriginalFailureAfterPartialEdit() throws {
        let document = try Pele.Engine().parsePrepared("gantt", source: "gantt\ndateFormat YYYY-MM-DD\nsection S\nA :a,2024-01-01,1d\n")
        defer { try? document.close() }
        let method = try XCTUnwrap(document.schema().methods.first { $0.name == "getTasks" }).id
        var escaped: Pele.ActiveModel?
        let function = Pele.HostFunction { view, receiver, arguments in
            escaped = view
            let tasks = try view.invokeBuiltin(method: method, receiver: receiver, arguments: arguments)
            let first = try view.read("0", on: tasks)
            try view.set("task", on: first, to: view.intern("Prepared callback edit"))
            return tasks
        }
        let value = try document.bindHostFunction(function)
        var graph = try document.snapshot().graph; try graph.setField("getTasks", value: value)
        try document.apply(graph, expectedRevision: document.revision)
        var options = Pele.Engine.Options(); options.now = 1_704_067_200_000
        var host = Pele.HostResources(); host.timeZone = TimeZone(secondsFromGMT: 0)!
        XCTAssertTrue(try document.render(options: options, resources: host).svg.contains("Prepared callback edit"))
        XCTAssertEqual(document.revision, 2)
        XCTAssertThrowsError(try XCTUnwrap(escaped).root())
        let marker = PreparedFailure()
        function.call = { view, receiver, _ in try view.set("work", on: receiver, to: .number(91)); throw marker }
        XCTAssertThrowsError(try document.render(options: options, resources: host)) { XCTAssertTrue(($0 as? PreparedFailure) === marker) }
        XCTAssertEqual(document.revision, 3)
        guard case .number(let work) = try document.snapshot().graph.field("work") else { return XCTFail("Missing partial state") }
        XCTAssertEqual(work, 91)
    }
    func testRawRenderIgnoresCoreOutputLimitAndRetainsHostFailureRevision() throws {
        let document = try Pele.Engine().parsePrepared("flowchart", source: "flowchart LR\nA[One] --> B[Two]\n")
        var options = Pele.Engine.Options(); options.outputLimit = 0
        XCTAssertTrue(try document.render(options: options).svg.contains("One")); XCTAssertEqual(document.revision, 1)
        let failure = PreparedFailure(); var host = Pele.HostResources(); host.textWidth = { _ in throw failure }
        XCTAssertThrowsError(try document.render(resources: host)) { XCTAssertTrue(($0 as? PreparedFailure) === failure) }
        XCTAssertEqual(document.revision, 2)
        options.padding = .nan
        XCTAssertTrue(try document.render(options: options).width.isNaN); XCTAssertEqual(document.revision, 3)
        try document.close(); XCTAssertThrowsError(try document.render()); try document.close()
    }
}
