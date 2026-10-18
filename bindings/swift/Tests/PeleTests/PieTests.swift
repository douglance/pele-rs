import Foundation
import XCTest
import Pele

final class PieTests: XCTestCase {
    func testFullPinnedPieCorpusThroughOwnedDocumentsAndResources() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let fixture = try JSONSerialization.jsonObject(with: Data(contentsOf: root.appendingPathComponent("fixtures/pele/pie.json"))) as! [String: Any]
        let encoder = JSONEncoder(), decoder = JSONDecoder(), engine = Pele.Engine()
        for item in fixture["cases"] as! [[String: Any]] {
            let id = item["id"] as! String, source = item["source"] as! String
            let settings = item["options"] as! [String: Any]
            let merged = ["config": [String: Any]()].merging(settings) { _, v in v }
            let options = try decoder.decode(Pele.Engine.Options.self, from: JSONSerialization.data(withJSONObject: merged))
            var parseOptions = Pele.Engine.ParseOptions()
            parseOptions.limit = options.limit ?? 50_000
            parseOptions.maxEdges = options.maxEdges ?? 5_000
            parseOptions.config = options.config
            let host = item["host"] as! [String: Any]
            var resources = Pele.HostResources()
            if host["measure"] as? String == "fixed" {
                resources.textWidth = { r in Double(r.text.utf16.count) * r.size * (r.style & 1 != 0 ? 0.71 : 0.6) }
            }
            if let icons = host["icons"] as? [String: String] {
                resources.icon = { name in icons[name].map { Pele.TrustedSVG(trustedMarkup: $0) } }
            }
            let expected = item["expected"] as! [String: Any]
            do {
                let document = try engine.parse(source, options: parseOptions)
                let model = try JSONSerialization.jsonObject(with: encoder.encode(document.model)) as! NSDictionary
                XCTAssertEqual(model, expected["model"] as? NSDictionary, id)
                let rendered = try engine.render(document, options: options, resources: resources)
                let result = try JSONSerialization.jsonObject(with: encoder.encode(rendered)) as! NSDictionary
                XCTAssertEqual(result, expected["result"] as? NSDictionary, id)
            } catch let error as CompatibilityError {
                let value = try JSONSerialization.jsonObject(with: encoder.encode(error)) as! NSDictionary
                let normalized = value.mutableCopy() as! NSMutableDictionary
                if normalized["type"] == nil { normalized["type"] = NSNull() }
                XCTAssertEqual(normalized, expected["error"] as? NSDictionary, id)
            }
        }
        let capabilities = try engine.capabilities()
        XCTAssertTrue(capabilities.contains { $0.type == "pie" && $0.registered })
        XCTAssertFalse(capabilities.contains { !["pie", "quadrantChart", "packet", "radar", "timeline", "journey", "flowchart", "kanban", "mindmap", "class", "er", "xychart", "state", "gantt", "sequence", "gitGraph", "architecture", "sankey", "treemap", "c4", "info", "requirement", "block", "railroad"].contains($0.type) && $0.registered })
        let document = try engine.parse("---\nconfig: {pie: {legendPosition: bottom}}\n---\npie\n\"One\":1")
        var ignoredParseOptions = Pele.Engine.Options(); ignoredParseOptions.limit = 0
        XCTAssertNoThrow(try engine.render(document, options: ignoredParseOptions))
    }
}
