import Foundation
import CPele

public struct Theme: Codable, Sendable {
    public var background = "#ffffff"
    public var foreground = "#1f1f1f"
    public var surface = "#f3f3f3"
    public var border = "#8a8a8a"
    public var line = "#1f1f1f"
    public init() {}
}
public struct RenderOptions: Codable, Sendable {
    public var theme = Theme()
    public var fontSize = 16.0
    public var edgeFontSize = 14.0
    public var nodeGap = 40.0
    public var rankGap = 48.0
    public var padding = 16.0
    public var nodePadding = 16.0
    public var radius = 4.0
    public var strokeWidth = 1.5
    public var title = "Flowchart"
    public init() {}
}
public struct Point: Codable, Sendable { public let x: Double; public let y: Double }
public struct Rect: Codable, Sendable { public let x: Double; public let y: Double; public let width: Double; public let height: Double }
public struct DiagramNode: Codable, Sendable {
    public let id: String; public let label: String; public let shape: String; public let bounds: Rect
}
public struct DiagramEdge: Codable, Sendable {
    public let index: Int; public let source: String; public let target: String
    public let label: String; public let style: String; public let arrow: Bool
    public let points: [Point]; public let labelBounds: Rect?
}
public struct Geometry: Codable, Sendable {
    public let version: Int; public let direction: String
    public let nodes: [DiagramNode]; public let edges: [DiagramEdge]
}
public struct RenderedDiagram: Codable, Sendable {
    public let svg: String; public let width: Double; public let height: Double
    public let geometry: Geometry
}
public struct SourceSpan: Codable, Sendable {
    public let start: Int; public let end: Int; public let line: Int; public let column: Int
}
public struct PeleError: Error, Decodable, Sendable, CustomStringConvertible {
    public let code: String; public let message: String; public let span: SourceSpan?
    public var description: String { "\(code): \(message)" }
    public init(code: String, message: String, span: SourceSpan? = nil) {
        self.code = code; self.message = message; self.span = span
    }
}
private struct Envelope: Decodable {
    let version: Int; let ok: Bool; let result: RenderedDiagram?; let error: PeleError?
}

public enum Pele {
    /// Rust owns the synchronous render. All returned bytes are copied before free.
    public static func render(_ source: String, options: RenderOptions = .init()) throws -> RenderedDiagram {
        guard pele_abi_version() == 1 else { throw PeleError(code: "internal", message: "Unsupported C ABI version") }
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        let input = Data(source.utf8)
        let settings = try encoder.encode(options)
        let handle = input.withUnsafeBytes { sourceBytes in
            settings.withUnsafeBytes { optionBytes in
                pele_render(sourceBytes.bindMemory(to: UInt8.self).baseAddress, sourceBytes.count,
                            optionBytes.bindMemory(to: UInt8.self).baseAddress, optionBytes.count)
            }
        }
        guard let handle else { throw PeleError(code: "internal", message: "Native result allocation failed") }
        defer { pele_result_free(handle) }
        let count = pele_result_len(handle)
        guard count <= 16 * 1024 * 1024, let bytes = pele_result_data(handle) else {
            throw PeleError(code: "internal", message: "Invalid native result")
        }
        let data = Data(bytes: bytes, count: count)
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let envelope = try decoder.decode(Envelope.self, from: data)
        guard envelope.version == 1 else { throw PeleError(code: "internal", message: "Unsupported result version") }
        if !envelope.ok, envelope.result == nil, let error = envelope.error { throw error }
        guard pele_result_status(handle) == 0, envelope.ok, envelope.error == nil,
              let diagram = envelope.result, diagram.geometry.version == 1 else {
            throw PeleError(code: "internal", message: "Inconsistent result envelope")
        }
        return diagram
    }
}
