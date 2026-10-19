import Foundation
import CPele

extension Pele.Engine {
    /// Strict native rendering uses explicit installed-font metrics and approved
    /// embedded raster resources, plus the Rust-produced presentation artifact.
    @MainActor public func renderNative(_ source: String, nativeResources: Pele.NativeResources,
        outputLimit: Pele.OutputLimit? = nil, budget: Pele.PresentationBudget = .init(),
        options: Options = .init(), resources: Pele.HostResources = .init()) throws -> FullRenderedDiagram {
        let (options, resources) = try nativeResources.resolved(options, resources: resources)
        let settings = try RenderNumberEncoding(options, outputLimit: outputLimit ?? options.outputLimit.map { .number(Double($0)) } ?? .undefined)
        var presentation = try budget.packet(); let source = Data(source.utf8)
        let (response, imageError) = nativeResources.captured { native in resources.withCapturedNativeResources { host in withUnsafePointer(to: &presentation) { presentation in
            settings.withPacket { numbers in source.withUnsafeBytes { input in settings.optionsJSON.withUnsafeBytes { bytes in
                pele_render_native_v14(input.bindMemory(to: UInt8.self).baseAddress, input.count,
                    bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count, numbers, settings.outputKind, settings.outputBits, presentation, host, native)
            }}}
        }}}
        do {
            let result = try Pele.compatibilityResult(response.handle, as: FullRenderedDiagram.self)
            if let error = response.error ?? imageError { throw error }
            return result
        } catch { throw response.error ?? imageError ?? error }
    }
    @MainActor public func renderNative(_ document: Pele.Document, nativeResources: Pele.NativeResources,
        outputLimit: Pele.OutputLimit? = nil, budget: Pele.PresentationBudget = .init(),
        options: Options = .init(), resources: Pele.HostResources = .init()) throws -> FullRenderedDiagram {
        let (options, resources) = try nativeResources.resolved(options, resources: resources)
        let settings = try RenderNumberEncoding(options, outputLimit: outputLimit ?? options.outputLimit.map { .number(Double($0)) } ?? .undefined)
        var presentation = try budget.packet()
        let (response, imageError) = nativeResources.captured { native in resources.withCapturedNativeResources { host in withExtendedLifetime(document) { withUnsafePointer(to: &presentation) { presentation in
            settings.withPacket { numbers in settings.optionsJSON.withUnsafeBytes { bytes in
                pele_render_document_native_v14(document.handle, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count,
                    numbers, settings.outputKind, settings.outputBits, presentation, host, native)
            }}
        }}}}
        do {
            let result = try Pele.compatibilityResult(response.handle, as: FullRenderedDiagram.self)
            if let error = response.error ?? imageError { throw error }
            return result
        } catch { throw response.error ?? imageError ?? error }
    }
}
