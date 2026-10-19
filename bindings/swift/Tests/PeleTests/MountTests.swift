import AppKit
import XCTest
import Pele

private final class MountFailure: Error {}
private final class MountCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0
    func next() -> Int { lock.lock(); defer { lock.unlock() }; count += 1; return count }
    var value: Int { lock.lock(); defer { lock.unlock() }; return count }
}

private final class MountResourceFailure: @unchecked Sendable {
    private let lock = NSLock()
    private var failing = false
    let error = MountFailure()
    func set(_ value: Bool) { lock.lock(); failing = value; lock.unlock() }
    func width(_ request: Pele.TextMeasurement) throws -> Double {
        lock.lock(); let fails = failing; lock.unlock()
        if fails { throw error }
        return Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6)
    }
}

@MainActor final class MountTests: XCTestCase {
    private func window(width: CGFloat = 360) -> (NSWindow, Pele.DiagramView) {
        _ = NSApplication.shared
        let view = Pele.DiagramView(frame: NSRect(x: 0, y: 0, width: width, height: 240))
        let window = NSWindow(contentRect: view.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.contentView = view
        window.orderFront(nil)
        window.displayIfNeeded()
        XCTAssertTrue(window.isVisible)
        XCTAssertTrue(view.window === window)
        return (window, view)
    }
    private let a = "pie\n\"A\" : 1"
    private let b = "pie\n\"B\" : 2"
    private let bad = "this is not a diagram"
    private func pixels(_ image: NSImage?) throws -> Data {
        let image = try XCTUnwrap(image)
        var rect = NSRect(origin: .zero, size: image.size)
        let cg = try XCTUnwrap(image.cgImage(forProposedRect: &rect, context: nil, hints: nil))
        return try XCTUnwrap(cg.dataProvider?.data) as Data
    }
    private func painted(_ view: Pele.DiagramView) throws -> Data {
        let window = try XCTUnwrap(view.window)
        XCTAssertTrue(window.isVisible)
        window.displayIfNeeded(); view.displayIfNeeded()
        let bitmap = try XCTUnwrap(view.bitmapImageRepForCachingDisplay(in: view.bounds))
        view.cacheDisplay(in: view.bounds, to: bitmap)
        let bytes = try XCTUnwrap(bitmap.bitmapData)
        let stride = bitmap.bitsPerPixel / 8
        XCTAssertGreaterThan(stride, 0)
        let first = Array(UnsafeBufferPointer(start: bytes, count: stride))
        var nonuniform = false
        for y in 0..<bitmap.pixelsHigh {
            for x in 0..<bitmap.pixelsWide {
                let pixel = Array(UnsafeBufferPointer(start: bytes + y * bitmap.bytesPerRow + x * stride, count: stride))
                if pixel != first { nonuniform = true; break }
            }
            if nonuniform { break }
        }
        XCTAssertTrue(nonuniform, "Native window view must paint visible diagram/error pixels")
        return Data(bytes: bytes, count: bitmap.bytesPerRow * bitmap.pixelsHigh)
    }
    func testNativeWindowLastGoodDrawingAndDeferredRecovery() throws {
        let (window, view) = window(); defer { window.close() }
        var renders = [String](), errors = [any Error]()
        var options = Pele.MountOptions()
        options.onRender = { renders.append($0.svg) }; options.onError = { errors.append($0) }
        let mounted = try Pele.mount(view, source: a, options: options)
        let first = try pixels(view.presentation), firstPaint = try painted(view)
        XCTAssertEqual(renders.count, 1); XCTAssertNotNil(mounted.result)
        view.isHidden = true
        try mounted.update(b)
        XCTAssertEqual(renders.count, 1)
        try mounted.update(bad)
        XCTAssertEqual(errors.count, 1); XCTAssertEqual(mounted.source, bad)
        XCTAssertEqual(try pixels(view.presentation), first)
        view.isHidden = false
        try mounted.refresh()
        XCTAssertEqual(renders.count, 2)
        XCTAssertTrue(try XCTUnwrap(mounted.result).svg.contains("B"))
        XCTAssertNotNil(mounted.error); XCTAssertNotNil(view.diagnostic)
        let errorPaint = try painted(view)
        XCTAssertNotEqual(errorPaint, firstPaint, "Error overlay and dimming must reach actual view pixels")
        try mounted.update(b)
        XCTAssertNil(mounted.error); XCTAssertNil(view.diagnostic)
        XCTAssertNotEqual(try pixels(view.presentation), first)
        let recoveredPaint = try painted(view)
        XCTAssertNotEqual(recoveredPaint, errorPaint)
        XCTAssertNotEqual(recoveredPaint, firstPaint)
    }
    func testPreparationFailureRetainsPixelsAndCallbacksSeeCommittedState() throws {
        let (window, view) = window(); defer { window.close() }
        let mounted = try Pele.mount(view, source: a)
        let first = try pixels(view.presentation), oldSVG = mounted.result?.svg
        let failure = MountFailure()
        var options = Pele.MountOptions()
        options.preparePresentation = { _ in throw failure }
        options.onError = { error in
            XCTAssertTrue(error as AnyObject === failure)
            XCTAssertTrue(mounted.error as AnyObject? === failure)
            XCTAssertEqual(mounted.result?.svg, oldSVG)
        }
        try mounted.update(b, options: options)
        XCTAssertEqual(try pixels(view.presentation), first)
        var recovered = Pele.MountOptions()
        recovered.onRender = { drawing in
            XCTAssertEqual(mounted.result?.svg, drawing.svg)
            XCTAssertNotNil(view.presentation); XCTAssertNil(view.diagnostic)
            throw failure
        }
        XCTAssertThrowsError(try mounted.update(b, options: recovered)) { XCTAssertTrue($0 as AnyObject === failure) }
        XCTAssertNil(mounted.error); XCTAssertNotEqual(mounted.result?.svg, oldSVG)
    }
    func testReentryReplacementDestroyAndDetachedDrawing() throws {
        let view = Pele.DiagramView(frame: NSRect(x: 0, y: 0, width: 240, height: 240))
        let mounted = try Pele.mount(view, source: a)
        XCTAssertNotNil(mounted.result); XCTAssertNotNil(view.presentation)
        var options = Pele.MountOptions(), calls = 0
        options.onRender = { _ in
            calls += 1
            if calls == 1 { try mounted.update(self.a); mounted.destroy() }
        }
        try mounted.update(b, options: options)
        XCTAssertEqual(calls, 2); XCTAssertEqual(mounted.source, a)
        try mounted.refresh(); XCTAssertEqual(calls, 2)
        try mounted.update(b); XCTAssertEqual(calls, 3)
        let replacement = try Pele.mount(view, source: a)
        XCTAssertNotEqual(mounted.result?.svg, replacement.result?.svg)
        XCTAssertNotEqual(try XCTUnwrap(mounted.result).svg, try XCTUnwrap(replacement.result).svg)
    }
    func testZeroWidthInitialBusyGuardAndOriginalObserverException() throws {
        let (window, view) = window(width: 0); defer { window.close() }
        let mounted = try Pele.mount(view, source: a)
        XCTAssertNotNil(mounted.result)
        let initialSVG = mounted.result?.svg
        try mounted.update(b)
        XCTAssertEqual(mounted.source, b)
        XCTAssertEqual(mounted.result?.svg, initialSVG, "Attached zero-width update must defer replacement")
        view.setFrameSize(NSSize(width: 240, height: 240))
        try mounted.refresh()
        XCTAssertNotEqual(mounted.result?.svg, initialSVG)
        var options = Pele.MountOptions()
        let failure = MountFailure()
        options.preparePresentation = { drawing in
            XCTAssertThrowsError(try mounted.update(self.b)) { XCTAssertTrue(String(describing: $0).contains("already in use")) }
            XCTAssertThrowsError(try Pele.mount(view, source: self.a)) {
                XCTAssertTrue(String(describing: $0).contains("already in use"))
            }
            XCTAssertEqual(mounted.source, self.b, "Rejected precommit replacement must preserve the active owner")
            return try XCTUnwrap(NSImage(data: Data(drawing.svg.utf8)))
        }
        options.onRender = { _ in throw failure }
        XCTAssertThrowsError(try mounted.update(b, options: options)) { XCTAssertTrue($0 as AnyObject === failure) }
        XCTAssertEqual(mounted.source, b)
        mounted.destroy(); try mounted.refresh()
    }

    func testDeferredRenderFailureRetainsLastGoodWindowPixelsAndObserverFailureIdentity() throws {
        let (window, view) = window(); defer { window.close() }
        let mounted = try Pele.mount(view, source: a)
        let first = try pixels(view.presentation), firstSVG = mounted.result?.svg
        let observerFailure = MountFailure()
        var errors = 0, options = Pele.MountOptions()
        options.onError = { error in
            errors += 1
            XCTAssertEqual((error as? CompatibilityError)?.type, "sankey")
            XCTAssertEqual(mounted.result?.svg, firstSVG)
            XCTAssertNotNil(mounted.error)
            throw observerFailure
        }
        view.isHidden = true
        try mounted.update("sankey\nA,B,10\nB,A,10", options: options)
        XCTAssertEqual(errors, 0)
        view.isHidden = false
        view.layout()
        XCTAssertEqual(errors, 1)
        XCTAssertTrue(mounted.unhandledCallbackError as AnyObject? === observerFailure)
        XCTAssertEqual(try pixels(view.presentation), first)
        XCTAssertNotNil(view.diagnostic)
        _ = try painted(view)
        try mounted.refresh()
        XCTAssertEqual(errors, 1, "Unchanged refresh must not retry failed deferred model")
    }

    func testErrorNotificationCanRecoverSynchronouslyAndInitialCallbackFailureLeavesImage() throws {
        let (window, view) = window(); defer { window.close() }
        let mounted = try Pele.mount(view, source: a)
        var trace = [String](), recovery = Pele.MountOptions(), failing = Pele.MountOptions()
        recovery.onRender = { drawing in
            trace.append("recovered")
            XCTAssertEqual(mounted.source, self.b)
            XCTAssertEqual(mounted.result?.svg, drawing.svg)
            XCTAssertNil(mounted.error); XCTAssertNil(view.diagnostic)
        }
        failing.onError = { error in
            trace.append("error")
            // Structured Pele errors are Swift values; boxing them as AnyObject
            // creates separate bridges. Compare every public field instead.
            let published = try XCTUnwrap(mounted.error as? CompatibilityError)
            let reported = try XCTUnwrap(error as? CompatibilityError)
            XCTAssertEqual(published.name, reported.name); XCTAssertEqual(published.code, reported.code)
            XCTAssertEqual(published.message, reported.message); XCTAssertEqual(published.type, reported.type)
            XCTAssertEqual(published.line, reported.line); XCTAssertEqual(published.column, reported.column)
            XCTAssertEqual(published.snippet, reported.snippet)
            try mounted.update(self.b, options: recovery)
        }
        try mounted.update(bad, options: failing)
        XCTAssertEqual(trace, ["error", "recovered"])
        XCTAssertEqual(mounted.source, b); XCTAssertNil(mounted.error)
        _ = try painted(view)

        let (initialWindow, initialView) = self.window(); defer { initialWindow.close() }
        let callbackFailure = MountFailure()
        var initial = Pele.MountOptions()
        initial.onRender = { _ in throw callbackFailure }
        XCTAssertThrowsError(try Pele.mount(initialView, source: a, options: initial)) {
            XCTAssertTrue($0 as AnyObject === callbackFailure)
        }
        XCTAssertNotNil(initialView.presentation); XCTAssertNil(initialView.diagnostic)
        _ = try painted(initialView)
        var notifications = 0, failed = Pele.MountOptions()
        failed.onError = { _ in notifications += 1 }
        let handle = try Pele.mount(initialView, source: bad, options: failed)
        XCTAssertNil(handle.result); XCTAssertNotNil(handle.error)
        XCTAssertNil(initialView.presentation); XCTAssertNotNil(initialView.diagnostic)
        XCTAssertEqual(notifications, 1)
        _ = try painted(initialView)
    }

    func testAdaptiveFollowingWidthIsQuietAndMountIDsRemainOwned() throws {
        let (window, view) = window(width: 80); defer { window.close() }
        let measurements = MountCounter()
        var options = Pele.MountOptions(), preparations = 0
        options.resources.textWidth = { request in
            _ = measurements.next()
            return Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6)
        }
        options.preparePresentation = { drawing in
            preparations += 1
            return try XCTUnwrap(NSImage(data: Data(drawing.svg.utf8)))
        }
        let mounted = try Pele.mount(view, source: "sankey\nA,B,3\nB,C,2", options: options)
        let drawing = try XCTUnwrap(mounted.result), previousMeasurements = measurements.value
        let previousImage = view.presentation
        view.setFrameSize(NSSize(width: drawing.width, height: 240))
        try mounted.refresh()
        XCTAssertEqual(measurements.value, previousMeasurements, "Following drawing's own width must not replay resource callbacks")
        XCTAssertEqual(preparations, 1); XCTAssertTrue(view.presentation === previousImage)
        try mounted.invalidateResources()
        XCTAssertGreaterThan(measurements.value, previousMeasurements)

        let source = "flowchart LR\naccTitle: Native IDs\nA --> B"
        let (otherWindow, otherView) = self.window(); defer { otherWindow.close() }
        let one = try Pele.mount(view, source: source), two = try Pele.mount(otherView, source: source)
        func prefix(_ result: FullRenderedDiagram?) throws -> String {
            let svg = try XCTUnwrap(result?.svg)
            let range = try XCTUnwrap(svg.range(of: "pele-native-[0-9a-f-]{36}", options: .regularExpression))
            return String(svg[range])
        }
        let original = try prefix(one.result)
        XCTAssertNotEqual(original, try prefix(two.result))
        try one.update(source)
        XCTAssertEqual(try prefix(one.result), original)
        var explicit = Pele.MountOptions(); explicit.render.idPrefix = "consumer-owned"
        try one.update(source, options: explicit)
        XCTAssertTrue(try XCTUnwrap(one.result).svg.contains("consumer-owned-title"))
        _ = try painted(view); _ = try painted(otherView)
    }

