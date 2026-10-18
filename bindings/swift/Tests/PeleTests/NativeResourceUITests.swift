import AppKit
import CoreText
import XCTest
import Pele

private final class ImageApprovalFailure: Error {}
@MainActor final class NativeResourceUITests: XCTestCase {
    private let source = "flowchart LR\nA@{img: \"https://example.test/picture.png\",label: \"Picture\",w:80,h:60} --> B[Done]"
    private func window() -> (NSWindow, Pele.DiagramView) {
        let view = Pele.DiagramView(frame: NSRect(x: 0,y: 0,width: 440,height: 220))
        let window = NSWindow(contentRect: view.frame,styleMask: [.titled],backing: .buffered,defer: false)
        window.isReleasedWhenClosed = false; window.contentView = view; window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
        let deadline = Date(timeIntervalSinceNow: 1)
        while !window.isKeyWindow && Date() < deadline { RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.01)) }
        XCTAssertTrue(window.isKeyWindow); return (window,view)
    }
    private func png(_ color: NSColor, width: Int = 2, height: Int = 2) throws -> Data {
        let bitmap = try XCTUnwrap(NSBitmapImageRep(bitmapDataPlanes: nil,pixelsWide: width,pixelsHigh: height,bitsPerSample: 8,samplesPerPixel: 4,hasAlpha: true,isPlanar: false,colorSpaceName: .deviceRGB,bytesPerRow: 0,bitsPerPixel: 0))
        let rgb = try XCTUnwrap(color.usingColorSpace(.deviceRGB)), bytes = try XCTUnwrap(bitmap.bitmapData)
        let components = [rgb.redComponent,rgb.greenComponent,rgb.blueComponent,rgb.alphaComponent].map { UInt8(clamping: Int(($0 * 255).rounded())) }
        for y in 0..<height { for x in 0..<width { for channel in 0..<4 { bytes[y * bitmap.bytesPerRow + x * 4 + channel] = components[channel] } } }
        let data = try XCTUnwrap(bitmap.representation(using: .png,properties: [:]))
        let decoded = try XCTUnwrap(NSBitmapImageRep(data: data)), sample = try XCTUnwrap(decoded.colorAt(x: 0,y: 0)?.usingColorSpace(.deviceRGB))
        if rgb.redComponent > rgb.blueComponent {
            XCTAssertGreaterThan(sample.redComponent,0.6); XCTAssertGreaterThan(sample.redComponent - max(sample.greenComponent,sample.blueComponent),0.4)
        } else {
            XCTAssertGreaterThan(sample.blueComponent,0.6); XCTAssertGreaterThan(sample.blueComponent - max(sample.redComponent,sample.greenComponent),0.4)
        }
        return data
    }
    private func pixels(_ view: Pele.DiagramView) throws -> (Data, Int, Int) {
        view.window?.displayIfNeeded(); view.displayIfNeeded()
        let bitmap = try XCTUnwrap(view.bitmapImageRepForCachingDisplay(in: view.bounds)); view.cacheDisplay(in: view.bounds,to: bitmap)
        let data = Data(bytes: try XCTUnwrap(bitmap.bitmapData),count: bitmap.bytesPerRow * bitmap.pixelsHigh)
        var red = 0, blue = 0
        for y in 0..<bitmap.pixelsHigh { for x in 0..<bitmap.pixelsWide {
            guard let c = bitmap.colorAt(x: x,y: y)?.usingColorSpace(.deviceRGB) else { continue }
            if c.redComponent > 0.6 && c.redComponent - max(c.greenComponent,c.blueComponent) > 0.4 { red += 1 }
            if c.blueComponent > 0.6 && c.blueComponent - max(c.redComponent,c.greenComponent) > 0.4 { blue += 1 }
        }}
        XCTAssertGreaterThan(Set(data).count,2); return (data,red,blue)
    }
    func testApprovedRasterUsesActualNativePixelsAndAllThreeRetainedRenderLanes() throws {
        let data = try png(.red), engine = Pele.Engine(); var calls = 0
        let owner = Pele.NativeResources { uri in calls += 1; XCTAssertEqual(uri,"https://example.test/picture.png"); return data }
        var settings = Pele.Engine.Options(); settings.nativeTheme = Pele.NativeTheme()
        let direct = try engine.renderNative(source,nativeResources: owner,options: settings)
        let document = try engine.parse(source)
        let retained = try engine.renderNative(document,nativeResources: owner,options: settings)
        let mutable = try engine.parseMutable(source)
        let edited = try mutable.renderNative(nativeResources: owner,options: settings)
        XCTAssertEqual(direct.svg,retained.svg); XCTAssertEqual(direct.svg,edited.svg); XCTAssertEqual(mutable.revision,1)
        XCTAssertTrue(direct.svg.contains("data:image/png;base64,")); XCTAssertFalse(direct.svg.contains("href=\"https://")); XCTAssertEqual(calls,1)
        let (window,view) = window(); defer { window.close() }
        var options = Pele.MountOptions(); options.nativeResources = owner; options.zoom = .disabled
        let mounted = try Pele.mount(view,source: source,options: options); defer { mounted.destroy() }
        XCTAssertNotNil(view.presentation); let painted = try pixels(view); XCTAssertGreaterThan(painted.1,200,"Approved raster must paint real red image pixels")
        XCTAssertEqual(calls,1); XCTAssertTrue(view.presentationGeometryAvailable)
        let bitmap = try XCTUnwrap(NSBitmapImageRep(data: data))
        let jpeg = try XCTUnwrap(bitmap.representation(using: .jpeg,properties: [.compressionFactor: 0.9]))
        options.nativeResources = Pele.NativeResources { _ in jpeg }
        try mounted.update(source,options: options)
        XCTAssertTrue(try XCTUnwrap(mounted.result).svg.contains("data:image/png;base64,"))
        XCTAssertGreaterThan(try pixels(view).1,200,"Approved JPEG must decode and paint through the same native raster lane")

    }
    func testOwnedGenerationRetainsLastGoodImageAcrossApprovalFailureAndRecoversWithoutReparse() throws {
        let red = try png(.red), blue = try png(.blue), failure = ImageApprovalFailure()
        var calls = 0, failed = false, current = red
        let owner = Pele.NativeResources { _ in calls += 1; if failed { throw failure }; return current }
        let (window,view) = window(); defer { window.close() }
        var options = Pele.MountOptions(); options.nativeResources = owner; options.zoom = .disabled
        let mounted = try Pele.mount(view,source: source,options: options); defer { mounted.destroy() }
        let original = try XCTUnwrap(view.presentation); let first = try pixels(view); XCTAssertGreaterThan(first.1,200)
        try mounted.refresh(); XCTAssertEqual(calls,1)
        failed = true
        try mounted.invalidateResources(); XCTAssertTrue((mounted.error as AnyObject?) === failure)
        XCTAssertTrue(view.presentation === original); XCTAssertTrue((view.diagnostic as AnyObject?) === failure); XCTAssertEqual(calls,2)
        failed = false; current = blue
        try mounted.invalidateResources(); XCTAssertNil(view.diagnostic); XCTAssertEqual(calls,3)
        let next = try pixels(view); XCTAssertNotEqual(first.0,next.0); XCTAssertGreaterThan(next.2,200); XCTAssertEqual(next.1,0)
        view.isHidden = true; current = red; try mounted.invalidateResources(); XCTAssertEqual(calls,3)
        view.isHidden = false; try mounted.refresh(); XCTAssertEqual(calls,4); XCTAssertGreaterThan(try pixels(view).1,200)
        XCTAssertGreaterThanOrEqual(owner.generation,3)
        let generation = owner.generation
        NotificationCenter.default.post(name: NSFont.fontSetChangedNotification,object: nil)
        try mounted.refresh(); XCTAssertEqual(owner.generation,generation + 1); XCTAssertEqual(calls,5)

    }
    func testNativeURIApprovalAndDecodeBudgetsPrecedePlatformSVGRasterization() throws {
        let image = try png(.red), engine = Pele.Engine(); var calls = 0
        let owner = Pele.NativeResources { _ in calls += 1; return image }
        let blocked = source.replacingOccurrences(of: "https://example.test/picture.png",with: "javascript:bad")
        _ = try engine.renderNative(blocked,nativeResources: owner); XCTAssertEqual(calls,0)
        var settings = Pele.Engine.Options(); settings.images = false
        _ = try engine.renderNative(source,nativeResources: owner,options: settings); XCTAssertEqual(calls,0)
        XCTAssertThrowsError(try engine.renderNative(source,nativeResources: Pele.NativeResources()))
        var limits = Pele.NativeResources.Limits(); limits.imageBytes = 1
        XCTAssertThrowsError(try engine.renderNative(source,nativeResources: Pele.NativeResources(limits: limits) { _ in image })) {
            XCTAssertEqual(($0 as? PeleError)?.code,"limit")
        }
        limits = .init(); limits.imagePixels = 3
        XCTAssertThrowsError(try engine.renderNative(source,nativeResources: Pele.NativeResources(limits: limits) { _ in image })) {
            XCTAssertEqual(($0 as? PeleError)?.code,"limit")
        }
        for input in [Data("<svg xmlns='http://www.w3.org/2000/svg'/>".utf8),Data(image.prefix(20))] {
            XCTAssertThrowsError(try engine.renderNative(source,nativeResources: Pele.NativeResources { _ in input }))
        }
        // The legacy SVG lane deliberately keeps its approved URI contract.
        let old = try engine.renderPresentation(source); XCTAssertTrue(old.svg.contains("https://example.test/picture.png"))
        XCTAssertEqual(calls,0)
        var observed: [Data] = []
        let first = "./é.png", second = "./e\u{301}.png"
        XCTAssertEqual(first,second,"Swift String uses canonical equivalence; host URI approval must retain raw UTF8 identity")
        XCTAssertNotEqual(Data(first.utf8),Data(second.utf8))
        let exactOwner = Pele.NativeResources { uri in
            observed.append(Data(uri.utf8)); return Data(uri.utf8) == Data(first.utf8) ? image : nil
        }
        _ = try engine.renderNative(source.replacingOccurrences(of: "https://example.test/picture.png",with: first),nativeResources: exactOwner)
        XCTAssertThrowsError(try engine.renderNative(source.replacingOccurrences(of: "https://example.test/picture.png",with: second),nativeResources: exactOwner))
        XCTAssertEqual(observed,[Data(first.utf8),Data(second.utf8)])
    }
    func testInstalledFontMetricsMatchConcreteSVGFontAndCustomMeasurementsRemainAuthoritative() throws {
        let engine = Pele.Engine(), source = "flowchart LR\nA[\"<strong>WWWWWW</strong> <em>Italic</em>\"] -->|iiiiii| B[MMMM]"
        var options = Pele.Engine.Options(); options.fontFamily = "Helvetica"; options.fontFamilyMono = "Menlo"
        let owner = Pele.NativeResources(); let native = try engine.renderNative(source,nativeResources: owner,options: options)
        var resources = Pele.HostResources()
        resources.textWidth = { request in
            let family = request.style & 4 != 0 ? "Menlo" : "Helvetica"
            let base = CTFontCreateWithName(family as CFString,request.size,nil)
            var traits: CTFontSymbolicTraits = []
            if request.style & 1 != 0 { traits.insert(.boldTrait) }; if request.style & 2 != 0 { traits.insert(.italicTrait) }
            let font = CTFontCreateCopyWithSymbolicTraits(base,0,nil,traits,traits) ?? base
            let text = NSAttributedString(string: request.text,attributes: [NSAttributedString.Key(kCTFontAttributeName as String): font])
            return CTLineGetTypographicBounds(CTLineCreateWithAttributedString(text),nil,nil,nil)
        }
        XCTAssertEqual(native.svg,try engine.renderPresentation(source,options: options,resources: resources).svg)
        XCTAssertTrue(native.svg.contains("Helvetica"))
        resources.textWidth = { _ in 123 }
        let custom = try engine.renderNative(source,nativeResources: owner,options: options,resources: resources)
        XCTAssertEqual(custom.svg,try engine.renderPresentation(source,options: options,resources: resources).svg)
        let (window,view) = window(); defer { window.close() }
        var settings = Pele.MountOptions(); settings.nativeResources = owner; settings.render = options; settings.zoom = .disabled
        let mounted = try Pele.mount(view,source: source,options: settings); defer { mounted.destroy() }
        let before = try pixels(view).0; settings.render.fontFamily = "Courier"
        try mounted.update(source,options: settings); XCTAssertNotEqual(before,try pixels(view).0)
        settings.render.fontFamily = "PeleMissingFontFamily_9D472"
        let accepted = view.presentation
        try mounted.update(source,options: settings); XCTAssertNotNil(mounted.error); XCTAssertTrue(view.presentation === accepted)
        settings.render.fontFamily = "Helvetica"; try mounted.update(source,options: settings); XCTAssertNil(view.diagnostic)
    }
    private func graphData(_ graph: Pele.PublicGraph) throws -> Data { let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys]; return try encoder.encode(graph) }
    func testMutableImageErrorsPublishRevisionAndRecoverWithOriginalCallbackIdentity() throws {
        let data = try png(.red), failure = ImageApprovalFailure(), engine = Pele.Engine()
        let document = try engine.parseMutable(source); let before = try document.snapshot()
        let owner = Pele.NativeResources { _ in throw failure }
        XCTAssertThrowsError(try document.renderNative(nativeResources: owner)) { XCTAssertTrue(($0 as AnyObject) === failure) }
        XCTAssertEqual(document.revision,before.revision + 1); XCTAssertEqual(try graphData(document.snapshot().graph),try graphData(before.graph))
        owner.imageLoader = { _ in data }; owner.invalidate()
        XCTAssertTrue(try document.renderNative(nativeResources: owner).svg.contains("data:image/png;base64,")); XCTAssertEqual(document.revision,before.revision + 2)
        try document.close()
        XCTAssertThrowsError(try document.renderNative(nativeResources: owner))
    }
}
