import AppKit

extension Pele {
    /// Native presentation hooks run on MainActor. Parsing and geometry stay in Rust.
    @MainActor public struct MountOptions {
        public var render = Engine.Options()
        public var limits = ParseLimits()
        public var outputLimit: OutputLimit?
        public var resources = HostResources()
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
                let width = min(bounds.width, image.size.width)
                let height = image.size.width > 0 ? image.size.height * width / image.size.width : 0
                image.draw(in: NSRect(x: 0, y: 0, width: width, height: height), from: .zero,
                           operation: .sourceOver, fraction: diagnostic == nil ? 1 : 0.4,
                           respectFlipped: true, hints: nil)
            }
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
        }
        public override func layout() { super.layout(); mounted?.observeRefresh() }
        public override func viewDidMoveToWindow() { super.viewDidMoveToWindow(); mounted?.environmentChanged() }
        public override func viewDidHide() { super.viewDidHide(); mounted?.observeRefresh() }
        public override func viewDidUnhide() { super.viewDidUnhide(); mounted?.observeRefresh() }
        private static func message(_ error: any Error) -> String {
            if let error = error as? CompatibilityError { return error.message }
            if let error = error as? PeleError { return error.message }
            return String(describing: error)
        }
        fileprivate func install(_ image: NSImage?, error: (any Error)?) {
            presentation = image; diagnostic = error
            toolTip = error.map(Self.message)
            setAccessibilityHelp(toolTip)
            invalidateIntrinsicContentSize(); needsDisplay = true
        }
    }

    @MainActor public static func mount(_ view: DiagramView, source: String, options: MountOptions = .init()) throws -> Mounted {
        guard view.mounted?.busy != true else { throw PeleError(code: "semantic", message: "Mounted document is already in use") }
        view.mounted?.destroy()
        let mounted = Mounted(view: view, options: options)
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
        private weak var view: DiagramView?
        private var settings: MountOptions
        private let engine = Engine()
        private let idPrefix = "pele-native-" + UUID().uuidString.lowercased()
        private var model: MutableDocument?
        private var drawnModel: MutableDocument?
        private var parsedKey: Data?
        private var natural: FullRenderedDiagram?
        private var adapts: Bool?
        private var drawnWidth: Double?
        private var stale = true
        private var current = false
        fileprivate var busy = false
        private var destroyed = false
        private var initialized = false
        private var resourceGeneration: UInt64 = 0
        private var drawnGeneration: UInt64 = 0
        private var image: NSImage?
        fileprivate init(view: DiagramView, options: MountOptions) { self.view = view; settings = options }

        /// Forces resource-sensitive re-rendering without reparsing the model.
        public func invalidateResources() throws {
            guard !busy else { throw busyError }
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
                let key = try parseIdentity(source)
                let sourcePolicy = (settings.limits.source ?? settings.render.limit.map { .number(Double($0)) } ?? .undefined).nativeValue
                let edgePolicy = (settings.limits.maxEdges ?? settings.render.maxEdges.map { .number(Double($0)) } ?? .undefined).nativeValue
                let incomparable = (sourcePolicy.kind == 2 && Double(bitPattern: sourcePolicy.bits).isNaN) || (edgePolicy.kind == 2 && Double(bitPattern: edgePolicy.bits).isNaN)
                if !current || incomparable || key != parsedKey {
                    var parse = Engine.ParseOptions(); parse.config = settings.render.config
                    let limits = ParseLimits(source: settings.limits.source ?? settings.render.limit.map { .number(Double($0)) } ?? .undefined,
                                             maxEdges: settings.limits.maxEdges ?? settings.render.maxEdges.map { .number(Double($0)) } ?? .undefined)
                    model = try engine.parseMutable(source, limits: limits, options: parse)
                    parsedKey = key
                }
                current = true; stale = true
                if !initial && waits { busy = false; return }
                drawingAttempted = true
                let shown = try draw(fresh: true, force: true, preserveError: false)
                busy = false
                if let shown { try settings.onRender?(shown) }
            } catch {
                // Notification errors follow an already committed operation.
                if !busy { throw error }
                busy = false; current = false
                if drawingAttempted { model = drawnModel; parsedKey = nil; stale = false }
                self.error = error; publishDiagnostic()
                try settings.onError?(error)
            }
        }
        public func refresh() throws {
            guard !busy else { throw busyError }
            guard !destroyed, let view, view.window != nil, !waits, model != nil else { return }
            let width = available
            let fresh = stale || drawnGeneration != resourceGeneration
            guard fresh || (adapts != false && settings.render.maxWidth == nil && drawnWidth?.bitPattern != width.bitPattern && result?.width.bitPattern != width.bitPattern) else { return }
            let preserveError = !current
            busy = true
            do {
                let shown = try draw(fresh: stale || drawnGeneration != resourceGeneration, force: false, preserveError: preserveError)
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
            if view?.mounted === self { view?.stopObservingFonts() }
        }
        fileprivate func fontsChanged() {
            guard !destroyed else { return }
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
        private func publishDiagnostic() { view?.install(image, error: error) }
        private func render(_ document: MutableDocument, width: Double?) throws -> FullRenderedDiagram {
            var options = settings.render; options.maxWidth = width
            options.limit = nil; options.maxEdges = nil; options.config = [:]
            if options.idPrefix == nil { options.idPrefix = idPrefix }
            if options.nativeTheme == nil { options.nativeTheme = NativeTheme() }
            let output = settings.outputLimit ?? settings.render.outputLimit.map { .number(Double($0)) } ?? .undefined
            return try document.render(outputLimit: output, options: options, resources: settings.resources)
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
                stale = view?.window == nil
                drawnModel = model; result = drawing; image = prepared
                if !preserveError { error = nil }
                publishDiagnostic()
                return quiet ? nil : drawing
            } catch {
                natural = savedNatural; adapts = savedAdapts; drawnWidth = savedWidth
                throw error
            }
        }
    }
}
