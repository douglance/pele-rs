import Foundation

extension Pele {
    public enum ErrorObjectKind: String, Codable, Sendable, CaseIterable { case error, pele, type, range, syntax, reference, eval, uri }
    /// Exact JavaScript string value. Conversion to Swift String is explicitly fallible.
    public struct JSString: Codable, Sendable, Equatable, Hashable {
        public var codeUnits: [UInt16]
        public init(codeUnits: [UInt16]) { self.codeUnits = codeUnits }
        public init(_ string: String) { codeUnits = Array(string.utf16) }
        public var stringIfWellFormed: String? {
            var i = 0
            while i < codeUnits.count {
                let unit = codeUnits[i]
                if (0xD800...0xDBFF).contains(unit) {
                    guard i + 1 < codeUnits.count, (0xDC00...0xDFFF).contains(codeUnits[i + 1]) else { return nil }
                    i += 2
                } else if (0xDC00...0xDFFF).contains(unit) { return nil }
                else { i += 1 }
            }
            return String(decoding: codeUnits, as: UTF16.self)
        }
        public var stringReplacingInvalidCodeUnits: String { String(decoding: codeUnits, as: UTF16.self) }
        public init(from decoder: any Decoder) throws { codeUnits = try decoder.singleValueContainer().decode([UInt16].self) }
        public func encode(to encoder: any Encoder) throws { var value = encoder.singleValueContainer(); try value.encode(codeUnits) }
    }
    /// Flat graph schema 1. References preserve aliases/cycles; numbers encode IEEE bits.
    public struct PublicGraph: Codable, Sendable {
        public enum Atom: Codable, Sendable {
            case undefined, null, bool(Bool), number(Double), string(UInt32), ref(UInt32)
            case builtinFunction(UInt32), hostReference(UInt32)
            private enum Keys: String, CodingKey { case kind, value }
            public init(from decoder: any Decoder) throws {
                let fields = try decoder.container(keyedBy: Keys.self)
                switch try fields.decode(String.self, forKey: .kind) {
                case "undefined": self = .undefined
                case "null": self = .null
                case "bool": self = .bool(try fields.decode(Bool.self, forKey: .value))
                case "number": self = .number(try decodeBits(fields.decode(String.self, forKey: .value), decoder))
                case "string": self = .string(try fields.decode(UInt32.self, forKey: .value))
                case "ref": self = .ref(try fields.decode(UInt32.self, forKey: .value))
                case "builtinFunction": self = .builtinFunction(try fields.decode(UInt32.self, forKey: .value))
                case "hostReference": self = .hostReference(try fields.decode(UInt32.self, forKey: .value))
                default: throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Unknown public atom kind"))
                }
            }
            public func encode(to encoder: any Encoder) throws {
                var fields = encoder.container(keyedBy: Keys.self)
                switch self {
                case .undefined: try fields.encode("undefined", forKey: .kind)
                case .null: try fields.encode("null", forKey: .kind)
                case .bool(let value): try fields.encode("bool", forKey: .kind); try fields.encode(value, forKey: .value)
                case .number(let value): try fields.encode("number", forKey: .kind); try fields.encode(encodeBits(value), forKey: .value)
                case .string(let id): try fields.encode("string", forKey: .kind); try fields.encode(id, forKey: .value)
                case .ref(let id): try fields.encode("ref", forKey: .kind); try fields.encode(id, forKey: .value)
                case .builtinFunction(let id): try fields.encode("builtinFunction", forKey: .kind); try fields.encode(id, forKey: .value)
                case .hostReference(let id): try fields.encode("hostReference", forKey: .kind); try fields.encode(id, forKey: .value)
                }
            }
        }
        public struct Property: Codable, Sendable {
            public var key: UInt32
            public var value: Atom
            /// Enumerable=1, writable=2, configurable=4.
            public var flags: UInt8
            public init(key: UInt32, value: Atom, flags: UInt8 = 7) { self.key = key; self.value = value; self.flags = flags }
        }
        public struct Entry: Codable, Sendable {
            public var key: Atom, value: Atom
            public init(key: Atom, value: Atom) { self.key = key; self.value = value }
            public init(from decoder: any Decoder) throws {
                var pair = try decoder.unkeyedContainer()
                key = try pair.decode(Atom.self); value = try pair.decode(Atom.self)
                guard pair.isAtEnd else { throw DecodingError.dataCorruptedError(in: pair, debugDescription: "Public Map entry must have two atoms") }
            }
            public func encode(to encoder: any Encoder) throws {
                var pair = encoder.unkeyedContainer(); try pair.encode(key); try pair.encode(value)
            }
        }
        public enum Node: Codable, Sendable {
            case error(errorType: ErrorObjectKind, properties: [Property])
            case instance(prototype: UInt32, properties: [Property])
            case object(properties: [Property])
            case array(length: UInt32, properties: [Property])
            case map(entries: [Entry], properties: [Property])
            case set(values: [Atom], properties: [Property])
            case date(milliseconds: Double, properties: [Property])
            public var properties: [Property] {
                get { switch self {
                case .error(_, let p), .instance(_, let p), .object(let p), .array(_, let p), .map(_, let p), .set(_, let p), .date(_, let p): return p
                } }
                set { switch self {
                case .error(let type, _): self = .error(errorType: type, properties: newValue)
                case .instance(let id, _): self = .instance(prototype: id, properties: newValue)
                case .object: self = .object(properties: newValue)
                case .array(let n, _): self = .array(length: n, properties: newValue)
                case .map(let entries, _): self = .map(entries: entries, properties: newValue)
                case .set(let values, _): self = .set(values: values, properties: newValue)
                case .date(let ms, _): self = .date(milliseconds: ms, properties: newValue)
                } }
            }
            private enum Keys: String, CodingKey { case kind, properties, length, entries, values, milliseconds, prototype, errorType }
            public init(from decoder: any Decoder) throws {
                let fields = try decoder.container(keyedBy: Keys.self)
                let properties = try fields.decode([Property].self, forKey: .properties)
                switch try fields.decode(String.self, forKey: .kind) {
                case "error": self = .error(errorType: try fields.decode(ErrorObjectKind.self, forKey: .errorType), properties: properties)
                case "instance": self = .instance(prototype: try fields.decode(UInt32.self, forKey: .prototype), properties: properties)
                case "object": self = .object(properties: properties)
                case "array": self = .array(length: try fields.decode(UInt32.self, forKey: .length), properties: properties)
                case "map": self = .map(entries: try fields.decode([Entry].self, forKey: .entries), properties: properties)
                case "set": self = .set(values: try fields.decode([Atom].self, forKey: .values), properties: properties)
                case "date": self = .date(milliseconds: try decodeBits(fields.decode(String.self, forKey: .milliseconds), decoder), properties: properties)
                default: throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Unknown public node kind"))
                }
            }
            public func encode(to encoder: any Encoder) throws {
                var fields = encoder.container(keyedBy: Keys.self)
                try fields.encode(properties, forKey: .properties)
                switch self {
                case .error(let type, _): try fields.encode("error", forKey: .kind); try fields.encode(type, forKey: .errorType)
                case .instance(let id, _): try fields.encode("instance", forKey: .kind); try fields.encode(id, forKey: .prototype)
                case .object: try fields.encode("object", forKey: .kind)
                case .array(let n, _): try fields.encode("array", forKey: .kind); try fields.encode(n, forKey: .length)
                case .map(let entries, _): try fields.encode("map", forKey: .kind); try fields.encode(entries, forKey: .entries)
                case .set(let values, _): try fields.encode("set", forKey: .kind); try fields.encode(values, forKey: .values)
                case .date(let ms, _): try fields.encode("date", forKey: .kind); try fields.encode(encodeBits(ms), forKey: .milliseconds)
                }
            }
        }
        public var version: UInt32, root: Atom, strings: [JSString], nodes: [Node]
        public init(version: UInt32 = 1, root: Atom, strings: [JSString], nodes: [Node]) {
            self.version = version; self.root = root; self.strings = strings; self.nodes = nodes
        }
        public mutating func intern(_ string: JSString) throws -> UInt32 {
            if let id = strings.firstIndex(of: string) { return UInt32(id) }
            guard let id = UInt32(exactly: strings.count) else { throw PeleError(code: "limit", message: "Public string table exhausted") }
            strings.append(string); return id
        }
        public mutating func intern(_ string: String) throws -> UInt32 { try intern(JSString(string)) }
        public func field(_ name: String, on object: UInt32? = nil) throws -> Atom? {
            let id = try objectID(object)
            for property in nodes[Int(id)].properties {
                guard strings.indices.contains(Int(property.key)) else { throw invalidGraph() }
                if strings[Int(property.key)] == JSString(name) { return property.value }
            }
            return nil
        }
        public mutating func setField(_ name: String, value: Atom, on object: UInt32? = nil) throws {
            let id = try objectID(object)
            var properties = nodes[Int(id)].properties
            for i in properties.indices {
                guard strings.indices.contains(Int(properties[i].key)) else { throw invalidGraph() }
                if strings[Int(properties[i].key)] == JSString(name) {
                    properties[i].value = value; nodes[Int(id)].properties = properties; return
                }
            }
            properties.append(Property(key: try intern(name), value: value)); nodes[Int(id)].properties = properties
        }
        private func objectID(_ object: UInt32?) throws -> UInt32 {
            let id: UInt32
            if let object { id = object }
            else if case .ref(let root) = root { id = root }
            else { throw invalidGraph() }
            guard nodes.indices.contains(Int(id)) else { throw invalidGraph() }; return id
        }
    }
    public struct PublicSnapshot: Codable, Sendable {
        public let type: String
        public let revision: UInt64
        public var graph: PublicGraph
        init(type: String, revision: UInt64, graph: PublicGraph) { self.type = type; self.revision = revision; self.graph = graph }
        private enum Keys: String, CodingKey { case type, revision, graph }
        public init(from decoder: any Decoder) throws {
            let fields = try decoder.container(keyedBy: Keys.self)
            type = try fields.decode(String.self, forKey: .type)
            revision = try decodeRevision(fields.decode(String.self, forKey: .revision), decoder)
            graph = try fields.decode(PublicGraph.self, forKey: .graph)
        }
        public func encode(to encoder: any Encoder) throws {
            var fields = encoder.container(keyedBy: Keys.self)
            try fields.encode(type, forKey: .type); try fields.encode(String(revision), forKey: .revision); try fields.encode(graph, forKey: .graph)
        }
    }
}
private func invalidGraph() -> PeleError { PeleError(code: "semantic", message: "Invalid public graph reference") }
private func encodeBits(_ value: Double) -> String { String(format: "%016llx", value.bitPattern) }
private func decodeBits(_ hex: String, _ decoder: any Decoder) throws -> Double {
    guard hex.utf8.count == 16, let bits = UInt64(hex, radix: 16) else {
        throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Invalid IEEE public number bits"))
    }
    return Double(bitPattern: bits)
}
private func decodeRevision(_ value: String, _ decoder: any Decoder) throws -> UInt64 {
    guard let revision = UInt64(value), String(revision) == value else {
        throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Invalid public revision"))
    }
    return revision
}
