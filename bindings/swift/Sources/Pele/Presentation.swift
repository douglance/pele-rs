import Foundation
import CPele

extension Pele {
    /// Opt-in metadata storage budget; Rust clamps requests to its hard ceilings.
    public struct PresentationBudget: Sendable, Equatable {
        public var maxElements: Int
        public var maxTextBytes: Int
        public init(maxElements: Int = 4096, maxTextBytes: Int = 1_048_576) {
            self.maxElements = maxElements; self.maxTextBytes = maxTextBytes
        }
        internal func packet() throws -> PelePresentationOptionsV11 {
            guard maxElements >= 0, maxTextBytes >= 0 else {
                throw PeleError(code: "presentation", message: "Native presentation budgets must not be negative")
            }
            var packet = PelePresentationOptionsV11()
            packet.version = 11; packet.struct_size = UInt32(MemoryLayout<PelePresentationOptionsV11>.size)
            packet.max_elements = UInt64(maxElements); packet.max_text_bytes = UInt64(maxTextBytes)
            return packet
        }
    }
    public struct PresentationRect: Codable, Sendable {
        public let x: Double; public let y: Double
        public let width: Double; public let height: Double
    }
    public struct PresentationText: Codable, Sendable {
        public let id: String; public let text: String
    }
    public struct PresentationLink: Codable, Sendable {
        public let id: String; public let href: String; public let `internal`: Bool
        public let bounds: PresentationRect
    }
    public struct PresentationArtifact: Codable, Sendable {
        public let version: UInt32
        public let width: Double; public let height: Double
        public let title: String?; public let description: String?
        public let text: [PresentationText]; public let links: [PresentationLink]
    }
    public enum PresentationUnavailable: String, Codable, Sendable {
        case unsupportedDiagram, budgetExceeded, invalidGeometry, invalidText
    }
    public enum PresentationOutcome: Codable, Sendable {
        case available(PresentationArtifact)
        case unavailable(PresentationUnavailable)
        private enum Keys: String, CodingKey { case status, value }
        public init(from decoder: any Decoder) throws {
            let fields = try decoder.container(keyedBy: Keys.self)
            switch try fields.decode(String.self, forKey: .status) {
            case "available":
                let artifact = try fields.decode(PresentationArtifact.self, forKey: .value)
                guard artifact.version == 1 else {
                    throw DecodingError.dataCorruptedError(forKey: .value, in: fields, debugDescription: "Unsupported native presentation version")
                }
                self = .available(artifact)
            case "unavailable": self = .unavailable(try fields.decode(PresentationUnavailable.self, forKey: .value))
            default: throw DecodingError.dataCorruptedError(forKey: .status, in: fields, debugDescription: "Invalid native presentation status")
            }
        }
        public func encode(to encoder: any Encoder) throws {
            var fields = encoder.container(keyedBy: Keys.self)
            switch self {
            case .available(let artifact):
                try fields.encode("available", forKey: .status); try fields.encode(artifact, forKey: .value)
            case .unavailable(let reason):
                try fields.encode("unavailable", forKey: .status); try fields.encode(reason, forKey: .value)
            }
        }
    }
}
extension Pele.Engine {
    /// Render with opt-in native accessibility/link frames. Unsupported metadata
    /// does not fail an otherwise successful SVG. Source integer parse policies
    /// retain Engine's existing contract; exact policies can parse a document first.
    public func renderPresentation(_ source: String, outputLimit: Pele.OutputLimit? = nil,
        budget: Pele.PresentationBudget = .init(), options: Options = .init(), resources: Pele.HostResources = .init()
    ) throws -> FullRenderedDiagram {
        let settings = try RenderNumberEncoding(options, outputLimit: outputLimit ?? options.outputLimit.map { .number(Double($0)) } ?? .undefined)
        var presentation = try budget.packet(); let source = Data(source.utf8)
        let handle = try resources.withNativeResources { host in withUnsafePointer(to: &presentation) { presentation in
            settings.withPacket { numbers in source.withUnsafeBytes { input in settings.optionsJSON.withUnsafeBytes { bytes in
                pele_render_presentation_v11(input.bindMemory(to: UInt8.self).baseAddress, input.count,
                    bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count, numbers, settings.outputKind, settings.outputBits, presentation, host)
            }}}
        }}
        return try Pele.compatibilityResult(handle, as: FullRenderedDiagram.self)
    }
    public func renderPresentation(_ document: Pele.Document, outputLimit: Pele.OutputLimit? = nil,
        budget: Pele.PresentationBudget = .init(), options: Options = .init(), resources: Pele.HostResources = .init()
    ) throws -> FullRenderedDiagram {
        let settings = try RenderNumberEncoding(options, outputLimit: outputLimit ?? options.outputLimit.map { .number(Double($0)) } ?? .undefined)
        var presentation = try budget.packet()
        let handle = try resources.withNativeResources { host in withExtendedLifetime(document) { withUnsafePointer(to: &presentation) { presentation in
            settings.withPacket { numbers in settings.optionsJSON.withUnsafeBytes { bytes in
                pele_render_document_presentation_v11(document.handle, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count,
                    numbers, settings.outputKind, settings.outputBits, presentation, host)
            }}
        }}}
        return try Pele.compatibilityResult(handle, as: FullRenderedDiagram.self)
    }
}
