import AppKit

@MainActor internal struct NativePresentationCommit {
    let identifier = UUID()
    let outcome: Pele.PresentationOutcome
    let drawingType: String
    let mapsSVG: Bool
    let activate: ((Pele.PresentationLink) throws -> Void)?
    let failed: ((any Error) -> Void)?
    var artifact: Pele.PresentationArtifact? {
        guard mapsSVG, case .available(let artifact) = outcome else { return nil }
        return artifact
    }
}
@MainActor internal final class NativePresentationState {
    var commit: NativePresentationCommit?
    var children = [NativePresentationLinkElement]()
    var pressed: Int?
    var pressedAt: NSPoint?
    var focused: Int?
}
@MainActor internal final class NativePresentationLinkElement: NSAccessibilityElement {
    // AppKit's inherited accessibility selectors are nonisolated. Keep only
    // immutable Sendable actor closures across that boundary, never transfer
    // the mutable NSAccessibilityElement itself to an isolated closure.
    nonisolated private let frameAction: @MainActor @Sendable () -> NSRect
    nonisolated private let pressAction: @MainActor @Sendable () -> Bool
    init(view: Pele.DiagramView, index: Int, identifier: UUID, label: String, href: String) {
        frameAction = { [weak view] in
            guard let view, view.nativePresentationState.commit?.identifier == identifier,
                  let rect = view.nativeLinkRect(index), let window = view.window else { return .zero }
            return window.convertToScreen(view.convert(rect, to: nil))
        }
        pressAction = { [weak view] in
            guard let view, view.nativePresentationState.commit?.identifier == identifier else { return false }
            return view.activateNativeLink(index)
        }
        super.init()
        setAccessibilityElement(true); setAccessibilityRole(.link)
        setAccessibilityParent(view); setAccessibilityLabel(label)
        setAccessibilityURL(URL(string: href))
    }
    override nonisolated func accessibilityFrame() -> NSRect {
        guard Thread.isMainThread else { return .zero }
        return MainActor.assumeIsolated(frameAction)
    }
    override nonisolated func accessibilityPerformPress() -> Bool {
        guard Thread.isMainThread else { return false }
        return MainActor.assumeIsolated(pressAction)
    }
}

