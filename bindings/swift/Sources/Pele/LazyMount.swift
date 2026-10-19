import Foundation

extension Pele.Registry {
    /// Select the initial source's family, then mount on MainActor. The returned
    /// controller keeps this registry and the ordinary synchronous lifecycle.
    /// Later update calls require their families to have been selected explicitly.
    /// No native code download or background AppKit access is performed.
    @MainActor public func mountAsync(
        _ view: Pele.DiagramView,
        source: String,
        options: Pele.MountOptions = .init()
    ) async throws -> Pele.Mounted {
        try Task.checkCancellation()
        // Parsing presents the authoritative source error after selection,
        // matching lazy renderAsync and the pinned JavaScript entry point.
        let type = try? detectType(source)
        _ = try load(type)
        return try Pele.mount(view, source: source, registry: self, options: options)
    }
}
