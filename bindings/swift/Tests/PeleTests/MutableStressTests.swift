import Foundation
import CryptoKit
import XCTest
import Pele

/// Large owned graphs cross the binary ABI, without expanding typed models to JSON.
final class MutableStressTests: XCTestCase {
    private func cases(_ family: String, key: String) throws -> [[String: Any]] {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let path = root.appendingPathComponent("fixtures/pele/\(family).json")
        let bytes: Data
        if FileManager.default.fileExists(atPath: path.path) { bytes = try Data(contentsOf: path) }
        else {
            let process = Process(), pipe = Pipe()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/gzip")
            process.arguments = ["-dc", path.path + ".gz"]; process.standardOutput = pipe
            try process.run(); bytes = pipe.fileHandleForReading.readDataToEndOfFile(); process.waitUntilExit()
            guard process.terminationStatus == 0 else { throw PeleError(code: "test", message: "Fixture decompression failed") }
        }
        let fixture = try XCTUnwrap(JSONSerialization.jsonObject(with: bytes) as? [String: Any])
        return try XCTUnwrap(fixture[key] as? [[String: Any]])
    }
    private func settings(_ item: [String: Any]) throws -> (Pele.Engine.ParseOptions, Pele.Engine.Options) {
        let fields = ["config": [String: Any]()].merging(item["options"] as? [String: Any] ?? [:]) { _, value in value }
        let options = try JSONDecoder().decode(Pele.Engine.Options.self, from: JSONSerialization.data(withJSONObject: fields))
        var parse = Pele.Engine.ParseOptions()
        parse.limit = options.limit ?? 50_000
        parse.maxEdges = options.maxEdges ?? 5_000
        parse.config = options.config
        return (parse, options)
    }
    private func reference(_ atom: Pele.PublicGraph.Atom?) throws -> UInt32 {
        guard case .ref(let id) = atom else { throw PeleError(code: "test", message: "Expected graph reference") }; return id
    }
    private func number(_ atom: Pele.PublicGraph.Atom?) throws -> Double {
        guard case .number(let value) = atom else { throw PeleError(code: "test", message: "Expected graph number") }; return value
    }
    private func array(_ graph: Pele.PublicGraph, _ atom: Pele.PublicGraph.Atom?) throws -> (UInt32, [Pele.PublicGraph.Property]) {
        let id = try reference(atom)
        guard case .array(let length, let properties) = graph.nodes[Int(id)] else { throw PeleError(code: "test", message: "Expected graph Array") }
        return (length, properties)
    }
    private func addExtras(_ graph: inout Pele.PublicGraph) throws {
        let raw = try graph.intern(Pele.JSString(codeUnits: [0xd800, 0, 0xdc00]))
        try graph.setField("__swiftStressValue", value: .string(raw))
        try graph.setField("__swiftStressSelf", value: graph.root)
    }
    private func verifyExtras(_ graph: Pele.PublicGraph) throws {
        guard case .string(let id) = try graph.field("__swiftStressValue") else { return XCTFail("Lost UTF16 value") }
        XCTAssertEqual(graph.strings[Int(id)].codeUnits, [0xd800, 0, 0xdc00])
        XCTAssertEqual(try reference(graph.field("__swiftStressSelf")), try reference(graph.root))
    }
    private func render(_ item: [String: Any], inspect: (Pele.PublicGraph) throws -> Void) throws {
        let source = try XCTUnwrap(item["source"] as? String), id = try XCTUnwrap(item["id"] as? String)
        let (parse, options) = try settings(item), engine = Pele.Engine()
        let expected = try XCTUnwrap((item["expected"] as? [String: Any])?["result"] as? [String: Any])
        // render(source) retains the typed model internally; do not call legacy parse/model JSON here.
        let immutable = try engine.render(source, options: options)
        XCTAssertEqual(SHA256.hash(data: Data(immutable.svg.utf8)).map { String(format: "%02x", $0) }.joined(), expected["svgSha256"] as? String, id)
        let document = try engine.parseMutable(source, options: parse)
        defer { try? document.close() }
        var snapshot = try document.snapshot()
        try inspect(snapshot.graph)
        try addExtras(&snapshot.graph)
        try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
        let mutable = try document.render(options: options)
        XCTAssertEqual(mutable.type, immutable.type, id)
        XCTAssertEqual(mutable.svg, immutable.svg, id)
        XCTAssertEqual(mutable.width.bitPattern, immutable.width.bitPattern, id)
        XCTAssertEqual(mutable.height.bitPattern, immutable.height.bitPattern, id)
        try verifyExtras(document.snapshot().graph)
    }
    func testPacketRepeatedLargeLabelUsesCompactBinaryGraphAndExactRendering() throws {
        try autoreleasepool {
            let item = try XCTUnwrap(cases("packet", key: "stress").first { $0["id"] as? String == "stress/repeated-40000-label-10000-rows" })
            let summary = try XCTUnwrap((item["expected"] as? [String: Any])?["modelSummary"] as? [String: Any])
            try render(item) { graph in
                let (length, rows) = try array(graph, graph.field("rows"))
                XCTAssertEqual(Int(length), summary["rows"] as? Int)
                XCTAssertEqual(rows.count, Int(length))
                XCTAssertEqual(try number(graph.field("bitsPerRow")), 1)
                var labelID: UInt32?
                for (index, row) in rows.enumerated() {
                    let (count, entries) = try array(graph, row.value)
                    XCTAssertEqual(count, 1)
                    let block = try reference(try XCTUnwrap(entries.first).value)
                    XCTAssertEqual(try number(graph.field("start", on: block)), Double(index))
                    XCTAssertEqual(try number(graph.field("end", on: block)), Double(index))
                    guard case .string(let label) = try graph.field("label", on: block) else { return XCTFail("Missing Packet label") }
                    if let labelID { XCTAssertEqual(label, labelID) } else { labelID = label }
                }
                let label = graph.strings[Int(try XCTUnwrap(labelID))].codeUnits
                XCTAssertEqual(label.count, summary["labelLength"] as? Int)
                XCTAssertTrue(label.allSatisfy { $0 == 120 })
            }
        }
    }
    func testMindmapDeepTypedCasesRenderThroughOwnedBinaryDocuments() throws {
        try autoreleasepool {
            let items = try cases("mindmap", key: "typedCases")
            XCTAssertEqual(Set(items.compactMap { $0["id"] as? String }), Set(["deep-80", "deep-310", "deep-3000"]))
            for item in items {
                try autoreleasepool {
                    let count = try XCTUnwrap((item["expected"] as? [String: Any])?["nodeCount"] as? Int)
                    try render(item) { graph in
                        let (length, properties) = try array(graph, graph.field("nodes"))
                        XCTAssertEqual(Int(length), count)
                        XCTAssertEqual(properties.count, count)
                        let last = try reference(try XCTUnwrap(properties.last).value)
                        XCTAssertEqual(try number(graph.field("level", on: last)), Double(count - 1))
                    }
                }
            }
        }
    }
    func testFlowchartLargeEdgeParseRetainsEveryEdgeThroughBinaryApply() throws {
        try autoreleasepool {
            let item = try XCTUnwrap(cases("flowchart", key: "generatedStress").first { $0["id"] as? String == "huge-47917-edges" })
            let source = try XCTUnwrap(item["source"] as? String), expected = try XCTUnwrap(item["expected"] as? [String: Any])
            let (parse, _) = try settings(item)
            XCTAssertEqual(parse.maxEdges, 50_000)
            let document = try Pele.Engine().parseMutable(source, options: parse)
            defer { try? document.close() }
            func inspect(_ graph: Pele.PublicGraph) throws {
                let nodes = try reference(graph.field("nodes"))
                guard case .map(let entries, _) = graph.nodes[Int(nodes)] else { return XCTFail("Flowchart nodes Map") }
                XCTAssertEqual(entries.count, expected["nodes"] as? Int)
                let (length, edges) = try array(graph, graph.field("edges"))
                XCTAssertEqual(Int(length), expected["edges"] as? Int)
                XCTAssertEqual(edges.count, Int(length))
                let first = try reference(try XCTUnwrap(edges.first).value)
                guard case .string(let kind) = try graph.field("type", on: first) else { return XCTFail("Flowchart edge type") }
                XCTAssertEqual(graph.strings[Int(kind)].stringIfWellFormed, expected["firstEdgeType"] as? String)
            }
            var snapshot = try document.snapshot()
            try inspect(snapshot.graph)
            try addExtras(&snapshot.graph)
            try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
            let saved = try document.snapshot().graph
            try inspect(saved); try verifyExtras(saved)
        }
    }
}
