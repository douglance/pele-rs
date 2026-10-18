import AppKit
import XCTest
import Pele

private final class ProviderMountFailure: Error {}
private final class ProviderMountModel { var renders = 0; var width = 80.0 }
private final class ProviderMountState {
    var generation = "A"
    var parses = 0
    var models = [ProviderMountModel]()
    var parseFailure: ProviderMountFailure?
    var renderFailure: ProviderMountFailure?
}
@MainActor final class ProviderMountTests: XCTestCase {
    private func fixture() -> (NSWindow, Pele.DiagramView) {
        _ = NSApplication.shared
        let view = Pele.DiagramView(frame: NSRect(x: 0, y: 0, width: 360, height: 180))
        let window = NSWindow(contentRect: view.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false; window.contentView = view; window.orderFront(nil)
        return (window, view)
    }
    private func options() -> Pele.MountOptions {
        var options = Pele.MountOptions(); options.zoom = .disabled; options.render.maxWidth = 0; return options
    }
    private func provider(_ state: ProviderMountState) -> Pele.Diagram<ProviderMountModel> {
        .init(type: "info", parse: { _ in
            state.parses += 1
            if let error = state.parseFailure { throw error }
            let model = ProviderMountModel(); state.models.append(model); return model
        }, render: { model, _ in
            model.renders += 1
            if let error = state.renderFailure { throw error }
            return .init(svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"\(model.width)\" height=\"24\"><rect data-provider=\"\(state.generation)\" width=\"\(model.width)\" height=\"24\" fill=\"red\"/></svg>", width: model.width, height: 24)
        })
    }
    func testCustomAsyncMountRetainsModelAcrossRefreshAndRegistryClose() async throws {
        let registry = try Pele.Registry.empty(), state = ProviderMountState()
        try registry.register(provider(state))
        let (window, view) = fixture(); defer { window.close() }
        let mounted = try await registry.mountAsync(view, source: "info", options: options())
        XCTAssertNil(mounted.error); XCTAssertNotNil(view.presentation)
        XCTAssertTrue(try XCTUnwrap(mounted.result).svg.contains("data-provider=\"A\""))
        let model = try XCTUnwrap(state.models.first); model.width = 123; state.generation = "live"
        try mounted.invalidateResources()
        XCTAssertEqual(state.parses, 1); XCTAssertEqual(mounted.result?.width, 123)
        XCTAssertTrue(try XCTUnwrap(mounted.result).svg.contains("data-provider=\"live\""))
        try registry.close(); try mounted.invalidateResources()
        XCTAssertNil(mounted.error); XCTAssertEqual(state.parses, 1)
        XCTAssertGreaterThanOrEqual(model.renders, 3)
        mounted.destroy()
    }
    func testReplacementAndBuiltinRestorationSelectOnlyFutureParses() throws {
        let registry = try Pele.Registry.empty(), first = ProviderMountState(), second = ProviderMountState()
        defer { try? registry.close() }
        second.generation = "B"
        try registry.register(provider(first))
        let (window, view) = fixture(); defer { window.close() }
        let mounted = try Pele.mount(view, source: "info title One", registry: registry, options: options())
        try registry.register(provider(second)); try mounted.invalidateResources()
        XCTAssertTrue(try XCTUnwrap(mounted.result).svg.contains("data-provider=\"A\""))
        XCTAssertEqual(second.parses, 0)
        try mounted.update("info title Two")
        XCTAssertNil(mounted.error); XCTAssertEqual(second.parses, 1)
        XCTAssertTrue(try XCTUnwrap(mounted.result).svg.contains("data-provider=\"B\""))
        try registry.registerBuiltin("info"); try mounted.update("info title Native")
        XCTAssertNil(mounted.error); XCTAssertFalse(try XCTUnwrap(mounted.result).svg.contains("data-provider="))
        XCTAssertEqual(first.parses, 1); XCTAssertEqual(second.parses, 1)
        mounted.destroy()
    }
    func testOriginalProviderFailureRetainsLastDrawingAndRecoveryUsesSameModel() throws {
        let registry = try Pele.Registry.empty(), state = ProviderMountState()
        defer { try? registry.close() }
        try registry.register(provider(state))
        let (window, view) = fixture(); defer { window.close() }
        let mounted = try Pele.mount(view, source: "info", registry: registry, options: options())
        let old = mounted.result?.svg, image = view.presentation, failure = ProviderMountFailure()
        state.renderFailure = failure
        try mounted.invalidateResources()
        XCTAssertTrue((mounted.error as? ProviderMountFailure) === failure)
        XCTAssertEqual(mounted.result?.svg, old); XCTAssertTrue(view.presentation === image)
        XCTAssertEqual(state.parses, 1); XCTAssertEqual(state.models.count, 1)
        state.renderFailure = nil; state.models[0].width = 111
        try mounted.invalidateResources()
        XCTAssertNil(mounted.error); XCTAssertEqual(mounted.result?.width, 111); XCTAssertEqual(state.parses, 1)
        mounted.destroy()
    }
    func testSourcePolicyPrecedesMalformedPreparationForBothDocumentLanes() throws {
        let registry = try Pele.Registry.empty(), state = ProviderMountState()
        defer { try? registry.close() }
        try registry.register(provider(state))
        let (window, view) = fixture(); defer { window.close() }
        var limited = options(); limited.limits.source = .number(-1)
        let mounted = try Pele.mount(view, source: "---\nconfig: [\n---\ninfo", registry: registry, options: limited)
        XCTAssertEqual((mounted.error as? CompatibilityError)?.code, "limit")
        XCTAssertEqual(state.parses, 0)
        try mounted.update("info", options: options())
        XCTAssertNil(mounted.error); XCTAssertEqual(state.parses, 1)
        mounted.destroy()
    }
}
