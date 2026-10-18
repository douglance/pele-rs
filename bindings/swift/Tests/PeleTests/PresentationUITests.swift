import AppKit
import XCTest
import Pele

private final class NativeLinkFailure: Error {}
private final class NativeLinkModel {}
private final class NativeLinkResources: @unchecked Sendable {
    private let lock = NSLock(); private var calls = 0; private var failed = false
    let error = NativeLinkFailure()
    func fail(_ value: Bool) { lock.lock(); failed = value; lock.unlock() }
    var count: Int { lock.lock(); defer { lock.unlock() }; return calls }
    func width(_ request: Pele.TextMeasurement) throws -> Double {
        lock.lock(); calls += 1; let failed = failed; lock.unlock()
        if failed { throw error }
        return Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6)
    }
}
@MainActor final class PresentationUITests: XCTestCase {
    private let source = "flowchart LR\naccTitle: Native orders\naccDescr: Accessible order flow\nA[Start] --> B[Local note] --> C[Finish]\nclick A \"https://example.test/start\"\nclass B internal-link\nclick C \"javascript:alert(1)\""
    private var sequence = 0
    private func window(width: CGFloat = 440) -> (NSWindow,Pele.DiagramView) {
        let app = NSApplication.shared
        let view = Pele.DiagramView(frame: NSRect(x: 0,y: 0,width: width,height: 280))
        let window = NSWindow(contentRect: view.frame,styleMask: [.titled],backing: .buffered,defer: false)
        window.isReleasedWhenClosed = false; window.contentView = view
        window.makeKeyAndOrderFront(nil); app.activate(ignoringOtherApps: true)
        let deadline = Date(timeIntervalSinceNow: 1)
        while (!app.isActive || !window.isKeyWindow) && Date() < deadline { RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.01)) }
        XCTAssertTrue(app.isActive); XCTAssertTrue(window.isKeyWindow); XCTAssertTrue(window.isVisible)
        return (window,view)
    }
    private func options(_ host: NativeLinkResources) -> Pele.MountOptions {
        var options = Pele.MountOptions(); options.zoom = .disabled
        options.resources.textWidth = { try host.width($0) }; return options
    }
    private func artifact(_ view: Pele.DiagramView) throws -> Pele.PresentationArtifact {
        guard case .available(let artifact) = view.nativePresentation else { XCTFail("Expected real Rust artifact"); throw NativeLinkFailure() }
        return artifact
    }
    private func children(_ view: Pele.DiagramView) -> [NSAccessibilityElement] { view.accessibilityChildren()?.compactMap { $0 as? NSAccessibilityElement } ?? [] }
    private func accessibleButtonLabels(_ values: [Any]) -> [String] {
        values.flatMap { value in
            if let button = value as? NSButton { return button.isHiddenOrHasHiddenAncestor ? [] : [button.accessibilityLabel()].compactMap { $0 } }
            if let cell = value as? NSCell {
                guard cell.controlView?.isHiddenOrHasHiddenAncestor != true, cell.accessibilityRole() == .button else { return [] }
                return [cell.accessibilityLabel()].compactMap { $0 }
            }
            if let view = value as? NSView { return accessibleButtonLabels(view.accessibilityChildren() ?? []) }
            if let element = value as? NSAccessibilityElement { return accessibleButtonLabels(element.accessibilityChildren() ?? []) }
            print("Unknown native AX child type: \(type(of: value))")
            return []
        }
    }
    private func rect(_ element: NSAccessibilityElement, view: Pele.DiagramView) throws -> NSRect {
        let window = try XCTUnwrap(view.window); return view.convert(window.convertFromScreen(element.accessibilityFrame()),from: nil)
    }
    private func mouse(_ type: NSEvent.EventType, view: Pele.DiagramView, point: NSPoint, clicks: Int = 1) throws -> NSEvent {
        let window = try XCTUnwrap(view.window); sequence += 1
        return try XCTUnwrap(NSEvent.mouseEvent(with: type,location: view.convert(point,to: nil),modifierFlags: [],timestamp: ProcessInfo.processInfo.systemUptime,windowNumber: window.windowNumber,context: nil,eventNumber: sequence,clickCount: clicks,pressure: type == .leftMouseUp ? 0 : 1))
    }
    private func click(_ element: NSAccessibilityElement, view: Pele.DiagramView, clicks: Int = 1) throws {
        let bounds = try rect(element,view: view); XCTAssertFalse(bounds.isEmpty)
        let point = NSPoint(x: bounds.midX,y: bounds.midY), window = try XCTUnwrap(view.window)
        window.sendEvent(try mouse(.leftMouseDown,view: view,point: point,clicks: clicks))
        window.sendEvent(try mouse(.leftMouseUp,view: view,point: point,clicks: clicks))
    }
    private func painted(_ view: Pele.DiagramView) throws -> Data {
        view.window?.displayIfNeeded(); view.displayIfNeeded()
        let bitmap = try XCTUnwrap(view.bitmapImageRepForCachingDisplay(in: view.bounds)); view.cacheDisplay(in: view.bounds,to: bitmap)
        let data = Data(bytes: try XCTUnwrap(bitmap.bitmapData),count: bitmap.bytesPerRow * bitmap.pixelsHigh)
        XCTAssertGreaterThan(Set(data).count,2,"Actual native window must paint nonblank drawing pixels")
        return data
    }
    func testRustAccessibilityAndHostOnlyMouseAndAccessibilityActionsInActualWindow() throws {
        let (window,view) = window(); defer { window.close() }
        let host = NativeLinkResources(); var settings = options(host), actions = [Pele.PresentationLink]()
        settings.onLink = { actions.append($0) }
        let mounted = try Pele.mount(view,source: source,options: settings)
        let frame = try artifact(view), links = children(view), result = try XCTUnwrap(mounted.result)
        XCTAssertEqual(frame.title,"Native orders"); XCTAssertTrue(view.presentationGeometryAvailable)
        XCTAssertEqual(view.accessibilityLabel(),"Native orders"); XCTAssertEqual(view.accessibilityHelp(),"Accessible order flow")
        XCTAssertTrue((view.accessibilityValue() as? String)?.contains("Start") == true)
        XCTAssertEqual(links.count,2); XCTAssertEqual(frame.links.count,2)
        XCTAssertEqual(links.map { $0.accessibilityRole() },[.link,.link])
        XCTAssertEqual(links.map { $0.accessibilityLabel() },["Start","Local note"])
        let ratio = min(Double(view.bounds.width),result.width) / result.width
        for (index,link) in frame.links.enumerated() {
            let actual = try rect(links[index],view: view)
            XCTAssertEqual(actual.minX,link.bounds.x * ratio,accuracy: 0.01)
            XCTAssertEqual(actual.minY,link.bounds.y * ratio,accuracy: 0.01)
            XCTAssertEqual(actual.width,link.bounds.width * ratio,accuracy: 0.01)
        }
        let calls = host.count
        try click(links[0],view: view); XCTAssertEqual(actions.map(\.href),["https://example.test/start"])
        XCTAssertTrue(links[1].accessibilityPerformPress()); XCTAssertEqual(actions.last?.href,"Local note"); XCTAssertTrue(actions.last?.internal == true)
        XCTAssertEqual(host.count,calls,"Link actions never reparse or relayout")
        _ = try painted(view)
        var inert = options(host); inert.onLink = nil
        try mounted.update(source,options: inert)
        XCTAssertTrue(children(view).isEmpty,"No default URL opening or implicit activation")
    }
    func testZoomUsesSameCameraForAccessibleRegionsAndSuppressesDragAndDoubleClickActions() throws {
        let (window,view) = window(width: 330); defer { window.close() }
        let host = NativeLinkResources(); var settings = options(host), actions = 0
        settings.zoom = .always; settings.render.maxWidth = 0; settings.onLink = { _ in actions += 1 }
        let long = "flowchart LR\nA[First long task] --> B[Second long task] --> C[Third long task]\nclick A \"https://example.test/a\"\nclick B \"https://example.test/b\"\nclick C \"https://example.test/c\""
        let mounted = try Pele.mount(view,source: long,options: settings), zoom = try XCTUnwrap(mounted.zoom)
        XCTAssertEqual(Set(accessibleButtonLabels(view.accessibilityChildren() ?? [])),Set(["Zoom in","Zoom out"]))
        let middle = try XCTUnwrap(children(view).dropFirst().first), before = try rect(middle,view: view), pixels = try painted(view), calls = host.count
        zoom.zoomTo(1)
        let after = try rect(middle,view: view)
        XCTAssertEqual(Set(accessibleButtonLabels(view.accessibilityChildren() ?? [])),Set(["Zoom in","Zoom out","Reset zoom"]))
        XCTAssertEqual(after.width / before.width,zoom.scale / zoom.fit,accuracy: 0.01)
        XCTAssertNotEqual(try painted(view),pixels); try click(middle,view: view); XCTAssertEqual(actions,1)
        let p = NSPoint(x: after.midX,y: after.midY)
        window.sendEvent(try mouse(.leftMouseDown,view: view,point: p))
        window.sendEvent(try mouse(.leftMouseDragged,view: view,point: NSPoint(x: p.x + 20,y: p.y)))
        window.sendEvent(try mouse(.leftMouseUp,view: view,point: NSPoint(x: p.x + 20,y: p.y)))
        XCTAssertEqual(actions,1,"Pan release must not activate a link")
        try click(middle,view: view,clicks: 2)
        XCTAssertEqual(actions,1,"Double-click zoom event is not another link activation")
        XCTAssertEqual(zoom.scale,zoom.fit,accuracy: 0.00001)
        XCTAssertEqual(host.count,calls); XCTAssertEqual(view.frame.size,NSSize(width: 330,height: 280))
        zoom.destroy()
        XCTAssertTrue(accessibleButtonLabels(view.accessibilityChildren() ?? []).isEmpty)
        XCTAssertEqual(children(view).count,3,"Destroying zoom preserves native link actions")
    }
    func testFailedAndDeferredUpdatesKeepAcceptedImageMetadataAndActionOwnerAtomically() throws {
        let (window,view) = window(); defer { window.close() }
        let host = NativeLinkResources(); var first = options(host), actions = [String]()
        first.onLink = { actions.append("old:" + $0.href) }
        let mounted = try Pele.mount(view,source: source,options: first), oldChild = try XCTUnwrap(children(view).first), oldImage = try XCTUnwrap(view.presentation)
        let oldSVG = mounted.result?.svg, oldPixels = try painted(view)
        var failed = options(host); failed.outputLimit = .number(0); failed.onLink = { actions.append("uncommitted:" + $0.href) }
        try mounted.update(source.replacingOccurrences(of: "Native orders",with: "Rejected title"),options: failed)
        XCTAssertNotNil(mounted.error); XCTAssertEqual(mounted.result?.svg,oldSVG); XCTAssertTrue(view.presentation === oldImage)
        XCTAssertEqual(try artifact(view).title,"Native orders"); XCTAssertTrue(children(view).first === oldChild)
        XCTAssertTrue(oldChild.accessibilityPerformPress()); XCTAssertTrue(actions.last?.hasPrefix("old:") == true)
        XCTAssertNotEqual(try painted(view),oldPixels)
        var recovered = options(host); recovered.onLink = { actions.append("new:" + $0.href) }
        try mounted.update(source.replacingOccurrences(of: "Native orders",with: "Accepted title"),options: recovered)
        XCTAssertNil(mounted.error); XCTAssertEqual(try artifact(view).title,"Accepted title"); XCTAssertFalse(oldChild.accessibilityPerformPress(),"Retained stale accessibility element cannot activate a different drawing")
        view.isHidden = true
        try mounted.update(source,options: first); XCTAssertEqual(try artifact(view).title,"Accepted title")
        view.isHidden = false; try mounted.refresh(); XCTAssertEqual(try artifact(view).title,"Native orders")
        let accepted = try XCTUnwrap(children(view).first), failure = NativeLinkFailure()
        var preparation = options(host); preparation.preparePresentation = { _ in throw failure }
        try mounted.update(source,options: preparation)
        XCTAssertTrue(mounted.error as? NativeLinkFailure === failure); XCTAssertTrue(children(view).first === accepted)
        XCTAssertTrue(accepted.accessibilityPerformPress())
    }
    func testCustomImageMappingRequiresExplicitGeometryDeclaration() throws {
        let (window,view) = window(); defer { window.close() }
        let host = NativeLinkResources(); var settings = options(host), actions = 0
        settings.onLink = { _ in actions += 1 }
        settings.preparePresentation = { drawing in
            let image = try XCTUnwrap(NSImage(data: Data(drawing.svg.utf8)))
            image.size = NSSize(width: drawing.width * 2,height: drawing.height * 2); return image
        }
        let mounted = try Pele.mount(view,source: source,options: settings)
        XCTAssertFalse(view.presentationGeometryAvailable); XCTAssertEqual(view.accessibilityLabel(),"flowchart diagram"); XCTAssertTrue(children(view).isEmpty)
        XCTAssertEqual(try artifact(view).links.count,2,"Raw Rust artifact is retained without claiming custom-image geometry")
        settings.preparedImagePreservesSVGGeometry = true
        try mounted.update(source,options: settings)
        XCTAssertTrue(view.presentationGeometryAvailable); XCTAssertEqual(children(view).count,2)
        try click(children(view)[0],view: view); XCTAssertEqual(actions,1); _ = try painted(view)
    }
    func testUnavailableAndCustomProviderMetadataKeepNativeDrawingWithGenericAccessibility() throws {
        let (window,view) = window(); defer { window.close() }
        let host = NativeLinkResources(); var settings = options(host)
        settings.presentationBudget = .init(maxElements: 0); settings.onLink = { _ in XCTFail("Unavailable metadata has no actions") }
        let mounted = try Pele.mount(view,source: source,options: settings)
        guard case .unavailable(.budgetExceeded) = view.nativePresentation else { return XCTFail("Explicit budget fallback") }
        XCTAssertEqual(view.accessibilityLabel(),"flowchart diagram"); XCTAssertFalse(view.presentationGeometryAvailable); XCTAssertTrue(children(view).isEmpty); _ = try painted(view)
        try mounted.update("pie\n\"A\":1",options: options(host))
        guard case .unavailable(.unsupportedDiagram) = view.nativePresentation else { return XCTFail("Explicit unsupported fallback") }
        XCTAssertEqual(view.accessibilityLabel(),"pie diagram"); _ = try painted(view)
        let registry = try Pele.Registry.empty()
        let custom = Pele.Diagram(type: "pie",parse: { _ in NativeLinkModel() },render: { _,_ in
            .init(svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"80\" height=\"40\"><rect width=\"60\" height=\"30\" fill=\"#247acd\"/></svg>",width: 80,height: 40,links: [.init(id: "fake",href: "https://example.test/provider")])
        })
        try registry.register(custom)
        let provider = try Pele.mount(view,source: "pie",registry: registry,options: options(host))
        XCTAssertEqual(provider.result?.links.count,1); XCTAssertTrue(children(view).isEmpty)
        guard case .unavailable(.unsupportedDiagram) = view.nativePresentation else { return XCTFail("Provider has no Rust link geometry") }
        XCTAssertEqual(view.accessibilityLabel(),"pie diagram"); _ = try painted(view)
    }
    func testDestroyKeyboardHostErrorsAndReplacementRespectCurrentOwner() throws {
        let (window,view) = window(); defer { window.close() }
        let host = NativeLinkResources(), failure = NativeLinkFailure(); var settings = options(host)
        settings.onLink = { _ in throw failure }
        let mounted = try Pele.mount(view,source: source,options: settings)
        let link = try XCTUnwrap(children(view).first)
        XCTAssertFalse(link.accessibilityPerformPress()); XCTAssertTrue(mounted.unhandledCallbackError as? NativeLinkFailure === failure); XCTAssertNil(mounted.error)
        var calls = 0; settings.onLink = { _ in calls += 1 }
        try mounted.update(source,options: settings); window.makeFirstResponder(view)
        let tab = try XCTUnwrap(NSEvent.keyEvent(with: .keyDown,location: .zero,modifierFlags: [],timestamp: ProcessInfo.processInfo.systemUptime,windowNumber: window.windowNumber,context: nil,characters: "\t",charactersIgnoringModifiers: "\t",isARepeat: false,keyCode: 48))
        let enter = try XCTUnwrap(NSEvent.keyEvent(with: .keyDown,location: .zero,modifierFlags: [],timestamp: ProcessInfo.processInfo.systemUptime,windowNumber: window.windowNumber,context: nil,characters: "\r",charactersIgnoringModifiers: "\r",isARepeat: false,keyCode: 36))
        window.sendEvent(tab); window.sendEvent(enter); XCTAssertEqual(calls,1); _ = try painted(view)
        let old = try XCTUnwrap(children(view).first); mounted.destroy(); XCTAssertTrue(children(view).isEmpty); XCTAssertFalse(old.accessibilityPerformPress()); XCTAssertNotNil(view.presentation)
        try mounted.update(source); XCTAssertTrue(children(view).isEmpty,"Explicit post-destroy update does not restore input ownership")
        let replacement = try Pele.mount(view,source: source,options: settings)
        mounted.destroy(); XCTAssertFalse(children(view).isEmpty,"Old controller cannot destroy new action owner")
        XCTAssertTrue(try XCTUnwrap(children(view).first).accessibilityPerformPress()); XCTAssertEqual(calls,2); XCTAssertNotNil(replacement.result)
    }
}

