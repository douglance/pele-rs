import AppKit

extension Pele {
    public struct ZoomLabels: Sendable, Equatable {
        public var zoomIn: String?
        public var zoomOut: String?
        public var reset: String?
        public init(zoomIn: String? = nil, zoomOut: String? = nil, reset: String? = nil) {
            self.zoomIn = zoomIn; self.zoomOut = zoomOut; self.reset = reset
        }
    }
    public struct ZoomOptions: Sendable, Equatable {
        public var always = false
        public var maxScale = 3.0
        public var controls = true
        public var labels = ZoomLabels()
        public init() {}
    }
    public enum ZoomPolicy: Sendable, Equatable {
        case automatic, disabled, always, custom(ZoomOptions)
        internal var options: ZoomOptions? {
            switch self {
            case .disabled: return nil
            case .automatic: return .init()
            case .always: var options = ZoomOptions(); options.always = true; return options
            case .custom(let options): return options
            }
        }
    }
    @MainActor public static func enableZoom(_ view: DiagramView, options: ZoomOptions = .init()) throws -> Zoom {
        guard options.maxScale.isFinite else {
            throw PeleError(code: "presentation", message: "Native zoom maximum scale must be finite")
        }
        view.zoomController?.destroy()
        let zoom = Zoom(view: view, options: options)
        view.zoomController = zoom; view.startZoomObservation(); zoom.refresh()
        return zoom
    }
    @MainActor final class ZoomControls: NSView { override var isFlipped: Bool { true } }
    /// Native image camera. Input changes presentation only, never Rust layout.
    @MainActor public final class Zoom: NSObject {
        private weak var view: DiagramView?
        private let options: ZoomOptions
        private var natural = NSSize.zero, rest = NSSize.zero, box = NSSize.zero
        private var measuredBounds = NSSize.zero
        private var origin = NSPoint.zero
        private var activeScale = 0.0
        private var destroyed = false
        private var bar: NSView?
        private var buttons: [String: NSButton] = [:]
        private var dragPoint: NSPoint?
        private var dragDistance = 0.0
        private var suppressActivation = false
        internal var keyboardFocus = true
        public var fit: Double { natural.width > 0 && rest.width > 0 ? rest.width / natural.width : 1 }
        public var scale: Double { activeScale == 0 ? fit : activeScale }
        internal var zoomable: Bool { view?.presentation != nil && natural.width > 0 && rest.width > 0 && (options.always || fit < 0.999) }
        private var maximum: Double { max(fit, options.maxScale) }
        internal var imageRect: NSRect {
            activeScale == 0 ? NSRect(origin: .zero, size: rest) : NSRect(x: -origin.x * activeScale, y: -origin.y * activeScale, width: natural.width * activeScale, height: natural.height * activeScale)
        }
        fileprivate init(view: DiagramView, options: ZoomOptions) { self.view = view; self.options = options; super.init() }
        private func measure() {
            guard let view else { return }
            measuredBounds = view.bounds.size
            natural = view.presentation?.size ?? .zero
            guard natural.width.isFinite, natural.height.isFinite, natural.width > 0, natural.height > 0 else { natural = .zero; rest = .zero; box = .zero; return }
            let width = max(0, min(view.bounds.width, natural.width))
            rest = NSSize(width: width, height: natural.height * (width / natural.width))
            box = NSSize(width: max(rest.width, view.bounds.width), height: max(rest.height, view.bounds.height))
            if box.width <= rest.width + 1 && box.height <= rest.height + 1 { box = rest }
        }
        public func refresh() {
            activeScale = 0; origin = .zero; dragPoint = nil; suppressActivation = false
            measure(); offer(); changed()
        }
        internal func resized() {
            guard !destroyed, let view, view.bounds.size != measuredBounds else { return }
            refresh()
        }
        public func reset() {
            activeScale = 0; origin = .zero; dragPoint = nil; suppressActivation = false
            measure(); offer(); changed()
        }
        public func zoomBy(_ factor: Double) { show(scale * factor) }
        public func zoomTo(_ scale: Double) { show(scale) }
        private func show(_ requested: Double, at anchor: NSPoint? = nil) {
            guard !requested.isNaN else { return }
            if activeScale == 0 { measure() }
            guard zoomable else { return }
            let before = NSRect(origin: .zero, size: activeScale == 0 ? rest : box)
            let from = scale, target = min(max(requested, fit), maximum)
            guard target.isFinite, (natural.width * target).isFinite, (natural.height * target).isFinite else { return }
            if target <= fit * 1.001 { reset(); return }
            let at = anchor ?? NSPoint(x: before.midX, y: before.midY)
            let point = NSPoint(x: (activeScale == 0 ? 0 : origin.x) + (at.x - before.minX) / from,
                                y: (activeScale == 0 ? 0 : origin.y) + (at.y - before.minY) / from)
            activeScale = target
            place(NSPoint(x: point.x - at.x / target, y: point.y - at.y / target))
            offer(); changed()
        }
        private func place(_ requested: NSPoint) {
            guard activeScale > 0 else { return }
            func within(_ at: Double, _ extent: Double, _ whole: Double) -> Double {
                extent >= whole ? min(max(at, whole - extent), 0) : min(max(at, 0), whole - extent)
            }
            origin = NSPoint(x: within(requested.x, box.width / activeScale, natural.width),
                             y: within(requested.y, box.height / activeScale, natural.height))
            changed()
        }
        private func pan(_ dx: Double, _ dy: Double) { if activeScale > 0 { place(NSPoint(x: origin.x - dx / activeScale, y: origin.y - dy / activeScale)) } }
        private func changed() {
            guard let view else { return }
            view.presentationCamera = self; view.needsDisplay = true
            view.window?.invalidateCursorRects(for: view)
        }
        private func offer() {
            guard !destroyed, let view else { return }
            if !zoomable && view.window?.firstResponder === view { view.window?.makeFirstResponder(nil) }
            if !options.controls { return }
            if bar == nil && zoomable {
                let container = ZoomControls(); bar = container; view.addSubview(container)
                let symbols = ["in": "plus", "out": "minus", "reset": "arrow.counterclockwise"]
                let labels = ["in": options.labels.zoomIn ?? "Zoom in", "out": options.labels.zoomOut ?? "Zoom out", "reset": options.labels.reset ?? "Reset zoom"]
                let actions = ["in": #selector(enlarge), "out": #selector(reduce), "reset": #selector(resetAction)]
                for (index, key) in ["in", "out", "reset"].enumerated() {
                    let button = NSButton(image: NSImage(systemSymbolName: symbols[key]!, accessibilityDescription: nil) ?? NSImage(), target: self, action: actions[key])
                    button.bezelStyle = .texturedRounded; button.toolTip = labels[key]; button.setAccessibilityLabel(labels[key])
                    button.frame = NSRect(x: 0, y: index * 26, width: 26, height: 24)
                    container.addSubview(button); buttons[key] = button
                }
            }
            bar?.frame = NSRect(x: max(0, view.bounds.width - 34), y: 8, width: 26, height: activeScale == 0 ? 50 : 76)
            bar?.isHidden = !zoomable
            buttons["in"]?.isEnabled = scale < maximum * 0.999
            buttons["out"]?.isEnabled = activeScale != 0
            buttons["reset"]?.isHidden = activeScale == 0
            if !zoomable && view.window?.firstResponder === view { view.window?.makeFirstResponder(nil) }
        }
        @objc private func enlarge() { show(scale * 1.5625) }
        @objc private func reduce() { show(scale / 1.5625) }
        @objc private func resetAction() { reset() }
        internal func key(_ event: NSEvent) -> Bool {
            guard zoomable, let view, view.window?.firstResponder === view,
                  event.modifierFlags.intersection([.control, .command, .option]).isEmpty else { return false }
            switch event.keyCode {
            case 123 where activeScale > 0: pan(48, 0)
            case 124 where activeScale > 0: pan(-48, 0)
            case 125 where activeScale > 0: pan(0, -48)
            case 126 where activeScale > 0: pan(0, 48)
            default:
                switch event.charactersIgnoringModifiers {
                case "+", "=": zoomBy(1.25)
                case "-": zoomBy(1 / 1.25)
                case "0", "\u{1b}": guard activeScale > 0 else { return false }; reset()
                default: return false
                }
            }
            keyboardFocus = true; changed(); return true
        }
        internal func wheel(_ event: NSEvent) -> Bool {
            guard zoomable, !event.modifierFlags.intersection([.control, .command]).isEmpty, let view else { return false }
            // AppKit scrollingDeltaY is positive upward; DOM deltaY is positive down.
            let turn = min(max(event.scrollingDeltaY, -24), 24)
            show(scale * pow(2, turn * 0.02), at: view.convert(event.locationInWindow, from: nil))
            return true
        }
        internal func magnify(_ event: NSEvent) -> Bool {
            guard zoomable, let view, event.magnification.isFinite else { return false }
            if event.phase == .cancelled { dragPoint = nil; return true }
            show(scale * max(0, 1 + event.magnification), at: view.convert(event.locationInWindow, from: nil))
            if event.phase == .ended || event.phase == .cancelled { dragPoint = nil }
            return true
        }
        internal func down(_ event: NSEvent) -> Bool {
            guard zoomable, event.buttonNumber == 0, let view else { return false }
            keyboardFocus = false; view.window?.makeFirstResponder(view)
            let point = view.convert(event.locationInWindow, from: nil)
            if event.clickCount == 2 { if activeScale == 0 { show(max(1, fit * 2), at: point) } else { reset() }; return true }
            dragPoint = point; dragDistance = 0; suppressActivation = false
            changed(); return true
        }
        internal func dragged(_ event: NSEvent) -> Bool {
            guard let before = dragPoint, let view else { return false }
            let point = view.convert(event.locationInWindow, from: nil), dx = point.x - before.x, dy = point.y - before.y
            dragPoint = point
            if activeScale > 0 {
                dragDistance += abs(dx) + abs(dy)
                if dragDistance > 4 { suppressActivation = true }
                pan(dx, dy)
            }
            return true
        }
        internal func up(_ event: NSEvent) -> Bool { guard dragPoint != nil else { return false }; dragPoint = nil; changed(); return true }
        internal func cancelInput() {
            dragPoint = nil; suppressActivation = false
            if let view { view.window?.invalidateCursorRects(for: view) }
        }
        internal func consumeSuppressedActivation() -> Bool {
            let suppressed = suppressActivation; suppressActivation = false; return suppressed
        }
        internal var dragging: Bool { dragPoint != nil && activeScale > 0 }
        internal var isEnlarged: Bool { activeScale != 0 }
        public func destroy() {
            guard !destroyed else { return }
            reset(); destroyed = true; bar?.removeFromSuperview(); bar = nil; buttons = [:]
            if let view, view.zoomController === self {
                if view.window?.firstResponder === view { view.window?.makeFirstResponder(nil) }
                view.zoomController = nil; view.stopZoomObservation(); view.needsDisplay = true
            }
        }
    }
}
