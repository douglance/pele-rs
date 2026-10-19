import Foundation
import XCTest
import Pele

/// Array editing crosses the actual binary C bridge into the native chart renderer.
final class MutableArrayEditingTests: XCTestCase {
    private let prefix = "xychart-beta\nx-axis 0 --> 4\ny-axis 0 --> 10\nbar "
    private func reference(_ atom: Pele.PublicGraph.Atom?) throws -> UInt32 {
        guard case .ref(let id) = atom else { throw PeleError(code: "test", message: "Expected reference") }; return id
    }
    private func data(_ graph: Pele.PublicGraph) throws -> UInt32 {
        let plots = try reference(graph.field("plots")), plot = try reference(graph.field("0", on: plots))
        return try reference(graph.field("data", on: plot))
    }
    private func point(_ graph: inout Pele.PublicGraph, values: UInt32, value: Double) throws -> Pele.PublicGraph.Atom {
        // Retain the real returned tuple shape and edit its numeric ordinate.
        let template = try reference(graph.field("0", on: values))
        let id = UInt32(graph.nodes.count); graph.nodes.append(graph.nodes[Int(template)])
        try graph.setField("1", value: .number(value), on: id)
        return .ref(id)
    }
    private func bytes(_ graph: Pele.PublicGraph) throws -> Data {
        let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys]; return try encoder.encode(graph)
    }
    private func expected(_ values: String) throws -> String { try Pele.Engine().render(prefix + values).svg }
    private func assertUnchanged(_ graph: inout Pele.PublicGraph, edit: (inout Pele.PublicGraph) throws -> Void) throws {
        let before = try bytes(graph); XCTAssertThrowsError(try edit(&graph)); XCTAssertEqual(try bytes(graph), before)
    }
    func testXYChartAppendTruncateGrowAndStaleEditsRenderEquivalentSource() throws {
        let document = try Pele.Engine().parseMutable(prefix + "[1,2]")
        defer { try? document.close() }
        let original = try document.snapshot()
        var graph = original.graph; let values = try data(graph)
        let three = try point(&graph, values: values, value: 3)
        try graph.setField("2", value: three, on: values)
        try graph.setField("01", value: .number(9), on: values)
        try graph.setField("4294967295", value: .number(9), on: values)
        try document.apply(graph, expectedRevision: original.revision)
        XCTAssertEqual(try document.render().svg, try expected("[1,2,3]"))
        XCTAssertThrowsError(try document.apply(original.graph, expectedRevision: original.revision))
        XCTAssertEqual(try document.render().svg, try expected("[1,2,3]"))
        graph = try document.snapshot().graph
        try graph.setField("length", value: .number(1), on: values)
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try document.render().svg, try expected("[1]"))
        graph = try document.snapshot().graph
        guard case .array(let length, let properties) = graph.nodes[Int(values)] else { return XCTFail("Array") }
        XCTAssertEqual(length, 1)
        XCTAssertFalse(properties.contains { graph.strings[Int($0.key)].stringIfWellFormed == "length" }, "Default length remains implicit")
        XCTAssertNotNil(try graph.field("01", on: values)); XCTAssertNotNil(try graph.field("4294967295", on: values))
        try graph.setArrayLength(3, on: values)
        let four = try point(&graph, values: values, value: 4), five = try point(&graph, values: values, value: 5)
        try graph.setArrayElement(1, value: four, on: values)
        try graph.setArrayElement(2, value: five, on: values)
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try document.render().svg, try expected("[1,4,5]"))
        graph = try document.snapshot().graph
        for invalid in [-1.0, 1.5, .nan, .infinity, 4_294_967_296] {
            try assertUnchanged(&graph) { try $0.setField("length", value: .number(invalid), on: values) }
        }
        try assertUnchanged(&graph) { try $0.setArrayElement(.max, value: .number(8), on: values) }
        try assertUnchanged(&graph) { try $0.setArrayLength(0, on: .max) }
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try document.render().svg, try expected("[1,4,5]"))
    }
    func testReadonlyAndNonconfigurableArraysRejectStructuralEditsAndRecover() throws {
        let document = try Pele.Engine().parseMutable(prefix + "[1,2]")
        defer { try? document.close() }
        let initial = try document.snapshot().graph
        var graph = initial; let values = try data(graph)
        let lengthKey = try graph.intern("length")
        graph.nodes[Int(values)].properties.append(.init(key: lengthKey, value: .number(2), flags: 0))
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try document.render().svg, try expected("[1,2]"))
        graph = try document.snapshot().graph
        try assertUnchanged(&graph) { try $0.setField("2", value: .number(3), on: values) }
        try assertUnchanged(&graph) { try $0.setField("length", value: .number(1), on: values) }
        let four = try point(&graph, values: values, value: 4)
        try graph.setField("0", value: four, on: values)
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try document.render().svg, try expected("[4,2]"))
        graph = initial
        var properties = graph.nodes[Int(values)].properties
        let second = try XCTUnwrap(properties.firstIndex { graph.strings[Int($0.key)].stringIfWellFormed == "1" })
        properties[second].flags = 3
        graph.nodes[Int(values)].properties = properties
        try document.apply(graph, expectedRevision: document.revision)
        graph = try document.snapshot().graph
        try assertUnchanged(&graph) { try $0.setArrayLength(1, on: values) }
        let six = try point(&graph, values: values, value: 6)
        try graph.setArrayElement(1, value: six, on: values)
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try document.render().svg, try expected("[1,6]"))
        graph = try document.snapshot().graph
        let saved = try XCTUnwrap(graph.nodes[Int(values)].properties.first { graph.strings[Int($0.key)].stringIfWellFormed == "1" })
        XCTAssertEqual(saved.flags, 3, "Updating preserves the nonconfigurable descriptor")
        properties = graph.nodes[Int(values)].properties
        let first = try XCTUnwrap(properties.firstIndex { graph.strings[Int($0.key)].stringIfWellFormed == "0" })
        properties[first].flags = 5
        graph.nodes[Int(values)].properties = properties
        try document.apply(graph, expectedRevision: document.revision)
        graph = try document.snapshot().graph
        try assertUnchanged(&graph) { try $0.setField("0", value: .number(7), on: values) }
        try document.apply(initial, expectedRevision: document.revision)
        graph = try document.snapshot().graph
        let three = try point(&graph, values: values, value: 3)
        try graph.setArrayElement(2, value: three, on: values)
        try document.apply(graph, expectedRevision: document.revision)
        XCTAssertEqual(try document.render().svg, try expected("[1,2,3]"))
    }
}
