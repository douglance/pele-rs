import AppKit
import XCTest
import Pele

@MainActor final class LazyMountTests: XCTestCase {
    private let pie = "pie\n\"A\" : 1"
    private let info = "info"
    private func fixture() -> (NSWindow, Pele.DiagramView) {
        _ = NSApplication.shared
        let view = Pele.DiagramView(frame: NSRect(x: 0, y: 0, width: 360, height: 240))
        let window = NSWindow(contentRect: view.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false; window.contentView = view; window.orderFront(nil)
        return (window, view)
    }
    private func options() -> Pele.MountOptions {
        var options = Pele.MountOptions(); options.zoom = .disabled; return options
    }
    func testLazyMountSelectsInitialFamilyAndExplicitUpdatesRecover() async throws {
        let registry = try Pele.Registry.empty()
        let (window, view) = fixture(); defer { window.close(); try? registry.close() }
        let mounted = try await registry.mountAsync(view, source: pie, options: options())
        XCTAssertTrue(try registry.registered("pie")); XCTAssertFalse(try registry.registered("info"))
        XCTAssertEqual(mounted.result?.type, "pie"); XCTAssertNil(mounted.error); XCTAssertNotNil(view.presentation)
        let first = mounted.result?.svg
        try mounted.update(info)
        XCTAssertNotNil(mounted.error); XCTAssertEqual(mounted.result?.svg, first)
        XCTAssertFalse(try registry.registered("info"), "A synchronous update must not silently select another family")
        _ = try registry.load("info")
        try mounted.update(info)
        XCTAssertNil(mounted.error); XCTAssertEqual(mounted.result?.type, "info")
        mounted.destroy()
    }
    func testLazyMountLoadsBeforeLimitFailureAndRecovers() async throws {
        let registry = try Pele.Registry.empty()
        let (window, view) = fixture(); defer { window.close(); try? registry.close() }
        var limited = options(); limited.limits.source = .number(-1)
        var callbacks = 0
        limited.onError = { _ in callbacks += 1 }
        let mounted = try await registry.mountAsync(view, source: pie, options: limited)
        XCTAssertTrue(try registry.registered("pie")); XCTAssertNotNil(mounted.error)
        XCTAssertNil(mounted.result); XCTAssertEqual(callbacks, 1)
        try mounted.update(pie, options: options())
        XCTAssertNil(mounted.error); XCTAssertEqual(mounted.result?.type, "pie")
        mounted.destroy()
    }
    func testSelectiveMountRetainsRegistryAndDocumentsAcrossClose() throws {
        var registry: Pele.Registry? = try .empty()
        weak var retained = registry
        _ = try registry?.load("pie")
        let (window, view) = fixture(); defer { window.close() }
        var mounted: Pele.Mounted? = try Pele.mount(view, source: pie, registry: XCTUnwrap(registry), options: options())
        registry = nil
        XCTAssertNotNil(retained, "Mounted updates retain their selective registration owner")
        let original = mounted?.result?.svg
        try retained?.close()
        try mounted?.invalidateResources()
        XCTAssertNil(mounted?.error); XCTAssertEqual(mounted?.result?.svg, original)
        try mounted?.update("pie\n\"B\" : 2")
        XCTAssertNotNil(mounted?.error); XCTAssertEqual(mounted?.result?.svg, original)
        mounted?.destroy(); mounted = nil
        // The view retains its current controller until replaced or released.
        let replacement = try Pele.mount(view, source: info, options: options())
        XCTAssertNil(retained)
        replacement.destroy()
    }
    func testCancelledMountLeavesExistingPresentationAndSelectionUntouched() async throws {
        let registry = try Pele.Registry.empty()
        let (window, view) = fixture(); defer { window.close(); try? registry.close() }
        let existing = try Pele.mount(view, source: info, options: options())
        let original = existing.result?.svg
        let operation = Task { @MainActor in
            do {
                _ = try await registry.mountAsync(view, source: self.pie, options: self.options())
                XCTFail("Cancelled mount must stop before selection or view replacement")
            } catch is CancellationError { } catch { XCTFail("Unexpected error: \(error)") }
        }
        operation.cancel()
        await operation.value
        XCTAssertFalse(try registry.registered("pie"))
        XCTAssertEqual(existing.result?.svg, original); XCTAssertNil(existing.error)
        try existing.update(info)
        XCTAssertNil(existing.error)
        existing.destroy()
    }
}
