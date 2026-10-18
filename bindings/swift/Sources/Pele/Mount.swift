import AppKit

// Both retained native model lanes share presentation without exporting a graph.
internal protocol MountedDocument: AnyObject {
    func render(outputLimit: Pele.OutputLimit, options: Pele.Engine.Options, resources: Pele.HostResources) throws -> FullRenderedDiagram
    func renderPresentation(outputLimit: Pele.OutputLimit?, budget: Pele.PresentationBudget, options: Pele.Engine.Options, resources: Pele.HostResources) throws -> FullRenderedDiagram
    @MainActor func renderNative(nativeResources: Pele.NativeResources, outputLimit: Pele.OutputLimit?, budget: Pele.PresentationBudget, options: Pele.Engine.Options, resources: Pele.HostResources) throws -> FullRenderedDiagram
}
extension MountedDocument {
    @MainActor func renderNative(nativeResources: Pele.NativeResources, outputLimit: Pele.OutputLimit?, budget: Pele.PresentationBudget, options: Pele.Engine.Options, resources: Pele.HostResources) throws -> FullRenderedDiagram {
        try renderPresentation(outputLimit: outputLimit, budget: budget, options: options, resources: resources)
    }
    func renderPresentation(outputLimit: Pele.OutputLimit?, budget: Pele.PresentationBudget, options: Pele.Engine.Options, resources: Pele.HostResources) throws -> FullRenderedDiagram {
        // Custom providers have no Rust-produced artifact in this slice.
        try render(outputLimit: outputLimit ?? .undefined, options: options, resources: resources)
    }
}
extension Pele.MutableDocument: MountedDocument {}
extension Pele.ProviderDocument: MountedDocument {}

extension Pele {
    /// Native presentation hooks run on MainActor. Parsing and geometry stay in Rust.
    @MainActor public struct MountOptions {
        public var render = Engine.Options()
        public var limits = ParseLimits()
        public var outputLimit: OutputLimit?
        public var resources = HostResources()
        /// Nil creates an owned native resource policy for this mounted view.
        public var nativeResources: NativeResources?
        public var zoom: ZoomPolicy = .automatic
        public var presentationBudget = PresentationBudget()
        /// A custom image explicitly depicts the complete SVG over its full image
        /// bounds. Default false disables native geometry/actions for custom images.
        public var preparedImagePreservesSVGGeometry = false
        public var onLink: ((PresentationLink) throws -> Void)?
        public var preparePresentation: ((FullRenderedDiagram) throws -> NSImage)?
        public var onRender: ((FullRenderedDiagram) throws -> Void)?
        public var onError: ((any Error) throws -> Void)?
        public init() {}
    }

