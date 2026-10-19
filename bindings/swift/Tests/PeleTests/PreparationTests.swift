import Foundation
import XCTest
import Pele
final class PreparationTests: XCTestCase {
    func testPinnedDocumentsThroughSwiftAndC() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let bytes = try Data(contentsOf: root.appendingPathComponent("fixtures/pele/source.json"))
        let fixture = try JSONSerialization.jsonObject(with: bytes) as! [String: Any]
        let encoder = JSONEncoder()
        for item in fixture["cases"] as! [[String: Any]] {
            let id = item["id"] as! String
            let source = item["source"] as! String
            let options = try JSONDecoder().decode(PrepareOptions.self, from: JSONSerialization.data(withJSONObject: ["limit": 50_000, "config": [:]].merging(item["options"] as! [String: Any]) { _, b in b }))
            let expected = item["expected"] as! [String: Any]
            if let expectedError = expected["error"] as? NSDictionary {
                XCTAssertThrowsError(try Pele.inspect(source, options: options), id) { error in
                    guard let error = error as? CompatibilityError, let data = try? encoder.encode(error), let value = try? JSONSerialization.jsonObject(with: data) as? NSDictionary else { return XCTFail("\(id): unexpected error \(error)") }
                    // Synthesized Codable omits nil optionals; upstream's error includes type:null.
                    let normalized = value.mutableCopy() as! NSMutableDictionary
                    if normalized["type"] == nil { normalized["type"] = NSNull() }
                    XCTAssertEqual(normalized, expectedError, id)
                }
            } else {
                let value = try Pele.inspect(source, options: options)
                let data = try encoder.encode(value)
                let actual = try JSONSerialization.jsonObject(with: data) as! NSDictionary
                let normalized = actual.mutableCopy() as! NSMutableDictionary
                if normalized["type"] == nil { normalized["type"] = NSNull() }
                XCTAssertEqual(normalized, expected as NSDictionary, id)
            }
        }
        let capabilities = try Pele.capabilities()
        XCTAssertEqual(capabilities.count, 33)
        XCTAssertTrue(capabilities.allSatisfy { $0.known && !$0.registered && !$0.renderParity })
    }
}
