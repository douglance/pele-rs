import Foundation
import XCTest
import Pele

private final class PresentationFailure: Error {}

/// Consumer features cross the actual Swift/C/Rust stack; no generated SVG parsing.
final class PresentationTests: XCTestCase {
    private let source = "flowchart LR\naccTitle: Orders &amp; links\naccDescr: Native description\nA[Start] -->|next| B[Finish]\nclick A \"https://example.test/orders\"\nclick B \"javascript:alert(1)\""
    private func frame(_ drawing: FullRenderedDiagram, file: StaticString = #filePath, line: UInt = #line) throws -> Pele.PresentationArtifact {
        guard case .available(let frame) = drawing.presentation else {
            XCTFail("Expected native frame", file: file, line: line)
            throw NSError(domain: "PresentationTests", code: 1)
        }
        XCTAssertEqual(frame.version, 1, file: file, line: line)
        XCTAssertEqual(frame.width, drawing.width, file: file, line: line)
        XCTAssertEqual(frame.height, drawing.height, file: file, line: line)
        return frame
    }
    private func unavailable(_ drawing: FullRenderedDiagram, _ expected: Pele.PresentationUnavailable, file: StaticString = #filePath, line: UInt = #line) {
        guard case .unavailable(let reason) = drawing.presentation else { return XCTFail("Expected explicit unavailable metadata", file: file, line: line) }
        XCTAssertEqual(reason, expected, file: file, line: line)
    }
    func testSourceRetainedAndMutablePublishSameApprovedAccessibleFrame() throws {
        let engine = Pele.Engine(), immutable = try engine.parse(source), mutable = try engine.parseMutable(source)
        defer { try? mutable.close() }
        let plain = try engine.render(source)
        XCTAssertNil(plain.presentation)
        let drawings = [try engine.renderPresentation(source), try engine.renderPresentation(immutable), try mutable.renderPresentation()]
        var encodedFrames: [Data] = []
        for drawing in drawings {
            XCTAssertEqual(drawing.svg, plain.svg)
            let frame = try frame(drawing)
            XCTAssertEqual(frame.title, "Orders & links")
            XCTAssertEqual(frame.description, "Native description")
            XCTAssertEqual(frame.text.map(\.text), ["Start", "Finish", "next"])
            XCTAssertEqual(frame.links.count, 1)
            let link = try XCTUnwrap(frame.links.first)
            XCTAssertEqual(link.id, "A"); XCTAssertEqual(link.href, "https://example.test/orders")
            XCTAssertFalse(link.internal)
            XCTAssertGreaterThan(link.bounds.width, 0); XCTAssertGreaterThan(link.bounds.height, 0)
            let encoder = JSONEncoder(); encoder.outputFormatting = .sortedKeys
            encodedFrames.append(try encoder.encode(frame))
            let roundTrip = try JSONDecoder().decode(FullRenderedDiagram.self, from: encoder.encode(drawing))
            XCTAssertEqual(roundTrip.svg, drawing.svg)
            XCTAssertEqual(try encoder.encode(self.frame(roundTrip)), encodedFrames.last)
        }
        XCTAssertTrue(encodedFrames.dropFirst().allSatisfy { $0 == encodedFrames.first })
        XCTAssertEqual(mutable.revision, 1)
        let legacy = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(plain)) as? [String: Any])
        XCTAssertNil(legacy["presentation"])
    }
    func testMetadataBudgetAndUnsupportedNeverChangeSuccessfulSVGOrCallbacks() throws {
        let engine = Pele.Engine()
        for budget in [Pele.PresentationBudget(maxElements: 0), .init(maxTextBytes: 1)] {
            let plain = try engine.render(source), drawing = try engine.renderPresentation(source, budget: budget)
            XCTAssertEqual(drawing.svg, plain.svg); unavailable(drawing, .budgetExceeded)
        }
        let pie = "pie\n\"A\":1"
        let drawing = try engine.renderPresentation(pie)
        XCTAssertEqual(drawing.svg, try engine.render(pie).svg); unavailable(drawing, .unsupportedDiagram)
        var options = Pele.Engine.Options(); options.padding = .nan
        let nonfinite = try engine.renderPresentation(source, options: options)
        XCTAssertEqual(nonfinite.svg, try engine.render(source, options: options).svg)
        XCTAssertEqual(nonfinite.width.isNaN, try engine.render(source, options: options).width.isNaN)
        unavailable(nonfinite, .invalidGeometry)
        XCTAssertThrowsError(try engine.renderPresentation(source, budget: .init(maxElements: -1)))
    }
    func testMutableFailurePublishesRevisionAndOriginalErrorThenRecovers() throws {
        let engine = Pele.Engine(), document = try engine.parseMutable(source)
        defer { try? document.close() }
        XCTAssertEqual(try frame(document.renderPresentation()).title, "Orders & links")
        XCTAssertEqual(document.revision, 1)
        XCTAssertThrowsError(try document.renderPresentation(outputLimit: .number(0)))
        XCTAssertEqual(document.revision, 2)
        let failure = PresentationFailure()
        var host = Pele.HostResources(); host.textWidth = { _ in throw failure }
        XCTAssertThrowsError(try document.renderPresentation(resources: host)) { XCTAssertTrue(($0 as? PresentationFailure) === failure) }
        XCTAssertEqual(document.revision, 3)
        XCTAssertEqual(try frame(document.renderPresentation(outputLimit: .number(.nan))).links.count, 1)
        XCTAssertEqual(document.revision, 4)
        var options = Pele.Engine.Options(); options.links = false
        XCTAssertTrue(try frame(document.renderPresentation(options: options)).links.isEmpty)
        XCTAssertEqual(document.revision, 5)
    }
    func testEveryCurrentProducerCrossesSwiftWithActualTextAndFrames() throws {
        let engine = Pele.Engine()
        let sources = [
            "swimlane-beta LR\nA[Start] --> B[Finish]\nclick A \"https://example.test/a\"",
            "agentflow-beta LR\na[\"First task\"] --> b[\"Local note\"]\nclick a href \"https://example.test/task\"",
            "erDiagram\nCustomer:::internal-link {\n int id PK\n string name \"Display name\"\n}\nCustomer ||--o{ Order : places",
            "---\nconfig:\n  kanban:\n    ticketBaseUrl: 'https://example.test/#TICKET#'\n---\nkanban\n todo[Todo]\n  a[First card]@{ ticket: T-1 }"
        ]
        for source in sources {
            let document = try engine.parseMutable(source); defer { try? document.close() }
            let drawing = try document.renderPresentation(), frame = try frame(drawing)
            XCTAssertFalse(frame.text.isEmpty, drawing.type)
            XCTAssertFalse(frame.links.isEmpty, drawing.type)
            XCTAssertEqual(drawing.svg, try engine.render(source).svg)
        }
    }
}