@MainActor extension PresentationUITests {
    func testLinkPressCancelsAcrossActualWindowKeyLossWithZoomDisabledAndDestroyed() throws {
        let (first,view) = window(); defer { first.close() }
        let host = NativeLinkResources(); var settings = options(host), calls = 0
        settings.onLink = { _ in calls += 1 }
        let mounted = try Pele.mount(view,source: source,options: settings)
        let second = NSWindow(contentRect: NSRect(x: 50,y: 50,width: 160,height: 80),styleMask: [.titled],backing: .buffered,defer: false)
        second.isReleasedWhenClosed = false; defer { second.close() }
        for destroyedZoom in [false,true] {
            if destroyedZoom { let zoom = try Pele.enableZoom(view); zoom.destroy() }
            first.makeKeyAndOrderFront(nil)
            let action = try XCTUnwrap(children(view).first), box = try rect(action,view: view), point = NSPoint(x: box.midX,y: box.midY)
            first.sendEvent(try mouse(.leftMouseDown,view: view,point: point))
            second.makeKeyAndOrderFront(nil)
            XCTAssertTrue(second.isKeyWindow); XCTAssertFalse(first.isKeyWindow)
            first.makeKeyAndOrderFront(nil)
            XCTAssertTrue(first.isKeyWindow)
            first.sendEvent(try mouse(.leftMouseUp,view: view,point: point))
            XCTAssertEqual(calls,0,"Resigning key cancels pending link, including after Zoom.destroy")
            try click(action,view: view); XCTAssertEqual(calls,1); calls = 0
        }
        mounted.destroy()
    }
}