    func testGanttNaturalResultSurvivesQuietResizeAndResourceInvalidationRefreshesClock() throws {
        let (window, view) = window(); defer { window.close() }
        let clocks = MountCounter()
        var options = Pele.MountOptions()
        options.render.maxWidth = 0
        options.resources.timeZone = TimeZone(secondsFromGMT: 0)!
        options.resources.nowMillis = { 1_704_067_200_000 + Double(clocks.next() - 1) * 86_400_000 }
        let source = "gantt\ndateFormat YYYY-MM-DD\nsection Start\nWork:active,1d"
        let mounted = try Pele.mount(view, source: source, options: options)
        let initial = try XCTUnwrap(mounted.result), clockCount = clocks.value
        let image = view.presentation
        XCTAssertGreaterThan(clockCount, 0)
        view.setFrameSize(NSSize(width: 220, height: 240))
        try mounted.refresh()
        XCTAssertEqual(clocks.value, clockCount)
        XCTAssertTrue(view.presentation === image); XCTAssertEqual(mounted.result?.svg, initial.svg)
        try mounted.invalidateResources()
        XCTAssertGreaterThan(clocks.value, clockCount)
        XCTAssertNotEqual(mounted.result?.svg, initial.svg)
        XCTAssertNil(mounted.error)
        _ = try painted(view)
    }
    func testResourceRefreshFailureRecoversWithoutReplacingSourceOrLastGoodPixels() throws {
        let (window, view) = window(); defer { window.close() }
        let resources = MountResourceFailure()
        var options = Pele.MountOptions(), errors = [any Error]()
        options.resources.textWidth = { try resources.width($0) }
        options.onError = { errors.append($0) }
        let mounted = try Pele.mount(view, source: a, options: options)
        let drawing = mounted.result?.svg, image = view.presentation, before = try painted(view)
        resources.set(true)
        try mounted.invalidateResources()
        XCTAssertEqual(errors.count, 1)
        XCTAssertTrue(errors.first as AnyObject? === resources.error)
        XCTAssertTrue(mounted.error as AnyObject? === resources.error)
        XCTAssertEqual(mounted.source, a); XCTAssertEqual(mounted.result?.svg, drawing)
        XCTAssertTrue(view.presentation === image)
        XCTAssertNotEqual(try painted(view), before)
        resources.set(false)
        try mounted.invalidateResources()
        XCTAssertNil(mounted.error); XCTAssertNil(view.diagnostic)
        XCTAssertEqual(mounted.source, a); XCTAssertEqual(mounted.result?.svg, drawing)
        XCTAssertEqual(try painted(view), before)
        XCTAssertEqual(errors.count, 1)
    }

    func testInitialInvalidZeroWidthMountDefersLaterValidUpdateUntilShown() throws {
        let (window, view) = window(width: 0); defer { window.close() }
        var options = Pele.MountOptions(), renders = 0, errors = 0
        options.onRender = { _ in renders += 1 }; options.onError = { _ in errors += 1 }
        let mounted = try Pele.mount(view, source: bad, options: options)
        XCTAssertNil(mounted.result); XCTAssertNotNil(mounted.error)
        XCTAssertEqual(errors, 1); XCTAssertEqual(renders, 0)
        try mounted.update(b)
        XCTAssertEqual(mounted.source, b)
        XCTAssertNil(mounted.result); XCTAssertNotNil(view.diagnostic)
        XCTAssertEqual(errors, 1); XCTAssertEqual(renders, 0)
        view.setFrameSize(NSSize(width: 240, height: 240))
        try mounted.refresh()
        XCTAssertNotNil(mounted.result); XCTAssertNil(mounted.error); XCTAssertNil(view.diagnostic)
        XCTAssertEqual(renders, 1)
        _ = try painted(view)
    }

}
