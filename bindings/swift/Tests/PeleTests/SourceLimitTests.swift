import Foundation
import XCTest
import Pele

final class SourceLimitTests: XCTestCase {
    private struct Fixture: Decodable {
        struct Host: Decodable { let now: Double }
        struct Case: Decodable {
            struct Limit: Decodable {
                let kind: String
                let bits: String?
                func value() throws -> Pele.SourceLimit {
                    switch kind {
                    case "undefined": return .undefined
                    case "null": return .null
                    case "number":
                        let text = try XCTUnwrap(bits)
                        XCTAssertEqual(text.count, 16)
                        return .number(Double(bitPattern: try XCTUnwrap(UInt64(text, radix: 16))))
                    default: throw PeleError(code: "test", message: "Unknown fixture source-limit kind")
                    }
                }
            }
            struct Expected: Decodable {
                let type: String?
                let result: FullRenderedDiagram?
                let widthBits: String?
                let heightBits: String?
                let error: CompatibilityError?
            }
            let id: String
            let source: String
            let limit: Limit
            let expected: Expected
        }
        let options: Pele.Engine.ParseOptions
        let host: Host
        let cases: [Case]
    }
    private func equalError(_ actual: CompatibilityError, _ expected: CompatibilityError, id: String) {
        XCTAssertEqual(actual.name, expected.name, id)
        XCTAssertEqual(actual.code, expected.code, id)
        XCTAssertEqual(actual.message, expected.message, id)
        XCTAssertEqual(actual.type, expected.type, id)
        XCTAssertEqual(actual.line, expected.line, id)
        XCTAssertEqual(actual.column, expected.column, id)
        XCTAssertEqual(actual.snippet, expected.snippet, id)
    }
    func testPinnedSourceLimitsThroughOwnedNativeDocuments() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let fixture = try JSONDecoder().decode(Fixture.self, from: Data(contentsOf: root.appendingPathComponent("fixtures/pele/source-limit.json")))
        XCTAssertFalse(fixture.cases.isEmpty)
        let engine = Pele.Engine()
        let now = fixture.host.now
        var resources = Pele.HostResources()
        resources.timeZone = TimeZone(secondsFromGMT: 0)!
        resources.nowMillis = { now }
        resources.textWidth = { request in Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6) }
        for item in fixture.cases {
            try autoreleasepool {
                let limit = try item.limit.value()
                if let expected = item.expected.error {
                    XCTAssertThrowsError(try engine.parseMutable(item.source, sourceLimit: limit, options: fixture.options), item.id) { error in
                        guard let actual = error as? CompatibilityError else { return XCTFail("Wrong error: \(item.id): \(error)") }
                        equalError(actual, expected, id: item.id)
                    }
                } else {
                    let document = try engine.parseMutable(item.source, sourceLimit: limit, options: fixture.options)
                    defer { try? document.close() }
                    XCTAssertEqual(document.type, try XCTUnwrap(item.expected.type), item.id)
                    var snapshot = try document.snapshot()
                    let raw = try snapshot.graph.intern(Pele.JSString(codeUnits: [0xd800, 0, 0xdc00]))
                    try snapshot.graph.setField("sourceLimitText", value: .string(raw))
                    try snapshot.graph.setField("sourceLimitSelf", value: snapshot.graph.root)
                    try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
                    var options = Pele.Engine.Options(); options.now = now
                    let rendered = try document.render(options: options, resources: resources)
                    let expected = try XCTUnwrap(item.expected.result)
                    XCTAssertEqual(rendered.type, expected.type, item.id)
                    XCTAssertEqual(rendered.svg, expected.svg, item.id)
                    XCTAssertEqual(rendered.width.bitPattern, try XCTUnwrap(UInt64(try XCTUnwrap(item.expected.widthBits), radix: 16)), item.id)
                    XCTAssertEqual(rendered.height.bitPattern, try XCTUnwrap(UInt64(try XCTUnwrap(item.expected.heightBits), radix: 16)), item.id)
                    let saved = try document.snapshot().graph
                    guard case .string(let key) = try saved.field("sourceLimitText") else { return XCTFail("Lost UTF16 extra") }
                    XCTAssertEqual(saved.strings[Int(key)].codeUnits, [0xd800, 0, 0xdc00], item.id)
                    guard case .ref(let identity) = try saved.field("sourceLimitSelf"), case .ref(let root) = saved.root else { return XCTFail("Lost cyclic extra") }
                    XCTAssertEqual(identity, root, item.id)
                    try document.close()
                    XCTAssertThrowsError(try document.snapshot(), item.id)
                }
            }
        }
    }
    func testOverrideRetainsOtherOptionsAndLegacyThreshold() throws {
        let engine = Pele.Engine()
        var options = Pele.Engine.ParseOptions(); options.limit = 1
        XCTAssertThrowsError(try engine.parseMutable("info", options: options)) { error in
            XCTAssertEqual((error as? CompatibilityError)?.message, "Diagram source is longer than the limit of 1 characters.")
        }
        options.limit = -1 // Ignored by the explicit-source-limit overload, not integer-decoded.
        let document = try engine.parseMutable("info", sourceLimit: .undefined, options: options)
        try document.close()

        options.limit = 50_000; options.maxEdges = 1
        let source = "flowchart TD\nA --> B --> C"
        var original: CompatibilityError?
        XCTAssertThrowsError(try engine.parseMutable(source, options: options)) { original = $0 as? CompatibilityError }
        let expected = try XCTUnwrap(original)
        XCTAssertThrowsError(try engine.parseMutable(source, sourceLimit: .number(.infinity), options: options)) { error in
            guard let actual = error as? CompatibilityError else { return XCTFail("Wrong edge error") }
            equalError(actual, expected, id: "maxEdges retained")
        }
        let recovered = try engine.parseMutable("info", sourceLimit: .number(Double(bitPattern: 0x7ff8000000000042)))
        XCTAssertEqual(recovered.type, "info"); try recovered.close()
    }
    func testNonfiniteThresholdsKeepIndependentNativeSourceBudget() throws {
        let source = String(repeating: "x", count: 16 * 1024 * 1024 + 1)
        for limit in [Pele.SourceLimit.number(.infinity), .number(.nan)] {
            XCTAssertThrowsError(try Pele.Engine().parseMutable(source, sourceLimit: limit)) { error in
                XCTAssertEqual((error as? CompatibilityError)?.code, "limit")
                XCTAssertEqual((error as? CompatibilityError)?.message, "Native transport: declared input exceeds budget")
            }
        }
    }
}
