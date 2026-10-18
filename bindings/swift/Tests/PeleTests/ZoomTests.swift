import AppKit
import CoreGraphics
import XCTest
import Pele

private final class ZoomResourceCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0
    func width(_ request: Pele.TextMeasurement) -> Double {
        lock.lock(); count += 1; lock.unlock()
        return Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6)
    }
    var value: Int { lock.lock(); defer { lock.unlock() }; return count }
}

@MainActor private final class ZoomFeatureWindow: NSWindow {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { true }
}
@MainActor private final class ZoomHostView: NSView {
    var forwardedKeys = [String]()
    override var acceptsFirstResponder: Bool { true }
    override func keyDown(with event: NSEvent) { forwardedKeys.append(event.charactersIgnoringModifiers ?? "") }
}
@MainActor private final class ZoomScrollHost: NSScrollView {
    var received = [CGFloat]()
    var events = [NSEvent]()
    var currentContexts = [Bool]()
    override func scrollWheel(with event: NSEvent) {
        received.append(event.scrollingDeltaY); events.append(event)
        currentContexts.append(NSApp.currentEvent === event)
        super.scrollWheel(with: event)
    }
}
// AppKit exposes no public magnification-event constructor. This fixture enters
// the public NSView magnify event path; it is not physical trackpad proof.
private final class ZoomMagnificationEvent: NSEvent, @unchecked Sendable {
    var amount: CGFloat = 0
    var gesturePhase: NSEvent.Phase = []
    var point = NSPoint.zero
    weak var hostWindow: NSWindow?
    var hostNumber = 0
    override var type: NSEvent.EventType { .magnify }
    override var magnification: CGFloat { amount }
    override var phase: NSEvent.Phase { gesturePhase }
    override var locationInWindow: NSPoint { point }
    override var window: NSWindow? { hostWindow }
    override var windowNumber: Int { hostNumber }
    override var modifierFlags: NSEvent.ModifierFlags { [] }
}