extension Pele.DiagramView {
    /// Rust's last accepted metadata, independent of diagnostics or custom image mapping.
    public var nativePresentation: Pele.PresentationOutcome? { nativePresentationState.commit?.outcome }
    /// Custom preparation must explicitly preserve complete SVG content/geometry.
    public var presentationGeometryAvailable: Bool { nativePresentationState.commit?.artifact != nil }
    internal var nativeImageRect: NSRect {
        if let camera = presentationCamera { return camera.imageRect }
        guard let image = presentation, image.size.width > 0 else { return .zero }
        let width = min(bounds.width, image.size.width)
        return NSRect(x: 0, y: 0, width: width, height: image.size.height * width / image.size.width)
    }
    internal var hasNativeLinkActions: Bool {
        nativePresentationState.commit?.activate != nil && nativePresentationState.commit?.artifact?.links.isEmpty == false
    }
    internal func installNativePresentation(_ commit: NativePresentationCommit?) {
        if commit == nil && nativePresentationState.commit == nil {
            setAccessibilityElement(true); setAccessibilityRole(.group); setAccessibilityLabel("Diagram"); return
        }
        guard nativePresentationState.commit?.identifier != commit?.identifier else { return }
        nativePresentationState.commit = commit
        nativePresentationState.pressed = nil; nativePresentationState.pressedAt = nil; nativePresentationState.focused = nil
        nativePresentationState.children = []
        setAccessibilityElement(true); setAccessibilityRole(.group)
        if let commit, let artifact = commit.artifact {
            setAccessibilityLabel(artifact.title ?? "\(commit.drawingType) diagram")
            setAccessibilityValue(artifact.text.map(\.text).joined(separator: "\n"))
            if commit.activate != nil {
                nativePresentationState.children = artifact.links.enumerated().map { index, link in
                    NativePresentationLinkElement(view: self, index: index, identifier: commit.identifier,
                        label: artifact.text.first(where: { $0.id == link.id })?.text ?? link.href, href: link.href)
                }
            }
        } else {
            setAccessibilityLabel(commit.map { "\($0.drawingType) diagram" })
            setAccessibilityValue(nil)
        }
        synchronizeNativeAccessibilityChildren()
        synchronizeInputObservation()
        window?.invalidateCursorRects(for: self)
    }
    internal func synchronizeNativeAccessibilityChildren() {
        // Preserve AppKit's native control traversal as well as virtual links.
        // Zoom creates/removes its bar after a drawing commit; it refreshes this
        // composition whenever controls or visibility change.
        let links: [Any] = nativePresentationState.children
        setAccessibilityChildren(links + NSAccessibility.unignoredChildren(from: subviews))
    }
    internal func updateNativeAccessibilityHelp(_ error: String?) {
        let description = nativePresentationState.commit?.artifact?.description
        let help = [description, error].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: "\n")
        setAccessibilityHelp(help.isEmpty ? nil : help)
    }
    internal func deactivateNativePresentation() {
        if let old = nativePresentationState.commit {
            installNativePresentation(NativePresentationCommit(outcome: old.outcome, drawingType: old.drawingType, mapsSVG: old.mapsSVG, activate: nil, failed: nil))
        }
    }
    internal func nativeLinkRect(_ index: Int) -> NSRect? {
        guard let artifact = nativePresentationState.commit?.artifact, artifact.links.indices.contains(index),
              artifact.width.isFinite, artifact.height.isFinite, artifact.width > 0, artifact.height > 0 else { return nil }
        let source = artifact.links[index].bounds, image = nativeImageRect
        let rect = NSRect(x: image.minX + source.x * image.width / artifact.width,
                          y: image.minY + source.y * image.height / artifact.height,
                          width: source.width * image.width / artifact.width,
                          height: source.height * image.height / artifact.height)
        guard [rect.origin.x,rect.origin.y,rect.width,rect.height].allSatisfy(\.isFinite), rect.width >= 0, rect.height >= 0 else { return nil }
        let clipped = rect.intersection(bounds).intersection(visibleRect)
        return clipped.isEmpty || clipped.isNull ? nil : clipped
    }
    internal func nativeLink(at point: NSPoint) -> Int? {
        guard hasNativeLinkActions, !isHiddenOrHasHiddenAncestor else { return nil }
        if diagnostic != nil && presentation != nil && point.y >= max(0,bounds.height - 16) { return nil }
        return nativePresentationState.commit?.artifact?.links.indices.reversed().first { nativeLinkRect($0)?.contains(point) == true }
    }
    @discardableResult internal func activateNativeLink(_ index: Int) -> Bool {
        guard window != nil, !isHiddenOrHasHiddenAncestor, let commit = nativePresentationState.commit,
              let artifact = commit.artifact, artifact.links.indices.contains(index),
              nativeLinkRect(index) != nil, let activate = commit.activate else { return false }
        do { try activate(artifact.links[index]); return true }
        catch { commit.failed?(error); return false }
    }
    internal func cancelNativeLinkInput() {
        nativePresentationState.pressed = nil; nativePresentationState.pressedAt = nil; nativePresentationState.focused = nil
    }
    internal func nativeLinkDown(_ event: NSEvent) {
        nativePresentationState.pressed = nil; nativePresentationState.pressedAt = nil
        guard event.buttonNumber == 0, event.clickCount == 1 else { return }
        let point = convert(event.locationInWindow, from: nil)
        if let index = nativeLink(at: point) {
            nativePresentationState.pressed = index; nativePresentationState.pressedAt = point
            nativePresentationState.focused = nil
            window?.makeFirstResponder(self)
        }
    }
    internal func nativeLinkDragged(_ event: NSEvent) {
        guard let start = nativePresentationState.pressedAt else { return }
        let point = convert(event.locationInWindow, from: nil)
        if abs(point.x - start.x) + abs(point.y - start.y) > 4 { nativePresentationState.pressed = nil }
    }
    internal func nativeLinkUp(_ event: NSEvent, suppressed: Bool) -> Bool {
        let pressed = nativePresentationState.pressed
        nativePresentationState.pressed = nil; nativePresentationState.pressedAt = nil
        guard !suppressed, event.buttonNumber == 0, event.clickCount == 1, let pressed,
              nativeLink(at: convert(event.locationInWindow, from: nil)) == pressed else { return false }
        return activateNativeLink(pressed)
    }
    internal func nativeLinkKey(_ event: NSEvent) -> Bool {
        guard hasNativeLinkActions, window?.firstResponder === self,
              event.modifierFlags.intersection([.command,.control,.option]).isEmpty,
              let links = nativePresentationState.commit?.artifact?.links, !links.isEmpty else { return false }
        if event.keyCode == 48 {
            let step = event.modifierFlags.contains(.shift) ? -1 : 1
            let previous = nativePresentationState.focused ?? (step > 0 ? -1 : links.count)
            let next = previous + step
            guard links.indices.contains(next) else { nativePresentationState.focused = nil; needsDisplay = true; return false }
            nativePresentationState.focused = next; needsDisplay = true; return true
        }
        if let index = nativePresentationState.focused, ["\r"," "].contains(event.charactersIgnoringModifiers ?? "") { _ = activateNativeLink(index); return true }
        return false
    }
    internal func drawNativeLinkFocus() {
        guard window?.isKeyWindow == true, window?.firstResponder === self,
              let index = nativePresentationState.focused, let rect = nativeLinkRect(index) else { return }
        NSGraphicsContext.saveGraphicsState(); NSFocusRingPlacement.only.set()
        NSBezierPath(rect: rect).fill(); NSGraphicsContext.restoreGraphicsState()
    }
}
