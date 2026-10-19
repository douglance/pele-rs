import Foundation
import CPele

extension Pele {
    /// Caller-owned parser and renderer for a canonical detected diagram type.
    /// Confine this provider and its documents to one owner/actor; they are not Sendable.
    public final class Diagram<Model: AnyObject> {
        public var type: String
        public var section: String?
        public var keepComments: Bool
        public var parse: (ProviderParseContext) throws -> Model
        public var render: (Model, ProviderRenderContext) throws -> ProviderRendered
        public init(type: String, section: String? = nil, keepComments: Bool = false,
                    parse: @escaping (ProviderParseContext) throws -> Model,
                    render: @escaping (Model, ProviderRenderContext) throws -> ProviderRendered) {
            self.type = type; self.section = section; self.keepComments = keepComments
            self.parse = parse; self.render = render
        }
    }
    /// Prepared UTF-8 source and finite Config value snapshot. maxEdges is effective:
    /// the undefined default is already 5000; null and raw numbers remain distinct.
    public struct ProviderParseContext {
        public let source: String
        public let config: [String: ConfigValue]
        public let title: String?
        public let lineOffset: UInt64
        public let maxEdges: EdgeLimit
    }
    /// A custom provider returns trusted SVG. Rust applies normal output policy,
    /// optional native theme and returned link-ID decoding after this callback.
    public struct ProviderRendered {
        public var svg: String
        public var width: Double
        public var height: Double
        public var links: [DiagramLink]
        public init(svg: String, width: Double, height: Double, links: [DiagramLink] = []) {
            self.svg = svg; self.width = width; self.height = height; self.links = links
        }
    }
    /// Context values may be retained; resource methods borrow the active render.
    /// All resource calls share one reentry guard and preserve original host errors.
    /// Config remains a value snapshot, separate from these borrowed capabilities.
    public final class ProviderRenderContext {
        public let config: [String: ConfigValue]
        public let options: Engine.Options
        public let outputLimit: OutputLimit
        private var call: OpaquePointer?
        private let invocation: ProviderInvocation
        private var resourceActive = false
        private var clockError: (any Error)?
        fileprivate init(request: PeleProviderRenderV10, call: OpaquePointer?, invocation: ProviderInvocation) throws {
            config = try JSONDecoder().decode([String: ConfigValue].self, from: providerData(request.config, request.config_len))
            var decoded = try JSONDecoder().decode(Engine.Options.self, from: providerData(request.options, request.options_len))
            let n = request.numbers
            if n.present & 1 != 0 { decoded.fontSize = Double(bitPattern: n.font_size_bits) }
            if n.present & 2 != 0 { decoded.maxWidth = Double(bitPattern: n.max_width_bits) }
            if n.present & 4 != 0 { decoded.directionBreakpoint = Double(bitPattern: n.direction_breakpoint_bits) }
            if n.present & 8 != 0 { decoded.padding = Double(bitPattern: n.padding_bits) }
            if n.present & 16 != 0 { decoded.now = Double(bitPattern: n.now_bits) }
            switch request.output_kind {
            case 0: outputLimit = .undefined
            case 1: outputLimit = .null
            case 2: outputLimit = .number(Double(bitPattern: request.output_bits))
            default: outputLimit = decoded.outputLimit.map { .number(Double($0)) } ?? .undefined
            }
            options = decoded; self.call = call; self.invocation = invocation
        }
        fileprivate func finish() { call = nil }
        private func withResource<T>(_ body: (OpaquePointer) throws -> T) throws -> T {
            guard let call else { throw PeleError(code: "semantic", message: "Provider resources are no longer active") }
            guard !resourceActive else { throw PeleError(code: "semantic", message: "Provider resources are already in use") }
            resourceActive = true; defer { resourceActive = false }
            _ = invocation.takeResourceError?()
            do { return try body(call) }
            catch { throw invocation.takeResourceError?() ?? error }
        }
        public func textWidth(_ text: String, size: Double, style: UInt8 = 0) throws -> Double {
            try withResource { call in
                guard style <= 7 else { throw PeleError(code: "semantic", message: "Invalid provider text style") }
                let bytes = Data(text.utf8); var bits: UInt64 = 0
                let _: Bool = try bytes.withUnsafeBytes { input in
                    try Pele.compatibilityResult(pele_foreign_text_width_v10(call, input.bindMemory(to: UInt8.self).baseAddress,
                        input.count, size.bitPattern, UInt32(style), &bits), as: Bool.self)
                }
                return Double(bitPattern: bits)
            }
        }
        /// Decodes authored entity names and resolves each call independently.
        public func icon(_ name: String) throws -> TrustedSVG? {
            try withResource { call in
                let bytes = Data(name.utf8)
                let markup: ProviderResourceValue<String> = try bytes.withUnsafeBytes { input in
                    try Pele.compatibilityResult(pele_foreign_icon_v10(call, input.bindMemory(to: UInt8.self).baseAddress,
                        input.count), as: ProviderResourceValue<String>.self)
                }
                return markup.value.map { TrustedSVG(trustedMarkup: $0) }
            }
        }
        /// Uses explicit finite options.now or the render session's cached host clock.
        public func nowMillis() throws -> Double {
            try withResource { call in
                if let clockError { throw clockError }
                var bits: UInt64 = 0
                do {
                    let _: Bool = try Pele.compatibilityResult(pele_foreign_now_millis_v10(call, &bits), as: Bool.self)
                    return Double(bitPattern: bits)
                } catch {
                    // Rust caches clock failures for this render. Retain the exact
                    // Swift error too when the provider catches and retries it.
                    let original = invocation.takeResourceError?() ?? error
                    clockError = original; throw original
                }
            }
        }
        public func parseDate(_ text: String) throws -> Double? {
            try withResource { call in
                let bytes = Data(text.utf8); var present: UInt32 = 0, bits: UInt64 = 0
                let _: Bool = try bytes.withUnsafeBytes { input in
                    try Pele.compatibilityResult(pele_foreign_parse_date_v10(call, input.bindMemory(to: UInt8.self).baseAddress,
                        input.count, &present, &bits), as: Bool.self)
                }
                return present == 0 ? nil : Double(bitPattern: bits)
            }
        }
        public func calendarFields(_ instantMillis: Double) throws -> CalendarFields? {
            try withResource { call in
                try Pele.compatibilityResult(pele_foreign_calendar_fields_v10(call, instantMillis.bitPattern), as: ProviderResourceValue<CalendarFields>.self).value
            }
        }
        public func calendarInstant(_ civil: CivilDateTime) throws -> Double? {
            try withResource { call in
                let bytes = try JSONEncoder().encode(civil); var present: UInt32 = 0, bits: UInt64 = 0
                let _: Bool = try bytes.withUnsafeBytes { input in
                    try Pele.compatibilityResult(pele_foreign_calendar_instant_v10(call, input.bindMemory(to: UInt8.self).baseAddress,
                        input.count, &present, &bits), as: Bool.self)
                }
                return present == 0 ? nil : Double(bitPattern: bits)
            }
        }
    }
    /// Owns the exact reference model and parse-time registration/config. Closing
    /// the registry or replacing its entry does not invalidate this document.
    public final class ProviderDocument<Model: AnyObject> {
        private var handle: OpaquePointer?
        private var active = false
        public let type: String
        public private(set) var revision: UInt64
        fileprivate init(handle: OpaquePointer, metadata: ProviderMetadata) throws {
            guard metadata.modelKind == "foreign", let revision = UInt64(metadata.revision), String(revision) == metadata.revision else {
                throw PeleError(code: "internal", message: "Invalid provider document metadata")
            }
            // Ownership transfers only after every throwing initialization step.
            var pointer: UnsafeMutableRawPointer?
            let _: ProviderMetadata = try Pele.compatibilityResult(pele_provider_model_v10(handle, &pointer), as: ProviderMetadata.self)
            guard let pointer, Unmanaged<ProviderModelBox>.fromOpaque(pointer).takeUnretainedValue().value is Model else {
                throw PeleError(code: "semantic", message: "Provider model has a different Swift type")
            }
            self.handle = handle; type = metadata.type; self.revision = revision
        }
        deinit { if let handle { pele_provider_document_free_v10(handle) } }
        private func withHandle<T>(_ body: (OpaquePointer) throws -> T) throws -> T {
            guard let handle else { throw PeleError(code: "semantic", message: "Provider document is closed") }
            guard !active else { throw PeleError(code: "semantic", message: "Provider document is already in use") }
            active = true; defer { active = false }
            return try withExtendedLifetime(self) { try body(handle) }
        }
        public func close() throws {
            guard !active else { throw PeleError(code: "semantic", message: "Provider document is already in use") }
            if let handle {
                self.handle = nil; active = true; defer { active = false }
                pele_provider_document_free_v10(handle)
            }
        }
        public var model: Model {
            get throws { try withHandle { handle in
                var pointer: UnsafeMutableRawPointer?
                let _: ProviderMetadata = try Pele.compatibilityResult(pele_provider_model_v10(handle, &pointer), as: ProviderMetadata.self)
                guard let pointer, let model = Unmanaged<ProviderModelBox>.fromOpaque(pointer).takeUnretainedValue().value as? Model else {
                    throw PeleError(code: "semantic", message: "Provider model has a different Swift type")
                }
                return model
            }}
        }
        public func render(options: Engine.Options = .init(), resources: HostResources = .init()) throws -> FullRenderedDiagram {
            try render(outputLimit: options.outputLimit.map { .number(Double($0)) } ?? .undefined, options: options, resources: resources)
        }
        public func render(outputLimit: OutputLimit, options: Engine.Options = .init(), resources: HostResources = .init()) throws -> FullRenderedDiagram {
            let settings = try RenderNumberEncoding(options, outputLimit: outputLimit)
            return try withHandle { handle in
                let invocation = ProviderInvocation()
                let response = resources.withCapturedNativeResources { host, takeError in
                    withoutActuallyEscaping(takeError) { errors in
                        invocation.takeResourceError = errors; defer { invocation.takeResourceError = nil }
                        return settings.withPacket { packet in settings.optionsJSON.withUnsafeBytes { input in
                            pele_provider_render_v10(handle, revision, input.bindMemory(to: UInt8.self).baseAddress, input.count,
                                packet, settings.outputKind, settings.outputBits, host, Unmanaged.passUnretained(invocation).toOpaque())
                        }}
                    }
                }
                do {
                    let outcome = try Pele.compatibilityResult(response.handle, as: ProviderOutcome<FullRenderedDiagram>.self)
                    if let value = outcome.revision, let next = UInt64(value), String(next) == value { revision = next }
                    else { throw PeleError(code: "internal", message: "Invalid provider render revision") }
                    return try outcome.result.get(invocation: invocation)
                } catch {
                    // A bounded transport failure can follow callback mutation.
                    // Reconcile only small metadata, never a full model snapshot.
                    if let status = try? Pele.compatibilityResult(pele_provider_status_v10(handle), as: ProviderMetadata.self),
                       let next = UInt64(status.revision) { revision = next }
                    throw error
                }
            }
        }
    }
}
extension Pele.Registry {
    /// Captures the current type as the registration key. All other provider
    /// properties remain live; replacements affect future parses only.
    public func register<Model: AnyObject>(_ diagram: Pele.Diagram<Model>) throws {
        let type = diagram.type
        let provider = ProviderBox(diagram), id = Data(type.utf8)
        var table = PeleDiagramProviderV10()
        table.version = 10; table.struct_size = UInt32(MemoryLayout<PeleDiagramProviderV10>.size)
        table.context = Unmanaged.passUnretained(provider).toOpaque()
        table.retain = providerRetain; table.release = providerRelease; table.release_model = providerModelRelease
        table.describe = providerDescribe; table.parse = providerParse; table.render = providerRender
        try withHandle { handle in try withExtendedLifetime(provider) { try id.withUnsafeBytes { input in
            let _: Bool = try Pele.compatibilityResult(pele_registry_register_provider_v10(handle,
                input.bindMemory(to: UInt8.self).baseAddress, input.count, &table), as: Bool.self)
            providerTypes.insert(type)
        }}}
    }
    /// Parses the effective Swift-authored provider. Built-ins use parseMutable.
    /// A mismatched Model type throws and releases the unpublished document.
    public func parseProvider<Model: AnyObject>(_ source: String, as: Model.Type,
            limits: Pele.ParseLimits = .init(), options: Pele.Engine.ParseOptions = .init()) throws -> Pele.ProviderDocument<Model> {
        let bytes = Data(source.utf8), settings = try JSONEncoder().encode(options)
        let sourceLimit = (limits.source ?? .number(Double(options.limit))).nativeValue
        let edgeLimit = (limits.maxEdges ?? .number(Double(options.maxEdges))).nativeValue
        return try withHandle { handle in
            let invocation = ProviderInvocation(); var document: OpaquePointer?
            let result = bytes.withUnsafeBytes { input in settings.withUnsafeBytes { settings in
                pele_registry_parse_provider_v10(handle, input.bindMemory(to: UInt8.self).baseAddress, input.count,
                    settings.bindMemory(to: UInt8.self).baseAddress, settings.count, sourceLimit.kind, sourceLimit.bits,
                    edgeLimit.kind, edgeLimit.bits, Unmanaged.passUnretained(invocation).toOpaque(), &document)
            }}
            do {
                let outcome = try Pele.compatibilityResult(result, as: ProviderOutcome<ProviderMetadata>.self)
                let metadata = try outcome.result.get(invocation: invocation)
                guard let document else { throw PeleError(code: "internal", message: "Missing provider document") }
                return try Pele.ProviderDocument(handle: document, metadata: metadata)
            } catch { if let document { pele_provider_document_free_v10(document) }; throw error }
        }
    }
}

