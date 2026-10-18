import Foundation
import CPele

public enum ConfigValue: Codable, Sendable, Equatable {
    case null, bool(Bool), number(Double), string(String), array([ConfigValue]), object([String: ConfigValue])
    public init(from decoder: any Decoder) throws {
        let value = try decoder.singleValueContainer()
        if value.decodeNil() { self = .null }
        else if let v = try? value.decode(Bool.self) { self = .bool(v) }
        else if let v = try? value.decode(Double.self) { self = .number(v) }
        else if let v = try? value.decode(String.self) { self = .string(v) }
        else if let v = try? value.decode([ConfigValue].self) { self = .array(v) }
        else { self = .object(try value.decode([String: ConfigValue].self)) }
    }
    public func encode(to encoder: any Encoder) throws {
        var out = encoder.singleValueContainer()
        switch self {
        case .null: try out.encodeNil()
        case .bool(let v): try out.encode(v)
        case .number(let v): try out.encode(v)
        case .string(let v): try out.encode(v)
        case .array(let v): try out.encode(v)
        case .object(let v): try out.encode(v)
        }
    }
}
public struct PrepareOptions: Codable, Sendable {
    public var limit = 50_000
    public var config: [String: ConfigValue] = [:]
    public init() {}
}
public struct PreparedSource: Codable, Sendable {
    public let text: String
    public let title: String?
    public let config: [String: ConfigValue]
    public let withComments: String
    public let lineOffset: Int
}
public struct SourceInspection: Codable, Sendable {
    public let type: String?
    public let registered: Bool
    public let prepared: PreparedSource
    public let encodedText: String
}
public struct DiagramCapability: Codable, Sendable {
    public let type: String
    public let known: Bool
    public let registered: Bool
    public let syntaxParity: Bool
    public let semanticParity: Bool
    public let renderParity: Bool
    public let hostParity: Bool
}
public struct CompatibilityError: Error, Codable, Sendable, CustomStringConvertible {
    public let name: String
    public let message: String
    public let code: String
    public let type: String?
    public let line: Int
    public let column: Int
    public let snippet: String
    public var description: String { "\(code): \(message)" }
}
private struct CompatibilityEnvelope<T: Decodable>: Decodable {
    let version: Int; let ok: Bool; let result: T?; let error: CompatibilityError?
}
extension Pele {
    private static func compatibilityResult<T: Decodable>(_ handle: OpaquePointer?, as: T.Type) throws -> T {
        guard let handle else { throw PeleError(code: "internal", message: "Native result allocation failed") }
        defer { pele_result_free(handle) }
        let count = pele_result_len(handle)
        guard count <= 64 * 1024 * 1024, let bytes = pele_result_data(handle) else {
            throw PeleError(code: "internal", message: "Invalid native compatibility result")
        }
        let envelope = try JSONDecoder().decode(CompatibilityEnvelope<T>.self, from: Data(bytes: bytes, count: count))
        guard envelope.version == 2 else { throw PeleError(code: "internal", message: "Unsupported compatibility result version") }
        if !envelope.ok, envelope.result == nil, let error = envelope.error { throw error }
        guard pele_result_status(handle) == 0, envelope.ok, envelope.error == nil, let result = envelope.result else {
            throw PeleError(code: "internal", message: "Inconsistent compatibility result")
        }
        return result
    }
    /// Full-library source preparation. `registered` remains false until a real handler is installed.
    public static func inspect(_ source: String, options: PrepareOptions = .init()) throws -> SourceInspection {
        let input = Data(source.utf8), settings = try JSONEncoder().encode(options)
        let handle = input.withUnsafeBytes { sourceBytes in
            settings.withUnsafeBytes { optionBytes in
                pele_inspect(sourceBytes.bindMemory(to: UInt8.self).baseAddress, sourceBytes.count,
                             optionBytes.bindMemory(to: UInt8.self).baseAddress, optionBytes.count)
            }
        }
        return try compatibilityResult(handle, as: SourceInspection.self)
    }
    public static func capabilities() throws -> [DiagramCapability] {
        try compatibilityResult(pele_capabilities(), as: [DiagramCapability].self)
    }
}
