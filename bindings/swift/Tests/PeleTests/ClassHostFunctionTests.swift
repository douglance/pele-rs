import Foundation
import XCTest
import Pele

private final class ClassHostMarker: Error {}

@MainActor final class ClassHostFunctionTests: XCTestCase {
    private let source = "classDiagram\nclass A {\n +value$\n +run(x) int*\n}\nA --> B"

    private func member(_ graph: Pele.PublicGraph) throws -> Pele.PublicGraph.Atom {
        guard case .ref(let classes) = try graph.field("classes"),
              case .map(let entries, _) = graph.nodes[Int(classes)],
              case .ref(let first) = entries.first?.value,
              case .ref(let members) = try graph.field("members", on: first),
              let value = try graph.field("0", on: members) else { throw ClassHostMarker() }
        return value
    }
    private func reference(_ atom: Pele.PublicGraph.Atom) throws -> UInt32 {
        guard case .ref(let id) = atom else { throw ClassHostMarker() }; return id
    }
    private func install(_ display: Pele.HostFunction, _ classifier: Pele.HostFunction, on document: Pele.MutableDocument) throws -> Pele.PublicGraph.Atom {
        let displayReference = try document.bindHostFunction(display), classifierReference = try document.bindHostFunction(classifier)
        var graph = try document.snapshot().graph
        let value = try member(graph), id = try reference(value)
        try graph.setField("getDisplayDetails", value: displayReference, on: id)
        try graph.setField("parseClassifier", value: classifierReference, on: id)
        try document.apply(graph, expectedRevision: document.revision)
        return value
    }
    func testNestedOriginalCallsAndCapturedDirectInvocationKeepAliasesAndLateRows() throws {
        let document = try Pele.Engine().parseMutable(source)
        defer { try? document.close() }
        var trace: [String] = [], escaped: Pele.ActiveModel?
        let classifier = Pele.HostFunction { view, receiver, arguments in
            XCTAssertTrue(arguments.isEmpty); trace.append("classifier")
            try view.set("classifier", on: receiver, to: view.intern("*"))
            return try view.invokeBuiltin(method: 27, receiver: receiver)
        }
        let display = Pele.HostFunction { view, receiver, _ in
            escaped = view; trace.append("display")
            let result = try view.invokeBuiltin(method: 26, receiver: receiver)
            let root = try view.root()
            try view.set("savedMember", on: root, to: receiver)
            try view.set("savedDisplay", on: root, to: result)
            try view.set("displayText", on: result, to: view.intern("Live label"))
            return result
        }
        let value = try install(display, classifier, on: document)
        var resources = Pele.HostResources()
        resources.textWidth = { request in MainActor.assumeIsolated {
            trace.append("width:\(request.text):\(request.style)")
            return Double(request.text.utf16.count) * request.size * 0.6
        } }
        let drawing = try document.render(outputLimit: .undefined, resources: resources)
        XCTAssertTrue(drawing.svg.contains("Live label"))
        XCTAssertTrue(trace.first?.hasPrefix("width:A:") == true)
        let index = try XCTUnwrap(trace.firstIndex(of: "display"))
        XCTAssertEqual(Array(trace[index...index + 2]), ["display", "classifier", "width:Live label:2"])
        let graph = try document.snapshot().graph
        XCTAssertEqual(try reference(try XCTUnwrap(graph.field("savedMember"))), try reference(value))
        XCTAssertThrowsError(try escaped?.root())
        let direct = try document.invoke(method: 26, receiver: value)
        guard case .returned(let details) = direct.result else { return XCTFail("Captured original failed") }
        let detailsID = try reference(details)
        guard case .string(let css) = try direct.graph.field("cssStyle", on: detailsID) else { return XCTFail() }
        XCTAssertEqual(direct.graph.strings[Int(css)].stringIfWellFormed, "font-style:italic;")
    }
    func testNestedSwiftErrorIdentityPartialMutationAndRecovery() throws {
        let document = try Pele.Engine().parseMutable(source), marker = ClassHostMarker()
        defer { try? document.close() }
        var fail = true, caught = false
        let classifier = Pele.HostFunction { view, receiver, _ in
            try view.set("partial", on: receiver, to: view.intern("Retained"))
            if fail { throw marker }
            return try view.invokeBuiltin(method: 27, receiver: receiver)
        }
        let display = Pele.HostFunction { view, receiver, _ in
            do { _ = try view.invokeBuiltin(method: 26, receiver: receiver); XCTFail("Expected nested failure") }
            catch { XCTAssertTrue((error as AnyObject) === marker); caught = true }
            return try view.invokeBuiltin(method: 26, receiver: receiver)
        }
        let value = try install(display, classifier, on: document)
        XCTAssertThrowsError(try document.render(outputLimit: .undefined)) { XCTAssertTrue(($0 as AnyObject) === marker) }
        XCTAssertTrue(caught)
        let graph = try document.snapshot().graph, id = try reference(value)
        guard case .string(let text) = try graph.field("partial", on: id) else { return XCTFail() }
        XCTAssertEqual(graph.strings[Int(text)].stringIfWellFormed, "Retained")
        fail = false
        display.call = { view, receiver, _ in try view.invokeBuiltin(method: 26, receiver: receiver) }
        XCTAssertTrue(try document.render(outputLimit: .undefined).svg.contains("value"))
    }
}
