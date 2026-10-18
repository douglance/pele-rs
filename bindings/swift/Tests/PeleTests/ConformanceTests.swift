import Foundation
import XCTest
import Pele

final class ConformanceTests: XCTestCase {
    func testSharedFeatureCorpusThroughNativeLibrary() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("../../../..").standardizedFileURL
        let fixtures = root.appendingPathComponent("fixtures")
        let manifest = try JSONSerialization.jsonObject(with: Data(contentsOf: fixtures.appendingPathComponent("manifest.json"))) as! [[String: Any]]
        let output = root.appendingPathComponent("dist/evidence/swift")
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        for fixture in manifest {
            let name = fixture["name"] as! String
            let source = try (fixture["source"] as? String) ?? String(contentsOf: fixtures.appendingPathComponent(fixture["file"] as! String), encoding: .utf8)
            if let expected = fixture["error"] as? String {
                XCTAssertThrowsError(try Pele.render(source), name) { error in
                    XCTAssertEqual((error as? PeleError)?.code, expected, name)
                }
            } else {
                let diagram = try Pele.render(source)
                XCTAssertEqual(diagram.geometry.nodes.count, fixture["nodes"] as! Int, name)
                XCTAssertEqual(diagram.geometry.edges.count, fixture["edges"] as! Int, name)
                XCTAssertEqual(diagram.geometry.direction, fixture["direction"] as! String, name)
                let encoder = JSONEncoder(); encoder.keyEncodingStrategy = .convertToSnakeCase
                try encoder.encode(diagram).write(to: output.appendingPathComponent("\(name).json"))
            }
        }
        XCTAssertNoThrow(try Pele.render("graph LR; A-->B"))
    }
}
