import Foundation

public struct PeleError: Error, Decodable, Sendable, CustomStringConvertible {
    public let code: String
    public let message: String
    public var description: String { "\(code): \(message)" }
    public init(code: String, message: String) {
        self.code = code
        self.message = message
    }
}

/// Namespace for the shared Rust engine and native host adapters.
public enum Pele {}
