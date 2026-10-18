import Foundation
import CryptoKit
import XCTest
import Pele

final class AdditionalDiagramTests: XCTestCase {
    private func fixtureData(_ url: URL) throws -> Data {
        if FileManager.default.fileExists(atPath: url.path) { return try Data(contentsOf: url) }
        let process = Process(), pipe = Pipe()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/gzip")
        process.arguments = ["-dc", url.path + ".gz"]
        process.standardOutput = pipe
        try process.run()
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else { throw NSError(domain: "FixtureGzip", code: Int(process.terminationStatus)) }
        return data
    }
    func testPinnedDiagramsThroughNativeDocuments() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let decoder = JSONDecoder(), encoder = JSONEncoder(), engine = Pele.Engine()
        for family in ["packet", "radar", "quadrant", "timeline", "journey", "flowchart", "kanban", "mindmap", "class", "er", "xychart", "state", "gantt", "sequence", "git-graph", "architecture", "sankey"] {
            if let families = ProcessInfo.processInfo.environment["PELE_FAMILIES"], !families.split(separator: ",").contains(Substring(family)) { continue }
            let fixture = try JSONSerialization.jsonObject(with: fixtureData(root.appendingPathComponent("fixtures/pele/\(family).json"))) as! [String: Any]
            let cases = fixture["cases"] as! [[String: Any]]
            XCTAssertFalse(cases.isEmpty, family)
            for item in cases {
                let id = "\(family)/\(item["id"]!)", source = item["source"] as! String
                let settings = ["config": [String: Any]()].merging(item["options"] as? [String: Any] ?? [:]) { _, value in value }
                let options = try decoder.decode(Pele.Engine.Options.self, from: JSONSerialization.data(withJSONObject: settings))
                var parseOptions = Pele.Engine.ParseOptions()
                parseOptions.limit = options.limit ?? 50_000
                parseOptions.maxEdges = options.maxEdges ?? 5_000
                parseOptions.config = options.config
                let host = item["host"] as? [String: Any] ?? [:]
                var resources = Pele.HostResources()
                resources.timeZone = TimeZone(secondsFromGMT: 0)!
                if host["measure"] as? String == "fixed" {
                    resources.textWidth = { r in Double(r.text.utf16.count) * r.size * (r.style & 1 != 0 ? 0.71 : 0.6) }
                }
                if host["allIcons"] as? Bool == true {
                    resources.icon = { _ in Pele.TrustedSVG(trustedMarkup: "<path d=\"M0,0\"/>") }
                } else if let icons = host["icons"] as? [String: String] {
                    resources.icon = { name in icons[name].map { Pele.TrustedSVG(trustedMarkup: $0) } }
                }
                let expected = item["expected"] as! [String: Any]
                var projected = false
                do {
                    if host["modelProjection"] as? String == "excluded" || expected["modelProjectionLimit"] as? Bool == true {
                        XCTAssertThrowsError(try engine.parse(source, options: parseOptions)) { error in
                            XCTAssertEqual((error as? CompatibilityError)?.code, "limit", id)
                            XCTAssertEqual((error as? CompatibilityError)?.type, family, id)
                        }
                        let rendered = try engine.render(source, options: options, resources: resources)
                        let result = (try JSONSerialization.jsonObject(with: encoder.encode(rendered)) as! NSDictionary).mutableCopy() as! NSMutableDictionary
                        result.removeObject(forKey: "$numbers")
                        if (expected["result"] as? [String: Any])?["svgSha256"] != nil {
                            result.removeObject(forKey: "svg")
                            result["svgSha256"] = SHA256.hash(data: Data(rendered.svg.utf8)).map { String(format: "%02x", $0) }.joined()
                        }
                        XCTAssertTrue(result.isEqual(expected["result"] as? NSDictionary), id)
                        continue
                    }
                    let document = try engine.parse(source, options: parseOptions)
                    let model = try JSONSerialization.jsonObject(with: encoder.encode(document.model)) as! NSDictionary
                    if let expectedModel = expected["model"] as? NSDictionary {
                        XCTAssertEqual(model, expectedModel, id)
                    }
                    projected = true
                    let rendered = try engine.render(document, options: options, resources: resources)
                    let result = (try JSONSerialization.jsonObject(with: encoder.encode(rendered)) as! NSDictionary).mutableCopy() as! NSMutableDictionary
                    result.removeObject(forKey: "$numbers")
                    if (expected["result"] as? [String: Any])?["svgSha256"] != nil {
                        result.removeObject(forKey: "svg")
                        result["svgSha256"] = SHA256.hash(data: Data(rendered.svg.utf8)).map { String(format: "%02x", $0) }.joined()
                    }
                    XCTAssertTrue(result.isEqual(expected["result"] as? NSDictionary), id)
                    // Retained model and source are separate ABI paths.
                    let sourceResult = try engine.render(source, options: options, resources: resources)
                    XCTAssertEqual(sourceResult.svg, rendered.svg, id)
                } catch let error as CompatibilityError {
                    if expected["model"] != nil { XCTAssertTrue(projected, id) }
                    let value = try JSONSerialization.jsonObject(with: encoder.encode(error)) as! NSDictionary
                    let normalized = value.mutableCopy() as! NSMutableDictionary
                    if normalized["type"] == nil { normalized["type"] = NSNull() }
                    XCTAssertEqual(normalized, expected["error"] as? NSDictionary, id)
                }
            }
            // Keep this pathological model in Rust: projecting its shared labels to JSON
            // would duplicate 400 MB. Exercise the foreign render-source path instead.
            for item in (fixture["stress"] as? [[String: Any]] ?? []) + (fixture["typedCases"] as? [[String: Any]] ?? []) {
                let settings = ["config": [String: Any]()].merging(item["options"] as? [String: Any] ?? [:]) { _, value in value }
                let options = try decoder.decode(Pele.Engine.Options.self, from: JSONSerialization.data(withJSONObject: settings))
                var parseOptions = Pele.Engine.ParseOptions()
                parseOptions.limit = options.limit ?? 50_000
                parseOptions.config = options.config
                XCTAssertThrowsError(try engine.parse(item["source"] as! String, options: parseOptions)) { error in
                    XCTAssertEqual((error as? CompatibilityError)?.code, "limit")
                    XCTAssertEqual((error as? CompatibilityError)?.type, family)
                }
                let rendered = try engine.render(item["source"] as! String, options: options)
                let result = (try JSONSerialization.jsonObject(with: encoder.encode(rendered)) as! NSDictionary).mutableCopy() as! NSMutableDictionary
                result.removeObject(forKey: "svg")
                result["svgSha256"] = SHA256.hash(data: Data(rendered.svg.utf8)).map { String(format: "%02x", $0) }.joined()
                XCTAssertEqual(result, (item["expected"] as? [String: Any])?["result"] as? NSDictionary, "\(family)/stress")
            }
            for item in fixture["generatedStress"] as? [[String: Any]] ?? [] {
                let settings = ["config": [String: Any]()].merging(item["options"] as? [String: Any] ?? [:]) { _, value in value }
                let options = try decoder.decode(Pele.Engine.ParseOptions.self, from: JSONSerialization.data(withJSONObject: settings))
                let document = try engine.parse(item["source"] as! String, options: options)
                let model = try JSONSerialization.jsonObject(with: encoder.encode(document.model)) as! [String: Any]
                let expected = item["expected"] as! [String: Any]
                XCTAssertEqual((model["nodes"] as? [Any])?.count, expected["nodes"] as? Int)
                let edges = model["edges"] as! [[String: Any]]
                XCTAssertEqual(edges.count, expected["edges"] as? Int)
                XCTAssertEqual(edges.first?["type"] as? String, expected["firstEdgeType"] as? String)
            }
        }
        let capabilities = try engine.capabilities()
        XCTAssertEqual(Set(capabilities.filter { $0.registered }.map(\.type)), Set(["pie", "packet", "radar", "quadrantChart", "timeline", "journey", "flowchart", "kanban", "mindmap", "class", "er", "xychart", "state", "gantt", "sequence", "gitGraph", "architecture", "sankey"]))
    }
}


final class NonfiniteDiagramTests: XCTestCase {
    func testSankeyOwnedRenderAndCodableKeepNonfiniteHeight() throws {
        let engine = Pele.Engine(), source = "sankey\na,b,5e-324"
        let document = try engine.parse(source)
        let rendered = try engine.render(document)
        XCTAssertTrue(rendered.height.isNaN)
        XCTAssertTrue(try engine.render(source).height.isNaN)
        let restored = try JSONDecoder().decode(FullRenderedDiagram.self, from: JSONEncoder().encode(rendered))
        XCTAssertEqual(restored.height.bitPattern, rendered.height.bitPattern)
        XCTAssertEqual(restored.svg, rendered.svg)
    }
}
