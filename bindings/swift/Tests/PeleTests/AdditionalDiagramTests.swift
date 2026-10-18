import Foundation
import CryptoKit
import XCTest
import Pele

final class AdditionalDiagramTests: XCTestCase {
    private let verifiesMutableCorpus = ProcessInfo.processInfo.environment["PELE_VERIFY_MUTABLE"] == "1"
    private var mutableDrawings = 0, mutableFailures = 0, mutableBudgetDifferences = 0
    private struct ModelExpectations: Decodable {
        struct Case: Decodable {
            struct Expected: Decodable { let model: ConfigValue?; let semanticModel: ConfigValue? }
            let expected: Expected
            private enum CodingKeys: CodingKey { case expected }
            init(from decoder: any Decoder) throws {
                expected = try autoreleasepool {
                    try decoder.container(keyedBy: CodingKeys.self).decode(Expected.self, forKey: .expected)
                }
            }
        }
        let cases: [Case]
    }
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
    private let mutableExportLimits: Set<String> = [
        "046702029bd9c44c84e60976877b377f08f77f0f4a874cde27c3985b9b232bc5",
        "56424df5324ecd9871e81aa3b3e06e3f8504a0fd521cc162e4330a725e13ffa0",
        "46758d3f9e7c0c9f083d20b7953fd94cbcffecc505a263f46e2daa7ad52010ad",
        "6e6e58ca26628fc90906c3c612eda9370df0ad472e8ce9d187d6ff11e502b141",
        "36ffadaa832afbe700a8086c1c6aebaffbb4ee8a9d0741037e8964085521907a"
    ]
    /// Opt-in full-corpus crossing of the actual mutable C/binary interface. The
    /// immutable lane independently compares pinned upstream outcomes below.
    private func verifyMutable(_ source: String, id: String, parse: Pele.Engine.ParseOptions,
                               options: Pele.Engine.Options, resources: Pele.HostResources,
                               expected: FullRenderedDiagram) throws {
        guard verifiesMutableCorpus else { return }
        let digest = SHA256.hash(data: Data(source.utf8)).map { String(format: "%02x", $0) }.joined()
        do {
            let document = try Pele.Engine().parseMutable(source, options: parse)
            defer { try? document.close() }
            var snapshot = try document.snapshot()
            let raw = try snapshot.graph.intern(Pele.JSString(codeUnits: [0xd800, 0, 0xdc00]))
            try snapshot.graph.setField("__swiftCorpusValue", value: .string(raw))
            try snapshot.graph.setField("__swiftCorpusSelf", value: snapshot.graph.root)
            try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
            let rendered = try document.render(options: options, resources: resources)
            mutableDrawings += 1
            XCTAssertEqual(rendered.svg, expected.svg, id)
            XCTAssertEqual(rendered.type, expected.type, id)
            XCTAssertEqual(rendered.width.bitPattern, expected.width.bitPattern, id)
            XCTAssertEqual(rendered.height.bitPattern, expected.height.bitPattern, id)
            let saved = try document.snapshot().graph
            guard case .string(let key) = try saved.field("__swiftCorpusValue"),
                  case .ref(let identity) = try saved.field("__swiftCorpusSelf"),
                  case .ref(let root) = saved.root else { return XCTFail("Mutable extras lost: \(id)") }
            XCTAssertEqual(identity, root, id)
            XCTAssertEqual(saved.strings[Int(key)].codeUnits, [0xd800, 0, 0xdc00], id)
            XCTAssertFalse(mutableExportLimits.contains(digest), "Declared limit unexpectedly closed: \(id)")
        } catch let error as CompatibilityError where mutableExportLimits.contains(digest) {
            XCTAssertEqual(error.code, "limit", id)
            XCTAssertTrue(error.message.contains("budget"), id)
            mutableBudgetDifferences += 1
            FileHandle.standardError.write(Data("Swift mutable explicit export limit: \(id)\n".utf8))
        }
    }
    private func verifyMutableFailure(_ source: String, id: String, parse: Pele.Engine.ParseOptions,
                                      options: Pele.Engine.Options, resources: Pele.HostResources,
                                      expected: CompatibilityError) throws {
        guard verifiesMutableCorpus else { return }
        let digest = SHA256.hash(data: Data(source.utf8)).map { String(format: "%02x", $0) }.joined()
        do {
            let document = try Pele.Engine().parseMutable(source, options: parse)
            defer { try? document.close() }
            _ = try document.render(options: options, resources: resources)
            XCTFail("Mutable error unexpectedly accepted: \(id)")
        } catch let actual as CompatibilityError {
            mutableFailures += 1
            if mutableExportLimits.contains(digest) {
                XCTAssertEqual(actual.code, "limit", id)
                XCTAssertTrue(actual.message.contains("budget"), id)
                mutableBudgetDifferences += 1
            } else {
                XCTAssertEqual(actual.name, expected.name, id)
                XCTAssertEqual(actual.code, expected.code, id)
                XCTAssertEqual(actual.message, expected.message, id)
                XCTAssertEqual(actual.type, expected.type, id)
                XCTAssertEqual(actual.line, expected.line, id)
                XCTAssertEqual(actual.column, expected.column, id)
                XCTAssertEqual(actual.snippet, expected.snippet, id)
            }
        }
    }
    func testPinnedDiagramsThroughNativeDocuments() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let decoder = JSONDecoder(), encoder = JSONEncoder(), engine = Pele.Engine()
        for family in ["pie", "packet", "radar", "quadrant", "timeline", "journey", "flowchart", "kanban", "mindmap", "class", "er", "xychart", "state", "gantt", "sequence", "git-graph", "architecture", "sankey", "treemap", "c4", "info", "requirement", "block", "railroad", "cynefin", "treeview", "agentflow", "wardley", "eventmodeling", "usecase", "venn", "ishikawa", "swimlane"] {
            if let families = ProcessInfo.processInfo.environment["PELE_FAMILIES"], !families.split(separator: ",").contains(Substring(family)) { continue }
            try autoreleasepool {
            FileHandle.standardError.write(Data("Swift corpus: \(family)\n".utf8))
            let publicType = family == "treeview" ? "treeView" : family == "git-graph" ? "gitGraph" : family
            let bytes = try fixtureData(root.appendingPathComponent("fixtures/pele/\(family).json"))
            let fixture = try JSONSerialization.jsonObject(with: bytes) as! [String: Any]
            let modelExpectations = try decoder.decode(ModelExpectations.self, from: bytes)
            let cases = fixture["cases"] as! [[String: Any]]
            XCTAssertFalse(cases.isEmpty, family)
            for (index, item) in cases.enumerated() {
                try autoreleasepool {
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
                            XCTAssertEqual((error as? CompatibilityError)?.type, publicType, id)
                        }
                        let rendered = try engine.render(source, options: options, resources: resources)
                        try verifyMutable(source, id: id, parse: parseOptions, options: options, resources: resources, expected: rendered)
                        let result = (try JSONSerialization.jsonObject(with: encoder.encode(rendered)) as! NSDictionary).mutableCopy() as! NSMutableDictionary
                        result.removeObject(forKey: "$numbers")
                        if (expected["result"] as? [String: Any])?["svgSha256"] != nil {
                            result.removeObject(forKey: "svg")
                            result["svgSha256"] = SHA256.hash(data: Data(rendered.svg.utf8)).map { String(format: "%02x", $0) }.joined()
                        }
                        XCTAssertTrue(result.isEqual(expected["result"] as? NSDictionary), id)
                        return
                    }
                    let document = try engine.parse(source, options: parseOptions)
                    // Decode directly from oracle bytes: Foundation's Any projection
                    // can retain decimal storage or strip leading U+FEFF from strings.
                    if let expectedModel = modelExpectations.cases[index].expected.model {
                        XCTAssertEqual(document.model, expectedModel, id)
                    }
                    if let expectedSemantic = modelExpectations.cases[index].expected.semanticModel {
                        XCTAssertEqual(try document.query("semanticModel"), expectedSemantic, id)
                    }
                    projected = true
                    let rendered = try engine.render(document, options: options, resources: resources)
                    try verifyMutable(source, id: id, parse: parseOptions, options: options, resources: resources, expected: rendered)
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
                    try verifyMutableFailure(source, id: id, parse: parseOptions, options: options, resources: resources, expected: error)
                    if expected["model"] != nil { XCTAssertTrue(projected, id) }
                    let value = try JSONSerialization.jsonObject(with: encoder.encode(error)) as! NSDictionary
                    let normalized = value.mutableCopy() as! NSMutableDictionary
                    if normalized["type"] == nil { normalized["type"] = NSNull() }
                    XCTAssertEqual(normalized, expected["error"] as? NSDictionary, id)
                }
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
                    XCTAssertEqual((error as? CompatibilityError)?.type, publicType)
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
        }
        if verifiesMutableCorpus {
            FileHandle.standardError.write(Data("Swift mutable corpus: \(mutableDrawings) drawings, \(mutableFailures) errors, \(mutableBudgetDifferences) explicit budget differences\n".utf8))
        }
        let capabilities = try engine.capabilities()
        XCTAssertEqual(Set(capabilities.filter { $0.registered }.map(\.type)), Set(["pie", "packet", "radar", "quadrantChart", "timeline", "journey", "flowchart", "kanban", "mindmap", "class", "er", "xychart", "state", "gantt", "sequence", "gitGraph", "architecture", "sankey", "treemap", "c4", "info", "requirement", "block", "railroad", "cynefin", "treeView", "agentflow", "wardley", "eventmodeling", "usecase", "venn", "ishikawa", "swimlane"]))
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
