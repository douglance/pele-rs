import Foundation
import XCTest
import Pele

private final class OutputMeasurements: @unchecked Sendable {
    private let lock = NSLock()
    private var records: [ConfigValue] = []
    func append(_ request: Pele.TextMeasurement) {
        let value: ConfigValue = .array([.array(request.text.utf16.map { .number(Double($0)) }), .string(String(format: "%016llx", request.size.bitPattern)), .number(Double(request.style))])
        lock.lock(); records.append(value); lock.unlock()
    }
    var value: ConfigValue { lock.lock(); defer { lock.unlock() }; return .array(records) }
}
private final class OutputHostFailure: Error {}

final class OutputLimitTests: XCTestCase {
    private struct Fixture: Decodable {
        struct Host: Decodable { let now: Double }
        struct Item: Decodable {
            struct Limit: Decodable {
                let kind: String
                let bits: String?
                func value() throws -> Pele.OutputLimit {
                    switch kind {
                    case "undefined": return .undefined
                    case "null": return .null
                    case "number":
                        let text = try XCTUnwrap(bits)
                        XCTAssertEqual(text.count, 16)
                        return .number(Double(bitPattern: try XCTUnwrap(UInt64(text, radix: 16))))
                    default: throw PeleError(code: "test", message: "Unknown fixture output-limit kind")
                    }
                }
            }
            struct Expected: Decodable {
                struct Schedule: Decodable {
                    struct Task: Decodable {
                        let id: String
                        let byIdSame: Bool
                        let previousEndSame: Bool
                    }
                    let tasks: [Task]
                }
                let schedule: Schedule?
                let type: String?
                let result: FullRenderedDiagram?
                let widthBits: String?
                let heightBits: String?
                let error: CompatibilityError?
                let calls: ConfigValue
                let modelBefore: ConfigValue
                let modelAfter: ConfigValue
                let tasksIdentityRetained: Bool?
            }
            let id: String
            let source: String
            let limit: Limit
            let expected: Expected
        }
        struct Options: Decodable {
            let outputLimit: Int
            let now: Double
            var native: Pele.Engine.Options {
                var value = Pele.Engine.Options()
                value.outputLimit = outputLimit; value.now = now
                return value
            }
        }
        let options: Options
        let host: Host
        let cases: [Item]
        let ordering: [Item]
    }
    private func fixture() throws -> Fixture {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        return try JSONDecoder().decode(Fixture.self, from: Data(contentsOf: root.appendingPathComponent("fixtures/pele/output-limit.json")))
    }
    private func equalError(_ actual: CompatibilityError, _ expected: CompatibilityError, id: String) {
        XCTAssertEqual(actual.name, expected.name, id); XCTAssertEqual(actual.code, expected.code, id)
        XCTAssertEqual(actual.message, expected.message, id); XCTAssertEqual(actual.type, expected.type, id)
        XCTAssertEqual(actual.line, expected.line, id); XCTAssertEqual(actual.column, expected.column, id)
        XCTAssertEqual(actual.snippet, expected.snippet, id)
    }
    private func resources(now: Double, calls: OutputMeasurements) -> Pele.HostResources {
        var host = Pele.HostResources(); host.timeZone = TimeZone(secondsFromGMT: 0)!
        host.nowMillis = { now }
        host.textWidth = { request in
            calls.append(request)
            return Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6)
        }
        return host
    }
    // Pinned oracle's JSON projection: Map/Set become arrays, Dates ISO strings,
    // own undefined/function values disappear. Identity is checked separately.
    private func projected(_ graph: Pele.PublicGraph, _ atom: Pele.PublicGraph.Atom, depth: Int = 0) throws -> ConfigValue? {
        guard depth < 100 else { throw PeleError(code: "test", message: "Cyclic fixture projection") }
        func visit(_ value: Pele.PublicGraph.Atom) throws -> ConfigValue { try projected(graph, value, depth: depth + 1) ?? .null }
        switch atom {
        case .undefined, .builtinFunction, .hostReference: return nil
        case .null: return .null
        case .bool(let value): return .bool(value)
        case .number(let value): return value.isFinite ? .number(value) : .null
        case .string(let id): return .string(try XCTUnwrap(graph.strings[Int(id)].stringIfWellFormed))
        case .ref(let id):
            let node = graph.nodes[Int(id)]
            switch node {
            case .array(let length, _):
                return .array(try (0..<length).map { try visit(graph.field(String($0), on: id) ?? .undefined) })
            case .map(let entries, _): return .array(try entries.map { .array([try visit($0.key), try visit($0.value)]) })
            case .set(let values, _): return .array(try values.map(visit))
            case .date(let milliseconds, _):
                guard milliseconds.isFinite else { return .null }
                let formatter = ISO8601DateFormatter(); formatter.timeZone = TimeZone(secondsFromGMT: 0)!
                formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
                return .string(formatter.string(from: Date(timeIntervalSince1970: milliseconds / 1000)))
            default:
                var fields: [String: ConfigValue] = [:]
                for property in node.properties where property.flags & 1 != 0 {
                    if let value = try projected(graph, property.value, depth: depth + 1) {
                        fields[try XCTUnwrap(graph.strings[Int(property.key)].stringIfWellFormed)] = value
                    }
                }
                return .object(fields)
            }
        }
    }
    private func reference(_ value: Pele.PublicGraph.Atom?) throws -> UInt32 {
        guard case .ref(let id) = value else { throw PeleError(code: "test", message: "Expected public reference") }
        return id
    }
    private func checkTaskAliases(_ before: Pele.PublicGraph, _ after: Pele.PublicGraph, schedule: Fixture.Item.Expected.Schedule?, id: String) throws {
        let initial = try reference(before.field("tasks")), tasks = try reference(after.field("tasks"))
        XCTAssertEqual(initial, tasks, id)
        let byId = try reference(after.field("byId")), expected = try XCTUnwrap(schedule)
        guard case .map(let entries, _) = after.nodes[Int(byId)] else { return XCTFail("Expected task Map: \(id)") }
        var previousEnd: UInt32?
        for (index, task) in expected.tasks.enumerated() {
            let identity = try reference(after.field(String(index), on: tasks))
            let mapEntry = entries.first { entry in
                guard case .string(let key) = entry.key else { return false }
                return after.strings[Int(key)] == Pele.JSString(task.id)
            }
            XCTAssertEqual(try reference(mapEntry?.value) == identity, task.byIdSame, "\(id)/\(task.id)/byIdSame")
            let start: UInt32?
            if case .ref(let value) = try after.field("startTime", on: identity) { start = value } else { start = nil }
            let same = start != nil && previousEnd != nil && start == previousEnd
            XCTAssertEqual(same, task.previousEndSame, "\(id)/\(task.id)/previousEndSame")
            if case .ref(let value) = try after.field("endTime", on: identity) { previousEnd = value } else { previousEnd = nil }
        }
    }
    func testPinnedExactOutputLimitsAndRetainedState() throws {
        let data = try fixture(); XCTAssertEqual(data.cases.count, 192)
        for item in data.cases {
            try autoreleasepool {
                let document = try Pele.Engine().parseMutable(item.source); defer { try? document.close() }
                let before = try document.snapshot().graph
                XCTAssertEqual(try projected(before, before.root), item.expected.modelBefore, item.id)
                let calls = OutputMeasurements()
                let outcome = try document.renderOperation(outputLimit: item.limit.value(), options: data.options.native, resources: resources(now: data.host.now, calls: calls))
                XCTAssertEqual(outcome.revision, 1, item.id); XCTAssertEqual(document.revision, 1, item.id)
                XCTAssertEqual(calls.value, item.expected.calls, item.id)
                XCTAssertEqual(try projected(outcome.graph, outcome.graph.root), item.expected.modelAfter, item.id)
                if item.expected.tasksIdentityRetained == true { try checkTaskAliases(before, outcome.graph, schedule: item.expected.schedule, id: item.id) }
                if let expected = item.expected.error {
                    guard case .thrown(.pele(let actual)) = outcome.result else { return XCTFail("Expected Pele output error: \(item.id)") }
                    equalError(actual, expected, id: item.id)
                    let recovered = try document.render(outputLimit: .number(.infinity), options: data.options.native, resources: resources(now: data.host.now, calls: OutputMeasurements()))
                    XCTAssertEqual(recovered.type, document.type, item.id)
                    XCTAssertEqual(document.revision, 2, item.id)
                } else {
                    guard case .rendered(let actual) = outcome.result else { return XCTFail("Expected output: \(item.id)") }
                    let expected = try XCTUnwrap(item.expected.result)
                    XCTAssertEqual(actual.type, expected.type, item.id); XCTAssertEqual(actual.svg, expected.svg, item.id)
                    XCTAssertEqual(actual.width.bitPattern, try XCTUnwrap(UInt64(try XCTUnwrap(item.expected.widthBits), radix: 16)), item.id)
                    XCTAssertEqual(actual.height.bitPattern, try XCTUnwrap(UInt64(try XCTUnwrap(item.expected.heightBits), radix: 16)), item.id)
                }
            }
        }
    }
    func testHostFailurePrecedesOutputLimitAndPublishesGanttState() throws {
        let data = try fixture(); XCTAssertEqual(data.ordering.count, 2)
        for item in data.ordering {
            let document = try Pele.Engine().parseMutable(item.source); defer { try? document.close() }
            let before = try document.snapshot().graph, calls = OutputMeasurements(), sentinel = OutputHostFailure()
            var host = resources(now: data.host.now, calls: calls)
            host.textWidth = { request in calls.append(request); throw sentinel }
            XCTAssertThrowsError(try document.renderOperation(outputLimit: item.limit.value(), options: data.options.native, resources: host), item.id) {
                XCTAssertTrue(($0 as? OutputHostFailure) === sentinel, item.id)
            }
            XCTAssertEqual(document.revision, 1, item.id)
            let after = try document.snapshot().graph
            XCTAssertEqual(calls.value, item.expected.calls, item.id)
            XCTAssertEqual(try projected(after, after.root), item.expected.modelAfter, item.id)
            if item.expected.tasksIdentityRetained == true { try checkTaskAliases(before, after, schedule: item.expected.schedule, id: item.id) }
            XCTAssertEqual(try document.render(outputLimit: .undefined, options: data.options.native, resources: resources(now: data.host.now, calls: OutputMeasurements())).type, document.type, item.id)
            XCTAssertEqual(document.revision, 2, item.id)
        }
    }
    func testOverrideLegacyCompatibilityOptionsBudgetAndRecovery() throws {
        let document = try Pele.Engine().parseMutable("info title 😀"); defer { try? document.close() }
        var options = Pele.Engine.Options(); options.outputLimit = 0
        XCTAssertThrowsError(try document.render(options: options)) { XCTAssertEqual(($0 as? CompatibilityError)?.code, "limit") }
        XCTAssertEqual(document.revision, 1)
        // Old integer decoder would reject -1. Explicit outputLimit consumes it.
        options.outputLimit = -1; options.nativeTheme = Pele.NativeTheme()
        let result = try document.render(outputLimit: .null, options: options)
        XCTAssertFalse(result.svg.contains("var(")); XCTAssertEqual(document.revision, 2)
        let threshold = Double(result.svg.utf16.count)
        XCTAssertThrowsError(try document.render(outputLimit: .number(threshold - 0.5), options: options)) {
            XCTAssertEqual(($0 as? CompatibilityError)?.code, "limit")
        }
        XCTAssertEqual(document.revision, 3)
        XCTAssertEqual(try document.render(outputLimit: .number(threshold), options: options).svg, result.svg)
        options.fontFamily = String(repeating: "a", count: 1024 * 1024 + 1)
        let revision = document.revision
        XCTAssertThrowsError(try document.render(outputLimit: .number(.nan), options: options)) {
            XCTAssertEqual(($0 as? CompatibilityError)?.message, "Native transport: declared input exceeds budget")
        }
        XCTAssertEqual(document.revision, revision)
        XCTAssertEqual(try document.render(outputLimit: .number(Double(bitPattern: 0x7ff8000000000042))).type, "info")
        try document.close(); XCTAssertThrowsError(try document.render(outputLimit: .undefined))
    }
}