    /// Retained native drawing. Mounting replaces only this view's controller.
    @MainActor public final class DiagramView: NSView {
        public fileprivate(set) var presentation: NSImage?
        public fileprivate(set) var diagnostic: (any Error)?
        fileprivate var mounted: Mounted?
        internal let nativePresentationState = NativePresentationState()
        internal var zoomController: Zoom?
        internal var presentationCamera: Zoom?
        private weak var observedZoomWindow: NSWindow?
        internal func synchronizeInputObservation() {
            if zoomController != nil || hasNativeLinkActions { startZoomObservation() }
            else { stopZoomObservation() }
        }
        internal func startZoomObservation() {
            stopZoomObservation(); observedZoomWindow = window
            if let window { NotificationCenter.default.addObserver(self, selector: #selector(windowInactive), name: NSWindow.didResignKeyNotification, object: window) }
        }
        internal func stopZoomObservation() {
            NotificationCenter.default.removeObserver(self, name: NSWindow.didResignKeyNotification, object: nil)
            observedZoomWindow = nil
        }
        @objc private func windowInactive(_ notification: Notification) { zoomController?.cancelInput(); cancelNativeLinkInput(); needsDisplay = true }
        public override var acceptsFirstResponder: Bool { zoomController?.zoomable == true || hasNativeLinkActions }
        public override var focusRingMaskBounds: NSRect { bounds }
        public override func drawFocusRingMask() { NSBezierPath(rect: bounds).fill() }
        public override func becomeFirstResponder() -> Bool {
            if NSApp.currentEvent?.type == .keyDown { zoomController?.keyboardFocus = true }
            needsDisplay = true; return super.becomeFirstResponder()
        }
        public override func resignFirstResponder() -> Bool { zoomController?.cancelInput(); cancelNativeLinkInput(); needsDisplay = true; return super.resignFirstResponder() }
        public override func resetCursorRects() {
            if let zoomController, zoomController.zoomable && zoomController.isEnlarged {
                addCursorRect(bounds, cursor: zoomController.dragging ? .closedHand : .openHand)
            } else {
                super.resetCursorRects()
                for index in nativePresentationState.commit?.artifact?.links.indices ?? 0..<0 {
                    if hasNativeLinkActions, let rect = nativeLinkRect(index) { addCursorRect(rect, cursor: .pointingHand) }
                }
            }
        }
        public override func keyDown(with event: NSEvent) { if zoomController?.key(event) != true && !nativeLinkKey(event) { super.keyDown(with: event) } }
        public override func scrollWheel(with event: NSEvent) { if zoomController?.wheel(event) != true { super.scrollWheel(with: event) } }
        public override func magnify(with event: NSEvent) { if zoomController?.magnify(event) != true { super.magnify(with: event) } }
        public override func mouseDown(with event: NSEvent) { nativeLinkDown(event); if zoomController?.down(event) != true && nativePresentationState.pressed == nil { super.mouseDown(with: event) } }
        public override func mouseDragged(with event: NSEvent) { nativeLinkDragged(event); if zoomController?.dragged(event) != true && nativePresentationState.pressedAt == nil { super.mouseDragged(with: event) } }
        public override func mouseUp(with event: NSEvent) {
            let claimed = zoomController?.up(event) == true
            let suppressed = zoomController?.consumeSuppressedActivation() == true
            if !nativeLinkUp(event, suppressed: suppressed) && !claimed { super.mouseUp(with: event) }
        }
        fileprivate func observeFonts() {
            NotificationCenter.default.removeObserver(self, name: NSFont.fontSetChangedNotification, object: nil)
            NotificationCenter.default.addObserver(self, selector: #selector(fontsChanged), name: NSFont.fontSetChangedNotification, object: nil)
        }
        fileprivate func stopObservingFonts() { NotificationCenter.default.removeObserver(self, name: NSFont.fontSetChangedNotification, object: nil) }
        @objc private func fontsChanged(_ notification: Notification) { mounted?.fontsChanged() }
        public override var isFlipped: Bool { true }
        public override var intrinsicContentSize: NSSize { presentation?.size ?? NSSize(width: NSView.noIntrinsicMetric, height: NSView.noIntrinsicMetric) }
        public override func draw(_ dirtyRect: NSRect) {
            super.draw(dirtyRect)
            if let image = presentation {
                let rect = nativeImageRect
                NSGraphicsContext.saveGraphicsState(); NSBezierPath(rect: bounds).addClip()
                image.draw(in: rect, from: .zero,
                           operation: .sourceOver, fraction: diagnostic == nil ? 1 : 0.4,
                           respectFlipped: true, hints: nil)
                NSGraphicsContext.restoreGraphicsState()
            }
            drawNativeLinkFocus()
            if let diagnostic {
                let full = Self.message(diagnostic)
                let lines = full.components(separatedBy: .newlines).filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
                let summary = lines.count > 1 ? "\(lines[0]) \(lines[lines.count - 1])" : lines.first ?? ""
                let paragraph = NSMutableParagraphStyle()
                paragraph.lineBreakMode = presentation == nil ? .byWordWrapping : .byTruncatingTail
                let attributes: [NSAttributedString.Key: Any] = [.font: NSFont.monospacedSystemFont(ofSize: 11, weight: .regular), .foregroundColor: NSColor.systemRed, .paragraphStyle: paragraph]
                let area = presentation == nil ? bounds.insetBy(dx: 4, dy: 4) : NSRect(x: 0, y: max(0, bounds.height - 16), width: bounds.width, height: 16)
                ((presentation == nil ? full : summary) as NSString).draw(in: area, withAttributes: attributes)
            }
            if window?.firstResponder === self, window?.isKeyWindow == true, zoomController?.keyboardFocus == true {
                NSGraphicsContext.saveGraphicsState(); NSFocusRingPlacement.only.set()
                NSBezierPath(rect: bounds.insetBy(dx: 2, dy: 2)).fill()
                NSGraphicsContext.restoreGraphicsState()
            }
        }
        public override func layout() { super.layout(); mounted?.observeRefresh(); zoomController?.resized() }
        public override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow(); zoomController?.cancelInput(); cancelNativeLinkInput()
            synchronizeInputObservation()
            mounted?.environmentChanged()
        }
        public override func viewDidHide() { super.viewDidHide(); cancelNativeLinkInput(); mounted?.observeRefresh() }
        public override func viewDidUnhide() { super.viewDidUnhide(); mounted?.observeRefresh() }
        private static func message(_ error: any Error) -> String {
            if let error = error as? CompatibilityError { return error.message }
            if let error = error as? PeleError { return error.message }
            return String(describing: error)
        }
        fileprivate func install(_ image: NSImage?, error: (any Error)?, native: NativePresentationCommit?) {
            presentation = image; diagnostic = error
            if zoomController == nil { presentationCamera = nil }
            installNativePresentation(native)
            toolTip = error.map(Self.message)
            updateNativeAccessibilityHelp(toolTip)
            invalidateIntrinsicContentSize(); needsDisplay = true
        }
    }

    @MainActor public static func mount(_ view: DiagramView, source: String, options: MountOptions = .init()) throws -> Mounted {
        let engine = Engine()
        return try mount(view, source: source, options: options, parse: { source, limits, options in
            try engine.parseMutable(source, limits: limits, options: options)
        })
    }

    /// Mount using exactly this registry's current selection. Later updates do
    /// not select additional diagram families. Retained models keep their owner.
    @MainActor public static func mount(_ view: DiagramView, source: String, registry: Registry, options: MountOptions = .init()) throws -> Mounted {
        try mount(view, source: source, options: options, parse: { source, limits, options in
            // Detection chooses the owned document lane; the selected native parser
            // remains authoritative for source limits and preparation failures.
            if let type = try? registry.detectType(source), registry.providerTypes.contains(type) {
                return try registry.parseProvider(source, as: AnyObject.self, limits: limits, options: options)
            }
            return try registry.parseMutable(source, limits: limits, options: options)
        })
    }

    @MainActor private static func mount(_ view: DiagramView, source: String, options: MountOptions, parse: @escaping Mounted.Parse) throws -> Mounted {
        guard view.mounted?.busy != true else { throw PeleError(code: "semantic", message: "Mounted document is already in use") }
        view.mounted?.destroy()
        view.zoomController?.destroy()
        let mounted = Mounted(view: view, options: options, parse: parse)
        view.mounted = mounted
        view.observeFonts()
        do { try mounted.update(source) }
        catch { mounted.destroy(); throw error }
        return mounted
    }

    /// Owns parsed and last-drawn documents separately. Destroy stops automatic
    /// refresh; explicit updates remain valid, matching Pele's mount contract.
    @MainActor public final class Mounted {
        public private(set) var source = ""
        public private(set) var result: FullRenderedDiagram?
        public private(set) var error: (any Error)?
        public private(set) var unhandledCallbackError: (any Error)?
        public private(set) var zoom: Zoom?
        private var zoomPolicy: ZoomPolicy?
        private weak var view: DiagramView?
        private var settings: MountOptions
        fileprivate typealias Parse = (String, ParseLimits, Engine.ParseOptions) throws -> any MountedDocument
        private let parse: Parse
        private let idPrefix = "pele-native-" + UUID().uuidString.lowercased()
        private var model: (any MountedDocument)?
        private var drawnModel: (any MountedDocument)?
        private var parsedKey: Data?
        private var natural: FullRenderedDiagram?
        private var adapts: Bool?
        private var drawnWidth: Double?
        private var stale = true
        private var current = false
        fileprivate var busy = false
        private var destroyed = false
        private var initialized = false
        private let defaultNativeResources = NativeResources()
        private var resourceGeneration: UInt64 = 0
        private var drawnGeneration: UInt64 = 0
        private var drawnNativeGeneration: UInt64?
        private weak var drawnNativeOwner: NativeResources?
        private var nativeResourcesChanged: Bool {
            let owner = settings.nativeResources ?? defaultNativeResources
            return drawnNativeOwner !== owner || drawnNativeGeneration != owner.generation
        }
        private var image: NSImage?
        private var nativePresentation: NativePresentationCommit?
        fileprivate init(view: DiagramView, options: MountOptions, parse: @escaping Parse) { self.view = view; settings = options; self.parse = parse }

        /// Forces resource-sensitive re-rendering without reparsing the model.
        public func invalidateResources() throws {
            guard !busy else { throw busyError }
            (settings.nativeResources ?? defaultNativeResources).invalidate()
            resourceGeneration &+= 1; stale = true
            try refresh()
        }
        private var busyError: PeleError { PeleError(code: "semantic", message: "Mounted document is already in use") }
        private var waits: Bool { view.map { $0.window != nil && ($0.isHiddenOrHasHiddenAncestor || $0.bounds.width <= 0) } ?? false }
        private var available: Double { settings.render.maxWidth ?? Double(view?.bounds.width ?? 0) }
        private func parseIdentity(_ source: String) throws -> Data {
            struct Key: Encodable { let source: String; let sourceKind: UInt32; let sourceBits: UInt64; let edgeKind: UInt32; let edgeBits: UInt64; let config: [String: ConfigValue] }
            let sourceLimit = (settings.limits.source ?? settings.render.limit.map { .number(Double($0)) } ?? .undefined).nativeValue
            let edgeLimit = (settings.limits.maxEdges ?? settings.render.maxEdges.map { .number(Double($0)) } ?? .undefined).nativeValue
            let encoder = JSONEncoder(); encoder.outputFormatting = [.sortedKeys]
            return try encoder.encode(Key(source: source, sourceKind: sourceLimit.kind, sourceBits: sourceLimit.kind == 2 && Double(bitPattern: sourceLimit.bits) == 0 ? 0 : sourceLimit.bits, edgeKind: edgeLimit.kind, edgeBits: edgeLimit.kind == 2 && Double(bitPattern: edgeLimit.bits) == 0 ? 0 : edgeLimit.bits, config: settings.render.config))
        }
        public func update(_ source: String, options: MountOptions? = nil) throws {
            guard !busy else { throw busyError }
            let initial = !initialized
            initialized = true
            self.source = source
            if let options { settings = options }
            busy = true
            var drawingAttempted = false
            do {
                if let camera = settings.zoom.options, !camera.maxScale.isFinite {
                    throw PeleError(code: "presentation", message: "Native zoom maximum scale must be finite")
                }
                let key = try parseIdentity(source)
                let sourcePolicy = (settings.limits.source ?? settings.render.limit.map { .number(Double($0)) } ?? .undefined).nativeValue
                let edgePolicy = (settings.limits.maxEdges ?? settings.render.maxEdges.map { .number(Double($0)) } ?? .undefined).nativeValue
                let incomparable = (sourcePolicy.kind == 2 && Double(bitPattern: sourcePolicy.bits).isNaN) || (edgePolicy.kind == 2 && Double(bitPattern: edgePolicy.bits).isNaN)
                if !current || incomparable || key != parsedKey {
                    var parse = Engine.ParseOptions(); parse.config = settings.render.config
                    let limits = ParseLimits(source: settings.limits.source ?? settings.render.limit.map { .number(Double($0)) } ?? .undefined,
                                             maxEdges: settings.limits.maxEdges ?? settings.render.maxEdges.map { .number(Double($0)) } ?? .undefined)
                    model = try self.parse(source, limits, parse)
                    parsedKey = key
                }
                current = true; stale = true
                if !initial && waits { busy = false; try reconcileZoom(); return }
                drawingAttempted = true
                let shown = try draw(fresh: true, force: true, preserveError: false)
                busy = false
                if let shown { try settings.onRender?(shown) }
                try reconcileZoom()
            } catch {
                // Notification errors follow an already committed operation.
                if !busy { try reconcileZoom(); throw error }
                busy = false; current = false
                if drawingAttempted { model = drawnModel; parsedKey = nil; stale = false }
                self.error = error; publishDiagnostic()
                try settings.onError?(error)
                try reconcileZoom()
            }
        }
        private func reconcileZoom() throws {
            guard zoomPolicy != settings.zoom, let view else { return }
            let requested = settings.zoom.options
            guard requested?.maxScale.isFinite != false else { return }
            zoom?.destroy(); zoom = nil; zoomPolicy = settings.zoom
            if let requested {
                zoom = try Pele.enableZoom(view, options: requested)
                if destroyed { zoom?.destroy() }
            }
        }
        public func refresh() throws {
            guard !busy else { throw busyError }
            guard !destroyed, let view, view.window != nil, !waits, model != nil else { return }
            let width = available
            let fresh = stale || drawnGeneration != resourceGeneration || nativeResourcesChanged
            guard fresh || (adapts != false && settings.render.maxWidth == nil && drawnWidth?.bitPattern != width.bitPattern && result?.width.bitPattern != width.bitPattern) else { return }
            let preserveError = !current
            busy = true
            do {
                let shown = try draw(fresh: fresh, force: false, preserveError: preserveError)
                busy = false
                if let shown { try settings.onRender?(shown) }
            } catch {
                busy = false
                stale = false
                if current { self.error = error }
                if model !== drawnModel { model = drawnModel; parsedKey = nil; current = false }
                publishDiagnostic()
                try settings.onError?(error)
            }
        }
        public func destroy() {
            destroyed = true
            if let old = nativePresentation {
                nativePresentation = NativePresentationCommit(outcome: old.outcome, drawingType: old.drawingType, mapsSVG: old.mapsSVG, activate: nil, failed: nil)
            }
            zoom?.destroy()
            if view?.mounted === self { view?.stopObservingFonts(); view?.deactivateNativePresentation() }
        }
        fileprivate func fontsChanged() {
            guard !destroyed else { return }
            (settings.nativeResources ?? defaultNativeResources).invalidate()
            resourceGeneration &+= 1; stale = true; observeRefresh()
        }
        fileprivate func environmentChanged() {
            guard !destroyed else { return }
            stale = true; observeRefresh()
        }
        fileprivate func observeRefresh() {
            guard !destroyed, !busy else { return }
            do { try refresh() } catch { unhandledCallbackError = error }
        }
        private func publishDiagnostic() { view?.install(image, error: error, native: nativePresentation) }
        private func render(_ document: any MountedDocument, width: Double?) throws -> FullRenderedDiagram {
            var options = settings.render; options.maxWidth = width
            options.limit = nil; options.maxEdges = nil; options.config = [:]
            if options.idPrefix == nil { options.idPrefix = idPrefix }
            if options.nativeTheme == nil { options.nativeTheme = NativeTheme() }
            let output = settings.outputLimit ?? settings.render.outputLimit.map { .number(Double($0)) } ?? .undefined
            return try document.renderNative(nativeResources: settings.nativeResources ?? defaultNativeResources, outputLimit: output, budget: settings.presentationBudget, options: options, resources: settings.resources)
        }
        private func draw(fresh: Bool, force: Bool, preserveError: Bool) throws -> FullRenderedDiagram? {
            guard let model else { return nil }
            let savedNatural = natural, savedAdapts = adapts, savedWidth = drawnWidth
            do {
                if fresh || natural == nil { natural = try render(model, width: nil); adapts = nil }
                guard let natural else { return nil }
                let width = available
                var drawing = natural
                if width > 0 && width < natural.width {
                    if adapts == nil { adapts = try render(model, width: 1).svg != natural.svg }
                    if adapts == true { drawing = try render(model, width: width) }
                }
                let quiet = !force && drawing.svg == result?.svg
                let prepared: NSImage?
                if quiet { prepared = image } else {
                    guard drawing.width.isFinite, drawing.height.isFinite, drawing.width > 0, drawing.height > 0 else {
                        throw PeleError(code: "presentation", message: "Drawing has unusable native presentation dimensions")
                    }
                    if let prepare = settings.preparePresentation { prepared = try prepare(drawing) }
                    else { prepared = NSImage(data: Data(drawing.svg.utf8)) }
                    guard let prepared, prepared.size.width.isFinite, prepared.size.height.isFinite, prepared.size.width > 0, prepared.size.height > 0 else {
                        throw PeleError(code: "presentation", message: "AppKit could not decode drawing")
                    }
                }
                drawnWidth = width; drawnGeneration = resourceGeneration
                drawnNativeOwner = settings.nativeResources ?? defaultNativeResources
                drawnNativeGeneration = drawnNativeOwner?.generation
                stale = view?.window == nil
                drawnModel = model; result = drawing; image = prepared
                nativePresentation = NativePresentationCommit(outcome: drawing.presentation ?? .unavailable(.unsupportedDiagram),
                    drawingType: drawing.type, mapsSVG: settings.preparePresentation == nil || settings.preparedImagePreservesSVGGeometry,
                    activate: destroyed ? nil : settings.onLink,
                    failed: { [weak self] error in self?.unhandledCallbackError = error })
                if !preserveError { error = nil }
                publishDiagnostic()
                if !quiet { zoom?.refresh() }
                return quiet ? nil : drawing
            } catch {
                natural = savedNatural; adapts = savedAdapts; drawnWidth = savedWidth
                throw error
            }
        }
    }
}