private struct ProviderResourceValue<Value: Decodable>: Decodable { let value: Value? }
private struct ProviderMetadata: Decodable { let type: String; let revision: String; let modelKind: String }
private struct ProviderOutcome<Value: Decodable>: Decodable { let revision: String?; let result: ProviderResult<Value> }
private enum ProviderResult<Value: Decodable>: Decodable {
    case success(Value), failure(Pele.PublicOperationError)
    private enum Keys: String, CodingKey { case Ok, Err }
    init(from decoder: any Decoder) throws {
        let fields = try decoder.container(keyedBy: Keys.self)
        if fields.contains(.Ok) { self = .success(try fields.decode(Value.self, forKey: .Ok)) }
        else { self = .failure(try fields.decode(Pele.PublicOperationError.self, forKey: .Err)) }
    }
    func get(invocation: ProviderInvocation) throws -> Value {
        switch self {
        case .success(let value): return value
        case .failure(.hostException(let token)):
            guard token == 1, let error = invocation.error else { throw PeleError(code: "internal", message: "Unbound provider exception") }
            throw error
        case .failure(.pele(let error)): throw error
        case .failure(let error): throw Pele.PublicOperationFailure(value: error)
        }
    }
}
private final class ProviderInvocation {
    var error: (any Error)?
    var takeResourceError: (() -> (any Error)?)?
    func fail(_ error: any Error, reply: UnsafeMutablePointer<PeleForeignReplyV10>) {
        self.error = error; reply.pointee.status = 2; reply.pointee.exception = 1
    }
}
private final class ProviderModelBox {
    let value: AnyObject
    init(_ value: AnyObject) { self.value = value }
}
private final class ProviderBox {
    let describe: () -> (String?, Bool)
    let parse: (Pele.ProviderParseContext) throws -> AnyObject
    let render: (AnyObject, Pele.ProviderRenderContext) throws -> Pele.ProviderRendered
    init<Model>(_ diagram: Pele.Diagram<Model>) {
        describe = { (diagram.section, diagram.keepComments) }
        parse = { try diagram.parse($0) }
        render = { model, context in
            guard let model = model as? Model else { throw PeleError(code: "internal", message: "Foreign provider model type changed") }
            return try diagram.render(model, context)
        }
    }
}
private func providerData(_ pointer: UnsafePointer<UInt8>?, _ count: Int) -> Data {
    guard count > 0, let pointer else { return Data() }
    return Data(bytes: pointer, count: count)
}
private func providerBytesRelease(_ pointer: UnsafeMutableRawPointer?) {
    if let pointer { Unmanaged<NSData>.fromOpaque(pointer).release() }
}
private func providerBytes(_ data: Data) -> PeleForeignBytesV10 {
    let bytes = data as NSData
    return PeleForeignBytesV10(owner: Unmanaged.passRetained(bytes).toOpaque(),
        data: bytes.bytes.assumingMemoryBound(to: UInt8.self), len: bytes.length, release: providerBytesRelease)
}
private func providerRetain(_ pointer: UnsafeMutableRawPointer?) {
    if let pointer { _ = Unmanaged<ProviderBox>.fromOpaque(pointer).retain() }
}
private func providerRelease(_ pointer: UnsafeMutableRawPointer?) {
    if let pointer { Unmanaged<ProviderBox>.fromOpaque(pointer).release() }
}
private func providerModelRelease(_ context: UnsafeMutableRawPointer?, _ pointer: UnsafeMutableRawPointer?) {
    if let pointer { Unmanaged<ProviderModelBox>.fromOpaque(pointer).release() }
}
private func providerInvocation(_ call: OpaquePointer?) -> ProviderInvocation {
    Unmanaged<ProviderInvocation>.fromOpaque(pele_foreign_call_context_v10(call)!).takeUnretainedValue()
}
private func providerDescribe(_ pointer: UnsafeMutableRawPointer?, _ call: OpaquePointer?, _ phase: UInt32,
                              _ reply: UnsafeMutablePointer<PeleForeignReplyV10>?) {
    guard let pointer, let reply else { return }
    let (section, comments) = Unmanaged<ProviderBox>.fromOpaque(pointer).takeUnretainedValue().describe()
    struct Description: Encodable { let section: String?; let keepComments: Bool }
    do { reply.pointee.payload = providerBytes(try JSONEncoder().encode(Description(section: section, keepComments: comments))) }
    catch { providerInvocation(call).fail(error, reply: reply) }
}
private func providerParse(_ pointer: UnsafeMutableRawPointer?, _ call: OpaquePointer?,
                           _ request: UnsafePointer<PeleProviderParseV10>?, _ reply: UnsafeMutablePointer<PeleForeignReplyV10>?) {
    guard let pointer, let request, let reply else { return }
    let provider = Unmanaged<ProviderBox>.fromOpaque(pointer).takeUnretainedValue(), value = request.pointee
    do {
        let context = Pele.ProviderParseContext(source: String(decoding: providerData(value.source, value.source_len), as: UTF8.self),
            config: try JSONDecoder().decode([String: ConfigValue].self, from: providerData(value.config, value.config_len)),
            title: value.has_title == 0 ? nil : String(decoding: providerData(value.title, value.title_len), as: UTF8.self),
            lineOffset: value.line_offset, maxEdges: value.edge_kind == 1 ? .null : .number(Double(bitPattern: value.edge_bits)))
        let model = ProviderModelBox(try provider.parse(context))
        reply.pointee.model = Unmanaged.passRetained(model).toOpaque()
    } catch { providerInvocation(call).fail(error, reply: reply) }
}
private func providerRender(_ pointer: UnsafeMutableRawPointer?, _ model: UnsafeMutableRawPointer?, _ call: OpaquePointer?,
                            _ request: UnsafePointer<PeleProviderRenderV10>?, _ reply: UnsafeMutablePointer<PeleForeignReplyV10>?) {
    guard let pointer, let model, let request, let reply else { return }
    let provider = Unmanaged<ProviderBox>.fromOpaque(pointer).takeUnretainedValue()
    let invocation = providerInvocation(call)
    do {
        let context = try Pele.ProviderRenderContext(request: request.pointee, call: call, invocation: invocation)
        defer { context.finish() }
        let result = try provider.render(Unmanaged<ProviderModelBox>.fromOpaque(model).takeUnretainedValue().value, context)
        // Encode all fallible values before publishing owned reply parts.
        let links = try JSONEncoder().encode(result.links)
        reply.pointee.payload = providerBytes(Data(result.svg.utf8)); reply.pointee.links = providerBytes(links)
        reply.pointee.width_bits = result.width.bitPattern; reply.pointee.height_bits = result.height.bitPattern
    } catch { invocation.fail(error, reply: reply) }
}
