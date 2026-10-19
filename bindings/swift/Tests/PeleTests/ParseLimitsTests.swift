import Foundation
import XCTest
import Pele

private final class EdgeMeasurements: @unchecked Sendable {
    private let lock = NSLock()
    private var calls: [ConfigValue] = []
    func append(_ request: Pele.TextMeasurement) {
        let record: ConfigValue = .array([.array(request.text.utf16.map { .number(Double($0)) }), .string(String(format: "%016llx", request.size.bitPattern)), .number(Double(request.style))])
        lock.lock(); calls.append(record); lock.unlock()
    }
    var value: ConfigValue { lock.lock(); defer { lock.unlock() }; return .array(calls) }
}
final class ParseLimitsTests: XCTestCase {
    private struct Tagged: Decodable {
        let kind: String
        let bits: String?
        func number() throws -> Double { Double(bitPattern: try XCTUnwrap(UInt64(try XCTUnwrap(bits), radix: 16))) }
        func source() throws -> Pele.SourceLimit {
            switch kind {
            case "undefined": return .undefined
            case "null": return .null
            case "number": return .number(try number())
            default: throw PeleError(code: "test", message: "Unknown source threshold")
            }
        }
        func edge() throws -> Pele.EdgeLimit {
            switch kind {
            case "undefined": return .undefined
            case "null": return .null
            case "number": return .number(try number())
            default: throw PeleError(code: "test", message: "Unknown edge threshold")
            }
        }
        func atom() throws -> Pele.PublicGraph.Atom {
            switch kind {
            case "undefined": return .undefined
            case "null": return .null
            case "number": return .number(try number())
            default: throw PeleError(code: "test", message: "Unknown stored threshold")
            }
        }
    }
    private struct Limits: Decodable {
        let source: Tagged?
        let maxEdges: Tagged?
        func native() throws -> Pele.ParseLimits { Pele.ParseLimits(source: try source?.source(), maxEdges: try maxEdges?.edge()) }
    }
    private struct Drawing: Decodable {
        let result: FullRenderedDiagram?
        let widthBits: String?
        let heightBits: String?
        let calls: ConfigValue?
    }
    private struct Fixture: Decodable {
        struct Item: Decodable {
            struct Expected: Decodable {
                let type: String?
                let graph: ConfigValue?
                let result: FullRenderedDiagram?
                let widthBits: String?
                let heightBits: String?
                let calls: ConfigValue?
                let error: CompatibilityError?
                let edgeCount: Int?
                let maxEdges: Tagged?
            }
            let id: String
            let source: String
            let limits: Limits
            let options: Pele.Engine.ParseOptions?
            let parseOnly: Bool?
            let expected: Expected
        }
        struct Method: Decodable {
            struct Step: Decodable {
                struct Operation: Decodable {
                    let method: String?
                    let args: [ConfigValue]?
                    let setMaxEdges: Tagged?
                }
                struct Edge: Decodable { let id: String; let start: String; let end: String; let edgeByIdSame: Bool }
                let op: Operation
                let error: CompatibilityError?
                let returned: ConfigValue?
                let graph: ConfigValue
                let edgesIdentityRetained: Bool
                let edges: [Edge]
            }
            let id: String
            let source: String
            let limits: Limits
            let initial: ConfigValue
            let steps: [Step]
            let expected: Drawing
        }
        struct Host: Decodable { let now: Double }
        let options: Pele.Engine.ParseOptions
        let host: Host
        let cases: [Item]
        let methods: [Method]
    }
    private func fixture() throws -> Fixture {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        return try JSONDecoder().decode(Fixture.self, from: Data(contentsOf: root.appendingPathComponent("fixtures/pele/edge-limit.json")))
    }
    private func equalError(_ actual: CompatibilityError, _ expected: CompatibilityError, id: String) {
        XCTAssertEqual(actual.name, expected.name, id); XCTAssertEqual(actual.code, expected.code, id)
        XCTAssertEqual(actual.message, expected.message, id); XCTAssertEqual(actual.type, expected.type, id)
        XCTAssertEqual(actual.line, expected.line, id); XCTAssertEqual(actual.column, expected.column, id); XCTAssertEqual(actual.snippet, expected.snippet, id)
    }
    private func reference(_ atom: Pele.PublicGraph.Atom?) throws -> UInt32 {
        guard case .ref(let id) = atom else { throw PeleError(code: "test", message: "Expected graph reference") }; return id
    }
    private func canonical(_ graph: Pele.PublicGraph) -> ConfigValue {
        var ids: [UInt32: UInt32] = [:], pending: [UInt32] = [], nodes: [ConfigValue] = []
        func units(_ value: Pele.JSString) -> ConfigValue { .array(value.codeUnits.map { .number(Double($0)) }) }
        func atom(_ value: Pele.PublicGraph.Atom) -> ConfigValue {
            switch value {
            case .undefined: return .array([.string("undefined")])
            case .null: return .array([.string("null")])
            case .bool(let v): return .array([.string("bool"), .bool(v)])
            case .number(let v): return .array([.string("number"), .string(String(format: "%016llx", v.bitPattern))])
            case .string(let id): return .array([.string("string"), units(graph.strings[Int(id)])])
            case .builtinFunction(let id): return .array([.string("builtinFunction"), .number(Double(id))])
            case .hostReference(let id): return .array([.string("hostReference"), .number(Double(id))])
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
        return .object(["root": root, "nodes": .array(nodes)])
    }
    private func resources(now: Double, calls: EdgeMeasurements) -> Pele.HostResources {
        var resources = Pele.HostResources(); resources.timeZone = TimeZone(secondsFromGMT: 0)!
        resources.nowMillis = { now }; resources.textWidth = { request in calls.append(request); return Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6) }
        return resources
    }
    private func equalDrawing(_ actual: FullRenderedDiagram, expected: FullRenderedDiagram?, width: String?, height: String?, id: String) throws {
        let expected = try XCTUnwrap(expected)
        XCTAssertEqual(actual.type, expected.type, id); XCTAssertEqual(actual.svg, expected.svg, id)
        XCTAssertEqual(actual.width.bitPattern, try XCTUnwrap(UInt64(try XCTUnwrap(width), radix: 16)), id)
        XCTAssertEqual(actual.height.bitPattern, try XCTUnwrap(UInt64(try XCTUnwrap(height), radix: 16)), id)
    }
    func testPinnedIndependentParseLimitsAcrossThreeFamilies() throws {
        let fixture = try fixture(); XCTAssertEqual(fixture.cases.count, 313)
        for item in fixture.cases {
            try autoreleasepool {
                let options = item.options ?? fixture.options
                if let expected = item.expected.error {
                    XCTAssertThrowsError(try Pele.Engine().parseMutable(item.source, limits: item.limits.native(), options: options), item.id) {
                        guard let actual = $0 as? CompatibilityError else { return XCTFail("Wrong parse error: \(item.id): \($0)") }
                        equalError(actual, expected, id: item.id)
                    }
                } else {
                    let document = try Pele.Engine().parseMutable(item.source, limits: item.limits.native(), options: options)
                    defer { try? document.close() }
                    XCTAssertEqual(document.type, item.expected.type, item.id)
                    let snapshot = try document.snapshot()
                    XCTAssertEqual(snapshot.revision, 0, item.id)
                    if let expected = item.expected.graph { XCTAssertEqual(canonical(snapshot.graph), expected, item.id) }
                    if let expected = item.expected.maxEdges {
                        let options = try reference(snapshot.graph.field("options"))
                        let stored = try snapshot.graph.field("maxEdges", on: options)
                        switch try expected.atom() {
                        case .undefined: guard case .undefined = stored else { return XCTFail("Lost undefined: \(item.id)") }
                        case .null: guard case .null = stored else { return XCTFail("Lost null: \(item.id)") }
                        case .number(let expected):
                            guard case .number(let actual) = stored else { return XCTFail("Lost number: \(item.id)") }
                            XCTAssertEqual(actual.bitPattern, expected.bitPattern, item.id)
                        default: return XCTFail("Unexpected threshold")
                        }
                    }
                    if item.parseOnly == true {
                        let edges = try reference(snapshot.graph.field("edges"))
                        guard case .array(let count, _) = snapshot.graph.nodes[Int(edges)] else { return XCTFail("Expected edges Array") }
                        XCTAssertEqual(Int(count), item.expected.edgeCount, item.id)
                    } else {
                        let calls = EdgeMeasurements(); var options = Pele.Engine.Options(); options.now = fixture.host.now
                        let actual = try document.render(options: options, resources: resources(now: fixture.host.now, calls: calls))
                        try equalDrawing(actual, expected: item.expected.result, width: item.expected.widthBits, height: item.expected.heightBits, id: item.id)
                        XCTAssertEqual(calls.value, item.expected.calls, item.id)
                    }
                }
            }
        }
    }
    private func imported(_ value: ConfigValue, into graph: inout Pele.PublicGraph) throws -> Pele.PublicGraph.Atom {
        switch value {
        case .null: return .null
        case .bool(let value): return .bool(value)
        case .number(let value): return .number(value)
        case .string(let value): return .string(try graph.intern(value))
        case .array(let values):
            let id = UInt32(graph.nodes.count); graph.nodes.append(.array(length: 0, properties: []))
            for (index, value) in values.enumerated() { let atom = try imported(value, into: &graph); try graph.setArrayElement(UInt32(index), value: atom, on: id) }
            return .ref(id)
        case .object(let values):
            let id = UInt32(graph.nodes.count); graph.nodes.append(.object(properties: []))
            for (key, value) in values { let atom = try imported(value, into: &graph); try graph.setField(key, value: atom, on: id) }
            return .ref(id)
        }
    }
    func testFractionalPartialMethodsPreserveAliasesRevisionsAndRecover() throws {
        let fixture = try fixture(); XCTAssertEqual(fixture.methods.count, 3)
        for item in fixture.methods {
            let document = try Pele.Engine().parseMutable(item.source, limits: item.limits.native(), options: fixture.options)
            defer { try? document.close() }
            var graph = try document.snapshot().graph
            XCTAssertEqual(canonical(graph), item.initial, item.id)
            let edges = try reference(graph.field("edges")), schema = try document.schema()
            for (index, step) in item.steps.enumerated() {
                let id = "\(item.id)/\(index)"
                graph = try document.snapshot().graph
                if let maximum = step.op.setMaxEdges {
                    let options = try reference(graph.field("options"))
                    try graph.setField("maxEdges", value: maximum.atom(), on: options)
                    let previous = document.revision
                    try document.apply(graph, expectedRevision: previous)
                    XCTAssertEqual(document.revision, previous + 1, id)
                } else {
                    let method = try XCTUnwrap(schema.methods.first { $0.name == step.op.method })
                    let arguments = try (step.op.args ?? []).map { try imported($0, into: &graph) }
                    try document.apply(graph, expectedRevision: document.revision)
                    let previous = document.revision
                    let outcome = try document.invoke(method: method.id, receiver: graph.root, arguments: arguments)
                    XCTAssertEqual(outcome.revision, previous + 1, id)
                    if let expected = step.error {
                        guard case .thrown(.pele(let actual)) = outcome.result else { return XCTFail("Expected partial error: \(id)") }
                        equalError(actual, expected, id: id)
                    } else {
                        guard case .returned(.undefined) = outcome.result else { return XCTFail("Expected undefined return: \(id)") }
                        XCTAssertEqual(step.returned, .array([.string("undefined")]), id)
                    }
                }
                graph = try document.snapshot().graph
                XCTAssertEqual(canonical(graph), step.graph, id)
                XCTAssertEqual(try reference(graph.field("edges")) == edges, step.edgesIdentityRetained, id)
                let cache = try reference(graph.field("edgeById"))
                guard case .map(let entries, _) = graph.nodes[Int(cache)] else { return XCTFail("Expected edge cache") }
                for (index, expected) in step.edges.enumerated() {
                    let edge = try reference(graph.field(String(index), on: edges))
                    let entry = entries.first { entry in guard case .string(let key) = entry.key else { return false }; return graph.strings[Int(key)] == Pele.JSString(expected.id) }
                    XCTAssertEqual(try reference(entry?.value) == edge, expected.edgeByIdSame, id)
                    for (key, value) in [("id", expected.id), ("start", expected.start), ("end", expected.end)] {
                        guard case .string(let string) = try graph.field(key, on: edge) else { return XCTFail("Expected edge text") }
                        XCTAssertEqual(graph.strings[Int(string)], Pele.JSString(value), id)
                    }
                }
            }
            let calls = EdgeMeasurements(); var options = Pele.Engine.Options(); options.now = fixture.host.now
            let rendered = try document.render(options: options, resources: resources(now: fixture.host.now, calls: calls))
            try equalDrawing(rendered, expected: item.expected.result, width: item.expected.widthBits, height: item.expected.heightBits, id: item.id)
            XCTAssertEqual(calls.value, item.expected.calls, item.id)
        }
    }
    func testIndependentOverridesLegacyErrorsAndNativeBudgets() throws {
        let source = "flowchart TD\nA --> B --> C", engine = Pele.Engine()
        var options = Pele.Engine.ParseOptions(); options.limit = -1; options.maxEdges = -1
        XCTAssertThrowsError(try engine.parseMutable(source, options: options))
        for limits in [Pele.ParseLimits(source: .undefined), Pele.ParseLimits(maxEdges: .undefined)] {
            XCTAssertThrowsError(try engine.parseMutable(source, limits: limits, options: options)) { XCTAssertTrue(($0 as? CompatibilityError)?.message.hasPrefix("Invalid parse options:") == true) }
        }
        let document = try engine.parseMutable(source, limits: .init(source: .undefined, maxEdges: .number(Double(bitPattern: 0x7ff8000000000042))), options: options)
        XCTAssertEqual(document.type, "flowchart"); try document.close()
        options = .init(); options.limit = 0; options.maxEdges = 0
        XCTAssertThrowsError(try engine.parseMutable(source, limits: .init(source: .null, maxEdges: .number(-1)), options: options)) {
            XCTAssertEqual(($0 as? CompatibilityError)?.message, "Diagram source is longer than the limit of null characters.")
        }
        options.config = ["large": .string(String(repeating: "a", count: 1024 * 1024 + 1))]
        XCTAssertThrowsError(try engine.parseMutable(source, limits: .init(source: .number(.infinity), maxEdges: .number(.nan)), options: options)) {
            XCTAssertEqual(($0 as? CompatibilityError)?.message, "Native transport: declared input exceeds budget")
        }
        let tooLong = String(repeating: "x", count: 16 * 1024 * 1024 + 1)
        XCTAssertThrowsError(try engine.parseMutable(tooLong, limits: .init(source: .number(.nan), maxEdges: .number(.infinity)))) {
            XCTAssertEqual(($0 as? CompatibilityError)?.message, "Native transport: declared input exceeds budget")
        }
        let recovered = try engine.parseMutable(source, limits: .init(source: .undefined, maxEdges: .undefined))
        XCTAssertEqual(try recovered.render().type, "flowchart"); try recovered.close()
    }
}