@MainActor final class ZoomTests: XCTestCase {
    private struct Fixture {
        let window: NSWindow
        let host: ZoomHostView
        let view: Pele.DiagramView
        let sibling: NSView
        let mounted: Pele.Mounted
        let resources: ZoomResourceCounter
    }
    private let source = "info title Native zoom"
    private var eventSequence = 0
    private func activate(_ window: NSWindow) {
        let app = NSApplication.shared
        if app.activationPolicy() == .prohibited { XCTAssertTrue(app.setActivationPolicy(.accessory)) }
        app.activate(ignoringOtherApps: true); window.makeKeyAndOrderFront(nil)
        let deadline = Date(timeIntervalSinceNow: 1)
        while (!app.isActive || !window.isKeyWindow) && Date() < deadline {
            RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.01))
        }
        print("Zoom event fixture: active=\(app.isActive), key=\(window.isKeyWindow), visible=\(window.isVisible)")
        XCTAssertTrue(app.isActive); XCTAssertTrue(window.isKeyWindow)
    }
    private func stripedImage(_ size: NSSize) -> NSImage {
        let image = NSImage(size: size)
        image.lockFocus()
        for index in 0..<20 {
            NSColor(deviceRed: CGFloat(index + 1) / 21, green: 0.3, blue: CGFloat(20 - index) / 21, alpha: 1).setFill()
            NSBezierPath(rect: NSRect(x: CGFloat(index) * size.width / 20, y: 0, width: size.width / 20 + 0.1, height: size.height)).fill()
        }
        image.unlockFocus()
        return image
    }
    private func options(_ resources: ZoomResourceCounter, size: NSSize = NSSize(width: 1000, height: 500), zoom: Pele.ZoomPolicy = .automatic) -> Pele.MountOptions {
        var options = Pele.MountOptions()
        options.render.maxWidth = 0
        options.zoom = zoom
        options.resources.textWidth = { resources.width($0) }
        options.preparePresentation = { _ in self.stripedImage(size) }
        return options
    }
    private func fixture(width: CGFloat = 400, height: CGFloat = 240, natural: NSSize = NSSize(width: 1000, height: 500), zoom: Pele.ZoomPolicy = .automatic) throws -> Fixture {
        _ = NSApplication.shared
        let host = ZoomHostView(frame: NSRect(x: 0, y: 0, width: width + 100, height: height + 40))
        let view = Pele.DiagramView(frame: NSRect(x: 10, y: 10, width: width, height: height))
        let sibling = NSView(frame: NSRect(x: width + 20, y: 20, width: 50, height: 50))
        host.addSubview(view); host.addSubview(sibling)
        let window = ZoomFeatureWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false; window.contentView = host; activate(window)
        let resources = ZoomResourceCounter()
        let mounted = try Pele.mount(view, source: source, options: options(resources, size: natural, zoom: zoom))
        window.displayIfNeeded()
        XCTAssertTrue(window.isVisible); XCTAssertTrue(view.window === window)
        return Fixture(window: window, host: host, view: view, sibling: sibling, mounted: mounted, resources: resources)
    }
    private func bitmap(_ view: Pele.DiagramView) throws -> NSBitmapImageRep {
        XCTAssertTrue(try XCTUnwrap(view.window).isVisible)
        view.window?.displayIfNeeded(); view.displayIfNeeded()
        let rep = try XCTUnwrap(view.bitmapImageRepForCachingDisplay(in: view.bounds))
        view.cacheDisplay(in: view.bounds, to: rep)
        XCTAssertGreaterThan(rep.pixelsWide, 0); XCTAssertGreaterThan(rep.pixelsHigh, 0)
        return rep
    }
    private func pixels(_ view: Pele.DiagramView) throws -> Data {
        let rep = try bitmap(view)
        return Data(bytes: try XCTUnwrap(rep.bitmapData), count: rep.bytesPerRow * rep.pixelsHigh)
    }
    private func color(_ view: Pele.DiagramView, at point: NSPoint) throws -> [CGFloat] {
        let rep = try bitmap(view)
        let x = min(rep.pixelsWide - 1, max(0, Int(point.x / view.bounds.width * CGFloat(rep.pixelsWide))))
        let y = min(rep.pixelsHigh - 1, max(0, Int(point.y / view.bounds.height * CGFloat(rep.pixelsHigh))))
        let color = try XCTUnwrap(rep.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB))
        return [color.redComponent, color.greenComponent, color.blueComponent, color.alphaComponent]
    }
    private func sameColor(_ actual: [CGFloat], _ expected: [CGFloat], file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertEqual(actual.count, expected.count, file: file, line: line)
        for (a, b) in zip(actual, expected) { XCTAssertEqual(a, b, accuracy: 0.025, file: file, line: line) }
    }
    private func buttons(_ view: NSView) -> [NSButton] {
        view.subviews.flatMap { child in (child as? NSButton).map { [$0] } ?? buttons(child) }
    }
    private func mouse(_ type: NSEvent.EventType, view: Pele.DiagramView, point: NSPoint, clicks: Int = 1) throws -> NSEvent {
        let window = try XCTUnwrap(view.window)
        eventSequence += 1
        return try XCTUnwrap(NSEvent.mouseEvent(with: type, location: view.convert(point, to: nil), modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber, context: nil, eventNumber: eventSequence, clickCount: clicks, pressure: type == .leftMouseUp ? 0 : 1))
    }
    private func key(_ characters: String, code: UInt16, view: Pele.DiagramView, flags: NSEvent.ModifierFlags = []) throws -> NSEvent {
        let window = try XCTUnwrap(view.window)
        return try XCTUnwrap(NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber, context: nil, characters: characters, charactersIgnoringModifiers: characters, isARepeat: false, keyCode: code))
    }
    private func wheel(_ view: Pele.DiagramView, at point: NSPoint, amount: Int32, precise: Bool, modified: Bool = true) throws -> NSEvent {
        let window = try XCTUnwrap(view.window), windowPoint = view.convert(point, to: nil)
        let screen = window.convertPoint(toScreen: windowPoint), screenHeight = NSScreen.screens.first?.frame.height ?? 0
        let cg = try XCTUnwrap(CGEvent(scrollWheelEvent2Source: nil, units: precise ? .pixel : .line, wheelCount: 1, wheel1: amount, wheel2: 0, wheel3: 0))
        cg.flags = modified ? .maskControl : []
        cg.location = CGPoint(x: screen.x, y: screenHeight - screen.y)
        cg.setIntegerValueField(.mouseEventWindowUnderMousePointer, value: Int64(window.windowNumber))
        cg.setIntegerValueField(.mouseEventWindowUnderMousePointerThatCanHandleThisEvent, value: Int64(window.windowNumber))
        var event = try XCTUnwrap(NSEvent(cgEvent: cg))
        if event.window !== window {
            cg.location = CGPoint(x: windowPoint.x, y: screenHeight - windowPoint.y)
            event = try XCTUnwrap(NSEvent(cgEvent: cg))
        }
        let delivered = view.convert(event.locationInWindow, from: nil)
        XCTAssertEqual(delivered.x, point.x, accuracy: 0.5); XCTAssertEqual(delivered.y, point.y, accuracy: 0.5)
        XCTAssertEqual(event.hasPreciseScrollingDeltas, precise)
        return event
    }

    func testFitClampsRepeatedProgrammaticAnchorsAndStableLayout() throws {
        var camera = Pele.ZoomOptions(); camera.controls = false
        let f = try fixture(zoom: .custom(camera)); defer { f.mounted.destroy(); f.window.close() }
        let zoom = try XCTUnwrap(f.mounted.zoom), frame = f.view.frame, sibling = f.sibling.frame
        let drawing = f.mounted.result?.svg, callbacks = f.resources.value, original = try pixels(f.view)
        XCTAssertEqual(zoom.fit, 0.4, accuracy: 1e-12); XCTAssertEqual(zoom.scale, zoom.fit)
        zoom.zoomTo(0.1); XCTAssertEqual(zoom.scale, zoom.fit)
        let anchor = NSPoint(x: 200, y: 90), initialColor = try color(f.view, at: anchor)
        zoom.zoomTo(0.8); sameColor(try color(f.view, at: anchor), initialColor)
        zoom.zoomTo(1.6); sameColor(try color(f.view, at: anchor), initialColor)
        XCTAssertNotEqual(try pixels(f.view), original)
        zoom.zoomTo(.infinity); XCTAssertEqual(zoom.scale, 3)
        zoom.zoomBy(.nan); XCTAssertEqual(zoom.scale, 3)
        zoom.zoomTo(-.infinity); XCTAssertEqual(zoom.scale, zoom.fit)
        zoom.zoomBy(.infinity); XCTAssertEqual(zoom.scale, 3)
        zoom.reset(); XCTAssertEqual(zoom.scale, zoom.fit)
        XCTAssertEqual(try pixels(f.view), original)
        XCTAssertEqual(f.resources.value, callbacks); XCTAssertEqual(f.mounted.result?.svg, drawing)
        XCTAssertEqual(f.view.frame, frame); XCTAssertEqual(f.sibling.frame, sibling)
        zoom.zoomTo(1.2)
        f.view.setFrameSize(NSSize(width: 300, height: 240)); f.view.layout()
        XCTAssertEqual(zoom.fit, 0.3, accuracy: 1e-12); XCTAssertEqual(zoom.scale, zoom.fit)
        XCTAssertEqual(f.sibling.frame, sibling); XCTAssertEqual(f.mounted.result?.svg, drawing)
        XCTAssertEqual(f.resources.value, callbacks)
    }

    func testModifiedWheelAnchorsLineAndPreciseEventsAndForwardsParentScroll() throws {
        _ = NSApplication.shared
        let scroll = ZoomScrollHost(frame: NSRect(x: 0, y: 0, width: 400, height: 240))
        scroll.hasVerticalScroller = true
        let view = Pele.DiagramView(frame: NSRect(x: 0, y: 0, width: 380, height: 1200))
        scroll.documentView = view
        let window = ZoomFeatureWindow(contentRect: scroll.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false; window.contentView = scroll; activate(window)
        let resources = ZoomResourceCounter()
        var camera = Pele.ZoomOptions(); camera.controls = false
        let mounted = try Pele.mount(view, source: source, options: options(resources, zoom: .custom(camera)))
        defer { mounted.destroy(); window.close() }
        let zoom = try XCTUnwrap(mounted.zoom), count = resources.value, anchor = NSPoint(x: 170, y: 80)
        let original = try color(view, at: anchor)
        for precise in [false, true] {
            let event = try wheel(view, at: anchor, amount: 8, precise: precise)
            let before = zoom.scale, turn = min(max(Double(event.scrollingDeltaY), -24), 24)
            view.scrollWheel(with: event)
            XCTAssertEqual(zoom.scale, min(3, before * pow(2, turn * 0.02)), accuracy: 1e-10)
            sameColor(try color(view, at: anchor), original)
        }
        XCTAssertTrue(scroll.received.isEmpty, "Modified events belong to the camera")
        // Mounting invalidates intrinsic size. Settle the stock scroll view's
        // pending document/scroller layout before measuring native scrolling.
        scroll.tile(); scroll.layoutSubtreeIfNeeded(); window.displayIfNeeded()
        XCTAssertGreaterThan(scroll.contentView.documentRect.height, scroll.contentView.bounds.height)
        print("Zoom scroll readiness: documentRect=\(scroll.contentView.documentRect), clip=\(scroll.contentView.bounds), scrollerEnabled=\(scroll.verticalScroller?.isEnabled ?? false)")
        let previous = scroll.contentView.bounds.origin.y
        let ordinary = try wheel(view, at: anchor, amount: -3, precise: false, modified: false), scale = zoom.scale
        view.scrollWheel(with: ordinary)
        XCTAssertEqual(scroll.received, [ordinary.scrollingDeltaY]); XCTAssertEqual(zoom.scale, scale)
        XCTAssertTrue(scroll.events.last === ordinary)
        let scrollDeadline = Date(timeIntervalSinceNow: 0.5)
        while scroll.contentView.bounds.origin.y <= previous && Date() < scrollDeadline {
            RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.01))
        }
        let deliveredY = scroll.contentView.bounds.origin.y
        print("Zoom parent scroll: eventWindow=\(ordinary.window === window), delta=\(ordinary.deltaY), scrolling=\(ordinary.scrollingDeltaY), phase=\(ordinary.phase.rawValue), document=\(view.frame), clip=\(scroll.contentView.bounds), previousY=\(previous), contexts=\(scroll.currentContexts)")
        if deliveredY <= previous {
            for route in ["parent", "window", "application"] {
                scroll.contentView.scroll(to: .zero); scroll.reflectScrolledClipView(scroll.contentView)
                let baseline = try wheel(view, at: anchor, amount: -3, precise: false, modified: false)
                let count = scroll.events.count
                if route == "parent" { scroll.scrollWheel(with: baseline) }
                else if route == "window" { window.sendEvent(baseline) }
                else { NSApp.sendEvent(baseline) }
                let deadline = Date(timeIntervalSinceNow: 0.5)
                while scroll.contentView.bounds.origin.y <= previous && Date() < deadline {
                    RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.01))
                }
                print("Zoom scroll baseline: route=\(route), delivered=\(scroll.events.count-count), exact=\(scroll.events.last === baseline), current=\(scroll.currentContexts.last ?? false), y=\(scroll.contentView.bounds.origin.y)")
            }
        }
        XCTAssertGreaterThan(deliveredY, previous)
        XCTAssertEqual(resources.value, count)
    }

    func testLocalizedRealButtonsUseControlStepAndRespectDisabledStates() throws {
        var camera = Pele.ZoomOptions(); camera.labels = .init(zoomIn: "Enlarge locally", zoomOut: "Shrink locally", reset: "")
        let f = try fixture(zoom: .custom(camera)); defer { f.mounted.destroy(); f.window.close() }
        let zoom = try XCTUnwrap(f.mounted.zoom), count = f.resources.value
        let all = buttons(f.view)
        XCTAssertEqual(all.count, 3)
        let plus = try XCTUnwrap(all.first { $0.toolTip == "Enlarge locally" })
        let minus = try XCTUnwrap(all.first { $0.toolTip == "Shrink locally" })
        let reset = try XCTUnwrap(all.first { $0.toolTip == "" })
        XCTAssertEqual(plus.accessibilityLabel(), "Enlarge locally"); XCTAssertEqual(minus.accessibilityLabel(), "Shrink locally"); XCTAssertEqual(reset.accessibilityLabel(), "")
        XCTAssertFalse(minus.isEnabled); XCTAssertTrue(reset.isHidden)
        XCTAssertLessThan(plus.convert(plus.bounds, to: f.view).minY, minus.convert(minus.bounds, to: f.view).minY)
        let image = f.view.presentation, before = try pixels(f.view)
        plus.performClick(nil); XCTAssertEqual(zoom.scale, zoom.fit * 1.5625, accuracy: 1e-12)
        XCTAssertTrue(minus.isEnabled); XCTAssertFalse(reset.isHidden)
        XCTAssertNotEqual(try pixels(f.view), before)
        zoom.zoomTo(3); XCTAssertFalse(plus.isEnabled)
        minus.performClick(nil); XCTAssertEqual(zoom.scale, 3 / 1.5625, accuracy: 1e-12)
        reset.performClick(nil); XCTAssertEqual(zoom.scale, zoom.fit); XCTAssertFalse(minus.isEnabled)
        XCTAssertTrue(f.view.presentation === image); XCTAssertEqual(f.resources.value, count)
    }

    func testFocusTraversalKeyboardForwardingAndControlsFreeFocusRelease() throws {
        var camera = Pele.ZoomOptions(); camera.controls = false
        let f = try fixture(zoom: .custom(camera)); defer { f.mounted.destroy(); f.window.close() }
        let zoom = try XCTUnwrap(f.mounted.zoom), count = f.resources.value
        f.host.nextKeyView = f.view; f.view.nextKeyView = f.host
        XCTAssertTrue(f.window.makeFirstResponder(f.host)); f.window.selectNextKeyView(f.host)
        XCTAssertTrue(f.window.firstResponder === f.view)
        f.window.sendEvent(try key("=", code: 24, view: f.view)); XCTAssertEqual(zoom.scale, 0.5, accuracy: 1e-12)
        f.window.sendEvent(try key("+", code: 24, view: f.view, flags: .shift)); XCTAssertEqual(zoom.scale, 0.625, accuracy: 1e-12)
        let beforePan = try pixels(f.view)
        f.window.sendEvent(try key("\u{f703}", code: 124, view: f.view)); XCTAssertNotEqual(try pixels(f.view), beforePan)
        let current = zoom.scale
        f.view.keyDown(with: try key("q", code: 12, view: f.view))
        f.view.keyDown(with: try key("=", code: 24, view: f.view, flags: .command))
        XCTAssertEqual(f.host.forwardedKeys.suffix(2), ["q", "="]); XCTAssertEqual(zoom.scale, current)
        f.window.sendEvent(try key("0", code: 29, view: f.view)); XCTAssertEqual(zoom.scale, zoom.fit)
        let keyboardRing = try pixels(f.view)
        f.view.mouseDown(with: try mouse(.leftMouseDown, view: f.view, point: NSPoint(x: 100, y: 80)))
        f.view.mouseUp(with: try mouse(.leftMouseUp, view: f.view, point: NSPoint(x: 100, y: 80)))
        XCTAssertNotEqual(try pixels(f.view), keyboardRing, "Keyboard focus ring must differ from pointer-origin focus at the same resting camera")
        f.view.keyDown(with: try key("0", code: 29, view: f.view)); f.view.keyDown(with: try key("\u{1b}", code: 53, view: f.view))
        XCTAssertEqual(f.host.forwardedKeys.suffix(2), ["0", "\u{1b}"])
        XCTAssertTrue(f.window.makeFirstResponder(f.host))
        f.view.keyDown(with: try key("=", code: 24, view: f.view)); XCTAssertEqual(zoom.scale, zoom.fit)
        XCTAssertTrue(f.window.makeFirstResponder(f.view))
        f.view.setFrameSize(NSSize(width: 1200, height: 240)); f.view.layout()
        XCTAssertFalse(f.view.acceptsFirstResponder); XCTAssertFalse(f.window.firstResponder === f.view)
        XCTAssertTrue(f.window.makeFirstResponder(f.host)); zoom.refresh(); XCTAssertTrue(f.window.firstResponder === f.host)
        XCTAssertTrue(buttons(f.view).isEmpty); XCTAssertEqual(f.resources.value, count)
    }

    func testActualMousePanDoubleClickAndReleaseOutsideKeepOneDrawing() throws {
        let f = try fixture(); defer { f.mounted.destroy(); f.window.close() }
        let zoom = try XCTUnwrap(f.mounted.zoom), count = f.resources.value, image = f.view.presentation
        let initial = try pixels(f.view)
        f.window.sendEvent(try mouse(.leftMouseDown, view: f.view, point: NSPoint(x: 160, y: 80), clicks: 2))
        f.window.sendEvent(try mouse(.leftMouseUp, view: f.view, point: NSPoint(x: 160, y: 80), clicks: 2))
        XCTAssertEqual(zoom.scale, 1); XCTAssertNotEqual(try pixels(f.view), initial)
        f.window.sendEvent(try mouse(.leftMouseDown, view: f.view, point: NSPoint(x: 200, y: 80)))
        let before = try pixels(f.view)
        f.window.sendEvent(try mouse(.leftMouseDragged, view: f.view, point: NSPoint(x: 160, y: 80)))
        XCTAssertNotEqual(try pixels(f.view), before)
        f.window.sendEvent(try mouse(.leftMouseUp, view: f.view, point: NSPoint(x: -50, y: -50)))
        let released = try pixels(f.view)
        f.view.mouseDragged(with: try mouse(.leftMouseDragged, view: f.view, point: NSPoint(x: 250, y: 80)))
        XCTAssertEqual(try pixels(f.view), released, "Release outside must end AppKit drag ownership")
        f.window.sendEvent(try mouse(.leftMouseDown, view: f.view, point: NSPoint(x: 160, y: 80), clicks: 2))
        f.window.sendEvent(try mouse(.leftMouseUp, view: f.view, point: NSPoint(x: 160, y: 80), clicks: 2))
        XCTAssertEqual(zoom.scale, zoom.fit)
        f.window.sendEvent(try mouse(.rightMouseDown, view: f.view, point: NSPoint(x: 150, y: 80)))
        f.window.sendEvent(try mouse(.rightMouseUp, view: f.view, point: NSPoint(x: 150, y: 80)))
        XCTAssertEqual(zoom.scale, zoom.fit)
        XCTAssertTrue(f.view.presentation === image); XCTAssertEqual(f.resources.value, count)
    }

    func testNativeMagnificationEntryCancellationAndWindowFocusLossEndCapture() throws {
        let f = try fixture(); defer { f.mounted.destroy(); f.window.close() }
        let zoom = try XCTUnwrap(f.mounted.zoom), count = f.resources.value, anchor = NSPoint(x: 170, y: 80)
        let original = try color(f.view, at: anchor)
        let event = ZoomMagnificationEvent(); event.hostWindow = f.window; event.hostNumber = f.window.windowNumber; event.point = f.view.convert(anchor, to: nil)
        event.amount = 0.25; event.gesturePhase = .began; f.view.magnify(with: event)
        XCTAssertEqual(zoom.scale, 0.5, accuracy: 1e-12)
        event.amount = 0.2; event.gesturePhase = .changed; f.view.magnify(with: event)
        XCTAssertEqual(zoom.scale, 0.6, accuracy: 1e-12); sameColor(try color(f.view, at: anchor), original)
        event.amount = 0.9; event.gesturePhase = .cancelled; f.view.magnify(with: event)
        XCTAssertEqual(zoom.scale, 0.6, accuracy: 1e-12)
        f.view.mouseDown(with: try mouse(.leftMouseDown, view: f.view, point: anchor))
        f.window.resignKey()
        let before = try pixels(f.view)
        f.view.mouseDragged(with: try mouse(.leftMouseDragged, view: f.view, point: NSPoint(x: 240, y: 80)))
        XCTAssertEqual(try pixels(f.view), before)
        f.window.makeKeyAndOrderFront(nil)
        f.view.mouseDown(with: try mouse(.leftMouseDown, view: f.view, point: anchor))
        XCTAssertEqual(f.resources.value, count, "Camera input and focus cancellation must not render the engine")
        f.view.removeFromSuperview(); f.host.addSubview(f.view)
        // Reattachment may refresh host font resources; subsequent stale drag
        // input must neither move its camera nor cause another engine render.
        let detached = try pixels(f.view), reattachedCount = f.resources.value
        f.view.mouseDragged(with: try mouse(.leftMouseDragged, view: f.view, point: NSPoint(x: 250, y: 80)))
        XCTAssertEqual(try pixels(f.view), detached)
        XCTAssertEqual(f.resources.value, reattachedCount)
    }

    func testRoomyViewportAllowsNegativeOriginAndClampsWholeImageInsidePanel() throws {
        let f = try fixture(natural: NSSize(width: 160, height: 80), zoom: .always)
        defer { f.mounted.destroy(); f.window.close() }
        let zoom = try XCTUnwrap(f.mounted.zoom), count = f.resources.value, frame = f.view.frame
        XCTAssertEqual(zoom.fit, 1); zoom.zoomTo(2)
        let before = try pixels(f.view), opaque = try color(f.view, at: NSPoint(x: 10, y: 60))
        XCTAssertGreaterThan(opaque[3], 0.9)
        f.view.mouseDown(with: try mouse(.leftMouseDown, view: f.view, point: NSPoint(x: 80, y: 40)))
        f.view.mouseDragged(with: try mouse(.leftMouseDragged, view: f.view, point: NSPoint(x: 160, y: 80)))
        let shifted = try pixels(f.view)
        XCTAssertNotEqual(shifted, before)
        XCTAssertLessThan(try color(f.view, at: NSPoint(x: 10, y: 60))[3], 0.1)
        XCTAssertGreaterThan(try color(f.view, at: NSPoint(x: 90, y: 60))[3], 0.9)
        f.view.mouseDragged(with: try mouse(.leftMouseDragged, view: f.view, point: NSPoint(x: 1000, y: 80)))
        XCTAssertEqual(try pixels(f.view), shifted, "Camera must clamp the negative origin at the roomy panel edge")
        f.view.mouseUp(with: try mouse(.leftMouseUp, view: f.view, point: NSPoint(x: 1000, y: 80)))
        XCTAssertEqual(f.view.frame, frame); XCTAssertEqual(f.resources.value, count)
    }

    func testMountPoliciesCallbacksQuietRefreshAndLastGoodZoomedErrors() throws {
        let f = try fixture(zoom: .disabled); defer { f.mounted.destroy(); f.window.close() }
        XCTAssertNil(f.mounted.zoom); XCTAssertFalse(f.view.acceptsFirstResponder)
        var next = options(f.resources, zoom: .always), seen = [Pele.Zoom?]()
        next.onRender = { _ in seen.append(f.mounted.zoom) }
        try f.mounted.update("info title Zoom replacement", options: next)
        XCTAssertEqual(seen.count, 1); XCTAssertNil(seen[0])
        let first = try XCTUnwrap(f.mounted.zoom); first.zoomTo(1.2)
        let count = f.resources.value, drawing = f.mounted.result?.svg
        try f.mounted.refresh(); XCTAssertEqual(first.scale, 1.2); XCTAssertEqual(f.resources.value, count)
        var custom = Pele.ZoomOptions(); custom.maxScale = 2
        next.zoom = .custom(custom)
        try f.mounted.update(source, options: next)
        XCTAssertTrue(seen.last! === first); XCTAssertEqual(first.scale, first.fit)
        let second = try XCTUnwrap(f.mounted.zoom); XCTAssertFalse(second === first)
        try f.mounted.update(source, options: next); XCTAssertTrue(f.mounted.zoom === second)
        second.zoomTo(1.2)
        let good = f.view.presentation, goodSVG = f.mounted.result?.svg, before = try pixels(f.view)
        try f.mounted.update("not a valid diagram", options: next)
        XCTAssertTrue(f.mounted.zoom === second); XCTAssertEqual(second.scale, 1.2)
        XCTAssertTrue(f.view.presentation === good); XCTAssertEqual(f.mounted.result?.svg, goodSVG)
        XCTAssertNotNil(f.view.diagnostic); XCTAssertNotEqual(try pixels(f.view), before)
        let calls = f.resources.value
        try XCTUnwrap(buttons(f.view).first { $0.toolTip == "Zoom in" }).performClick(nil)
        XCTAssertGreaterThan(second.scale, 1.2); XCTAssertEqual(f.resources.value, calls)
        var recover = options(f.resources, zoom: .disabled)
        recover.onRender = { _ in }
        next.onRender = { _ in try f.mounted.update(self.source, options: recover) }
        try f.mounted.update("info title Reentrant policy", options: next)
        XCTAssertNil(f.mounted.zoom); XCTAssertFalse(f.view.acceptsFirstResponder)
        XCTAssertNotNil(drawing)
    }

    func testStandaloneReplacementDestroyRetainedMethodsResizeAndWindowIsolation() throws {
        let f = try fixture(), other = try fixture()
        defer { f.mounted.destroy(); other.mounted.destroy(); f.window.close(); other.window.close() }
        let original = try XCTUnwrap(f.mounted.zoom), isolated = try XCTUnwrap(other.mounted.zoom)
        var settings = Pele.ZoomOptions(); settings.controls = false
        let zoom = try Pele.enableZoom(f.view, options: settings)
        zoom.zoomTo(1); XCTAssertEqual(original.scale, original.fit); XCTAssertEqual(isolated.scale, isolated.fit)
        f.view.mouseDown(with: try mouse(.leftMouseDown, view: f.view, point: NSPoint(x: 150, y: 80)))
        zoom.destroy(); zoom.destroy(); XCTAssertFalse(f.view.acceptsFirstResponder)
        let rest = try pixels(f.view), callbacks = f.resources.value
        zoom.zoomBy(2); XCTAssertEqual(zoom.scale, 0.8, accuracy: 1e-12); XCTAssertNotEqual(try pixels(f.view), rest)
        f.view.mouseDragged(with: try mouse(.leftMouseDragged, view: f.view, point: NSPoint(x: 240, y: 80)))
        XCTAssertEqual(zoom.scale, 0.8, accuracy: 1e-12)
        f.view.setFrameSize(NSSize(width: 300, height: 240)); f.view.layout()
        XCTAssertEqual(zoom.fit, 0.4, accuracy: 1e-12, "Destroyed controller must not resume automatic resize observation")
        zoom.refresh(); XCTAssertEqual(zoom.fit, 0.3, accuracy: 1e-12)
        let replacement = try Pele.enableZoom(f.view); defer { replacement.destroy() }
        replacement.zoomTo(1)
        original.destroy(); zoom.destroy(); XCTAssertTrue(f.view.acceptsFirstResponder)
        XCTAssertTrue(f.window.makeFirstResponder(f.view))
        f.view.keyDown(with: try key("=", code: 24, view: f.view))
        XCTAssertEqual(replacement.scale, 1.25, accuracy: 1e-12); XCTAssertEqual(zoom.scale, zoom.fit)
        XCTAssertEqual(isolated.scale, isolated.fit); XCTAssertEqual(f.resources.value, callbacks)
        var bad = Pele.ZoomOptions(); bad.maxScale = .infinity
        XCTAssertThrowsError(try Pele.enableZoom(f.view, options: bad)) { XCTAssertEqual(($0 as? PeleError)?.code, "presentation") }
        var finite = Pele.ZoomOptions(); finite.maxScale = -2
        let minimum = try Pele.enableZoom(f.view, options: finite); defer { minimum.destroy() }
        minimum.zoomTo(.infinity); XCTAssertEqual(minimum.scale, minimum.fit)
    }
}
