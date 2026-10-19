import Foundation
import XCTest
import Pele

private final class ResourceModel { var attempts = 0 }
private final class ResourceMarker: Error {}
@MainActor private final class ResourceState {
    var events: [String] = []
    var context: Pele.ProviderRenderContext?
    var actions: [(Pele.ProviderRenderContext) throws -> Void] = []
}

// Real Swift provider -> C borrowed capability -> Rust session -> Swift host.
// Only public consumer interfaces; no @testable access or mock native dispatch.
@MainActor final class ProviderResourceTests: XCTestCase {
    private let civil = Pele.CivilDateTime(year: 2024, month: 1, day: 2, hour: 3)
    private func drawing(_ contents: String = "") -> Pele.ProviderRendered {
        .init(svg: "<svg xmlns=\"http://www.w3.org/2000/svg\">\(contents)</svg>", width: 20, height: 10)
    }
    func testResourcesReachHostWithSessionPoliciesAndPortableFallbacks() throws {
        let state = ResourceState(), registry = try Pele.Registry.empty()
        let provider = Pele.Diagram(type: "pie", parse: { _ in ResourceModel() }, render: { _, context in
            state.context = context
            XCTAssertEqual(try context.textWidth("label", size: 13, style: 1), 17)
            let first = try context.icon("ﬂ°°65¶ß")
            XCTAssertEqual(first?.markup, "<g id=\"A\"/>")
            XCTAssertEqual(try context.icon("ﬂ°°65¶ß")?.markup, first?.markup)
            XCTAssertEqual(try context.icon("empty")?.markup, "")
            XCTAssertNil(try context.icon("missing"))
            XCTAssertEqual(try context.nowMillis(), 42.875)
            XCTAssertEqual(try context.nowMillis(), 42.875)
            XCTAssertEqual(try context.parseDate("fraction"), 123)
            XCTAssertNil(try context.parseDate("missing"))
            let fields = try context.calendarFields(123.875)
            XCTAssertEqual(fields?.civil.year, 2024)
            XCTAssertEqual(fields?.utcOffsetMinutes, 330)
            XCTAssertNil(try context.calendarFields(.nan))
            XCTAssertNil(try context.calendarFields(8.64e15 + 1))
            XCTAssertEqual(try context.calendarInstant(self.civil), -0.0)
            return self.drawing(first!.markup)
        })
        try registry.register(provider)
        let document = try registry.parseProvider("pie", as: ResourceModel.self)
        var host = Pele.HostResources()
        host.textWidth = { request in MainActor.assumeIsolated {
            state.events.append("width:\(request.text)"); return 17
        }}
        host.icon = { name in MainActor.assumeIsolated {
            state.events.append("icon:\(name)")
            if name == "missing" { return nil }
            return .init(trustedMarkup: name == "empty" ? "" : "<g id=\"\(name)\"/>")
        }}
        host.nowMillis = { MainActor.assumeIsolated { state.events.append("clock"); return 42.875 } }
        host.parseDate = { name in MainActor.assumeIsolated {
            state.events.append("date:\(name)"); return name == "missing" ? nil : 123.875
        }}
        host.calendarFields = { instant in MainActor.assumeIsolated {
            state.events.append("fields:\(instant)")
            return .init(civil: .init(year: 2024, month: 1, day: 2), weekday: 2, utcOffsetMinutes: 330)
        }}
        host.calendarInstant = { civil in MainActor.assumeIsolated {
            state.events.append("instant:\(civil.year)"); return -0.875
        }}
        var options = Pele.Engine.Options(); options.now = .nan
        XCTAssertTrue(try document.render(options: options, resources: host).svg.contains("id=\"A\""))
        XCTAssertEqual(state.events, ["width:label", "icon:A", "icon:A", "icon:empty", "icon:missing", "clock", "date:fraction", "date:missing", "fields:123.0", "instant:2024"])
        provider.render = { _, context in
            XCTAssertEqual(try context.nowMillis().bitPattern, (-0.0).bitPattern)
            XCTAssertEqual(try context.nowMillis().bitPattern, (-0.0).bitPattern)
            return self.drawing()
        }
        options.now = -0.0; state.events = []
        _ = try document.render(options: options, resources: host); XCTAssertEqual(state.events, [])
        provider.render = { _, context in
            XCTAssertNil(try context.icon("absent"))
            XCTAssertEqual(try context.parseDate("1970-01-01T00:00:00Z"), 0)
            XCTAssertNil(try context.parseDate("invalid"))
            let fields = try context.calendarFields(0)
            XCTAssertEqual(fields?.civil.year, 1970); XCTAssertEqual(fields?.weekday, 4)
            XCTAssertEqual(try context.calendarInstant(.init(year: 1970, month: 1, day: 1)), 0)
            return self.drawing()
        }
        var defaults = Pele.HostResources(); defaults.timeZone = TimeZone(secondsFromGMT: 0)!
        _ = try document.render(resources: defaults)
        try document.close(); try registry.close()
    }
    func testOriginalErrorsPartialStateCaughtRecoveryAndClockFailureCache() throws {
        let registry = try Pele.Registry.empty(), marker = ResourceMarker(), state = ResourceState()
        let actions: [(Pele.ProviderRenderContext) throws -> Void] = [
            { _ = try $0.textWidth("x", size: 13) }, { _ = try $0.icon("x") },
            { _ = try $0.nowMillis() }, { _ = try $0.parseDate("x") },
            { _ = try $0.calendarFields(0) }, { _ = try $0.calendarInstant(self.civil) }
        ]
        var host = Pele.HostResources()
        host.textWidth = { _ in throw marker }; host.icon = { _ in throw marker }
        host.nowMillis = { try MainActor.assumeIsolated { state.events.append("clock"); throw marker } }
        host.parseDate = { _ in throw marker }; host.calendarFields = { _ in throw marker }
        host.calendarInstant = { _ in throw marker }
        let provider = Pele.Diagram(type: "pie", parse: { _ in ResourceModel() }, render: { _, _ in self.drawing() })
        try registry.register(provider)
        let document = try registry.parseProvider("pie", as: ResourceModel.self), model = try document.model
        for action in actions {
            provider.render = { model, context in model.attempts += 1; try action(context); return self.drawing() }
            do { _ = try document.render(outputLimit: .number(0), resources: host); XCTFail("Host error precedes output limit") }
            catch { XCTAssertTrue(error as AnyObject === marker) }
            provider.render = { model, context in
                model.attempts += 1
                for _ in 0..<2 { do { try action(context); XCTFail("Expected host error") }
                    catch { XCTAssertTrue(error as AnyObject === marker) } }
                return self.drawing("<g id=\"recovered\"/>")
            }
            XCTAssertTrue(try document.render(resources: host).svg.contains("recovered"))
        }
        XCTAssertEqual(model.attempts, 12); XCTAssertEqual(document.revision, 12)
        XCTAssertEqual(state.events, ["clock", "clock"])
        try document.close(); try registry.close()
    }
    func testEveryResourceRejectsExpiredAndCrossResourceReentryAndValidatesCalendar() throws {
        let registry = try Pele.Registry.empty(), state = ResourceState()
        let actions: [(Pele.ProviderRenderContext) throws -> Void] = [
            { _ = try $0.textWidth("x", size: 13) }, { _ = try $0.icon("x") },
            { _ = try $0.nowMillis() }, { _ = try $0.parseDate("x") },
            { _ = try $0.calendarFields(0) }, { _ = try $0.calendarInstant(self.civil) }
        ]
        let provider = Pele.Diagram(type: "pie", parse: { _ in ResourceModel() }, render: { _, context in
            state.context = context
            XCTAssertEqual(try context.icon("outer")?.markup, "<g/>")
            let fields = try context.calendarFields(1.75)
            XCTAssertEqual(fields?.civil.year, 2024)
            return self.drawing()
        })
        try registry.register(provider)
        let document = try registry.parseProvider("pie", as: ResourceModel.self)
        var host = Pele.HostResources()
        state.actions = actions
        host.icon = { _ in MainActor.assumeIsolated {
            for action in state.actions {
                do { try action(state.context!); XCTFail("Cross-resource reentry") }
                catch { XCTAssertEqual((error as? PeleError)?.message, "Provider resources are already in use") }
            }
            return .init(trustedMarkup: "<g/>")
        }}
        host.calendarFields = { _ in .init(civil: .init(year: 2024, month: 1, day: 1), weekday: 1, utcOffsetMinutes: 0) }
        _ = try document.render(resources: host)
        for action in actions {
            do { try action(state.context!); XCTFail("Expired borrowed call") }
            catch { XCTAssertEqual((error as? PeleError)?.message, "Provider resources are no longer active") }
        }
        provider.render = { _, context in _ = try context.calendarFields(0); return self.drawing() }
        host.calendarFields = { _ in .init(civil: .init(year: 2024, month: 13, day: 1), weekday: 1, utcOffsetMinutes: 0) }
        XCTAssertThrowsError(try document.render(resources: host)) { error in
            XCTAssertEqual((error as? CompatibilityError)?.message, "Host calendar returned invalid fields")
        }
        provider.render = { _, context in
            XCTAssertNil(try context.parseDate("over-limit"))
            XCTAssertNil(try context.calendarInstant(self.civil))
            return self.drawing()
        }
        host.parseDate = { _ in 8.64e15 + 1 }; host.calendarInstant = { _ in 8.64e15 + 1 }
        _ = try document.render(resources: host)
        try document.close(); try registry.close()
    }
}
