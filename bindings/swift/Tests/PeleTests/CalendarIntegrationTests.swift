import Foundation
import XCTest
import Pele

final class CalendarIntegrationTests: XCTestCase {
    func testGanttSchedulingAcrossTimeZonesGapsFoldsAndExcludedDays() throws {
        let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().appendingPathComponent("../../../..").standardizedFileURL
        let data = try Data(contentsOf: root.appendingPathComponent("fixtures/pele/calendar.json"))
        let fixture = try JSONSerialization.jsonObject(with: data) as! [String: Any]
        let engine = Pele.Engine(), encoder = JSONEncoder(), decoder = JSONDecoder()
        for item in fixture["cases"] as! [[String: Any]] {
            let id = item["id"] as! String, source = item["source"] as! String
            let options = try decoder.decode(Pele.Engine.Options.self, from: JSONSerialization.data(withJSONObject: item["options"]!))
            let expected = item["expected"] as! [String: Any]
            var resources = Pele.HostResources()
            resources.timeZone = TimeZone(identifier: item["zone"] as! String)!
            resources.textWidth = { r in Double(r.text.utf16.count) * r.size * (r.style & 1 != 0 ? 0.71 : 0.6) }
            do {
                let document = try engine.parse(source)
                let result = try engine.render(document, options: options, resources: resources)
                let value = try JSONSerialization.jsonObject(with: encoder.encode(result)) as! NSDictionary
                XCTAssertEqual(value, expected["result"] as? NSDictionary, id)
                XCTAssertEqual(try engine.render(source, options: options, resources: resources).svg, result.svg, id)
            } catch let error as CompatibilityError {
                let value = try JSONSerialization.jsonObject(with: encoder.encode(error)) as! NSDictionary
                XCTAssertEqual(value, expected["error"] as? NSDictionary, id)
            }
        }
    }

    func testCalendarHostFailuresKeepIdentityAndEngineRecovers() throws {
        final class HostFailure: Error, @unchecked Sendable {}
        let engine = Pele.Engine(), failure = HostFailure()
        let source = "gantt\ndateFormat YYYY-MM-DD\nA :2024-03-01,2d"
        var resources = Pele.HostResources()
        resources.calendarFields = { _ in throw failure }
        XCTAssertThrowsError(try engine.render(source, resources: resources)) { error in
            XCTAssertTrue((error as? HostFailure) === failure)
        }
        resources.calendarFields = { _ in .init(civil: .init(year: 2024, month: 3, day: 1), weekday: 7, utcOffsetMinutes: 0) }
        XCTAssertThrowsError(try engine.render(source, resources: resources)) { error in
            XCTAssertEqual((error as? CompatibilityError)?.code, "semantic")
        }
        resources = .init(); resources.timeZone = TimeZone(secondsFromGMT: 0)!
        XCTAssertEqual(try engine.render(source, resources: resources).type, "gantt")
        resources.parseDate = { _ in throw failure }
        XCTAssertThrowsError(try engine.render("gantt\ndateFormat INVALID\nA :Jan 1 2024,1d", resources: resources)) { error in
            XCTAssertTrue((error as? HostFailure) === failure)
        }
    }
}
