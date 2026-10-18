import Foundation

extension Pele {
    /// Stable native descriptor IDs. Record/prototype metadata is data, not a
    /// second implementation of diagram methods in Swift.
    public struct PublicModelSchema: Decodable, Sendable {
        public struct Method: Decodable, Sendable {
            public let id: UInt32
            public let name: String
            public let arity: UInt32
        }
        public let id: String
        public let version: UInt32
        public let root: UInt32?
        public let records: [ConfigValue]
        public let prototypes: [ConfigValue]
        public let methods: [Method]
    }
    public struct PublicLosslessError: Decodable, Sendable {
        public let name: String
        public let message: JSString
        public let code: String
        public let type: JSString?
        public let line: UInt64
        public let column: UInt64
        public let snippet: JSString
    }
    public enum PublicOperationError: Decodable, Sendable {
        case pele(CompatibilityError)
        case peleUTF16(PublicLosslessError)
        case javaScript(name: String, message: JSString)
        case hostException(UInt32)
        private enum Keys: String, CodingKey { case kind, value }
        private struct JavaScript: Decodable { let name: String; let message: JSString }
        public init(from decoder: any Decoder) throws {
            let fields = try decoder.container(keyedBy: Keys.self)
            switch try fields.decode(String.self, forKey: .kind) {
            case "pele": self = .pele(try fields.decode(CompatibilityError.self, forKey: .value))
            case "peleUtf16": self = .peleUTF16(try fields.decode(PublicLosslessError.self, forKey: .value))
            case "javaScript":
                let value = try fields.decode(JavaScript.self, forKey: .value)
                self = .javaScript(name: value.name, message: value.message)
            case "hostException": self = .hostException(try fields.decode(UInt32.self, forKey: .value))
            default: throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Unknown public operation error"))
            }
        }
    }
    public enum PublicOperationResult: Decodable, Sendable {
        case returned(PublicGraph.Atom)
        case thrown(PublicOperationError)
        private enum Keys: String, CodingKey { case Ok, Err }
        public init(from decoder: any Decoder) throws {
            let fields = try decoder.container(keyedBy: Keys.self)
            if fields.contains(.Ok) { self = .returned(try fields.decode(PublicGraph.Atom.self, forKey: .Ok)) }
            else { self = .thrown(try fields.decode(PublicOperationError.self, forKey: .Err)) }
        }
    }
    public struct PublicInvocationOutcome: Decodable, Sendable {
        public let revision: UInt64
        public let graph: PublicGraph
        public let result: PublicOperationResult
    }
}
