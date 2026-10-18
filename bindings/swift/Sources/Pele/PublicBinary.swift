import Foundation
import CPele

private let binaryBudget = 64 * 1024 * 1024
private func invalidBinary() -> PeleError { PeleError(code: "semantic", message: "Invalid public binary graph transport") }
private func binaryLimit() -> PeleError { PeleError(code: "limit", message: "Public binary graph exceeds transport budget") }
private struct BinaryWriter {
    var data = Data()
    mutating func integer<T: FixedWidthInteger>(_ value: T) throws {
        guard data.count <= binaryBudget - MemoryLayout<T>.size else { throw binaryLimit() }
        var little = value.littleEndian
        withUnsafeBytes(of: &little) { data.append(contentsOf: $0) }
    }
    mutating func count(_ n: Int) throws { guard let value = UInt32(exactly: n) else { throw binaryLimit() }; try integer(value) }
    mutating func atom(_ value: Pele.PublicGraph.Atom) throws {
        switch value {
        case .undefined: try integer(UInt8(0))
        case .null: try integer(UInt8(1))
        case .bool(let value): try integer(UInt8(value ? 3 : 2))
        case .number(let value): try integer(UInt8(4)); try integer(value.bitPattern)
        case .string(let id): try integer(UInt8(5)); try integer(id)
        case .ref(let id): try integer(UInt8(6)); try integer(id)
        case .builtinFunction(let id): try integer(UInt8(7)); try integer(id)
        case .hostReference(let id): try integer(UInt8(8)); try integer(id)
        }
    }
    mutating func properties(_ p: [Pele.PublicGraph.Property]) throws {
        try count(p.count)
        for v in p { try integer(v.key); try integer(v.flags); try atom(v.value) }
    }
}
extension Pele.PublicGraph {
    func binaryBytes(revision: UInt64) throws -> Data {
        guard version == 1 else { throw invalidBinary() }
        var w = BinaryWriter()
        for byte in "PELGBIN1".utf8 { try w.integer(byte) }
        try w.integer(version); try w.integer(revision); try w.integer(UInt32(0))
        try w.atom(root); try w.count(strings.count)
        for s in strings { try w.count(s.codeUnits.count); for unit in s.codeUnits { try w.integer(unit) } }
        try w.count(nodes.count)
        for n in nodes {
            switch n {
            case .error(let kind, _): try w.integer(UInt8(6)); try w.integer(UInt8(Pele.ErrorObjectKind.allCases.firstIndex(of: kind)!))
            case .object: try w.integer(UInt8(0))
            case .instance(let id, _): try w.integer(UInt8(1)); try w.integer(id)
            case .array(let count, _): try w.integer(UInt8(2)); try w.integer(count)
            case .map(let entries, _): try w.integer(UInt8(3)); try w.count(entries.count); for entry in entries { try w.atom(entry.key); try w.atom(entry.value) }
            case .set(let values, _): try w.integer(UInt8(4)); try w.count(values.count); for value in values { try w.atom(value) }
            case .date(let time, _): try w.integer(UInt8(5)); try w.integer(time.bitPattern)
            }
            try w.properties(n.properties)
        }
        return w.data
    }
}
struct PublicBinaryExchange {
    let revision: UInt64
    let graph: Pele.PublicGraph
    let metadata: Data
}
private struct BinaryReader {
    let data: Data
    var offset = 0
    var end: Int
    var work = 0
    var units = 0
    mutating func integer<T: FixedWidthInteger>(_ type: T.Type) throws -> T {
        let width = MemoryLayout<T>.size
        guard offset <= end, width <= end - offset else { throw invalidBinary() }
        let value = data.withUnsafeBytes { $0.loadUnaligned(fromByteOffset: offset, as: T.self) }
        offset += width; return T(littleEndian: value)
    }
    mutating func count(_ maximum: Int, _ minimum: Int) throws -> Int {
        let n = Int(try integer(UInt32.self))
        guard n <= maximum else { throw binaryLimit() }
        guard offset <= end, n <= (end - offset) / minimum else { throw invalidBinary() }
        return n
    }
    mutating func addWork(_ n: Int) throws { guard n <= 8_000_000 - work else { throw binaryLimit() }; work += n }
    mutating func atom() throws -> Pele.PublicGraph.Atom {
        switch try integer(UInt8.self) {
        case 0: return .undefined
        case 1: return .null
        case 2: return .bool(false)
        case 3: return .bool(true)
        case 4: return .number(Double(bitPattern: try integer(UInt64.self)))
        case 5: return .string(try integer(UInt32.self))
        case 6: return .ref(try integer(UInt32.self))
        case 7: return .builtinFunction(try integer(UInt32.self))
        case 8: return .hostReference(try integer(UInt32.self))
        default: throw invalidBinary()
        }
    }
    mutating func properties() throws -> [Pele.PublicGraph.Property] {
        let n = try count(8_000_000, 6); try addWork(n)
        var p: [Pele.PublicGraph.Property] = []; p.reserveCapacity(n)
        for _ in 0..<n {
            let key = try integer(UInt32.self), flags = try integer(UInt8.self)
            guard flags & 0xf8 == 0 else { throw invalidBinary() }
            p.append(.init(key: key, value: try atom(), flags: flags))
        }
        return p
    }
    mutating func node() throws -> Pele.PublicGraph.Node {
        var n: Pele.PublicGraph.Node
        switch try integer(UInt8.self) {
        case 6:
            let tag = Int(try integer(UInt8.self))
            guard Pele.ErrorObjectKind.allCases.indices.contains(tag) else { throw invalidBinary() }
            n = .error(errorType: Pele.ErrorObjectKind.allCases[tag], properties: [])
        case 0: n = .object(properties: [])
        case 1: n = .instance(prototype: try integer(UInt32.self), properties: [])
        case 2: n = .array(length: try integer(UInt32.self), properties: [])
        case 3:
            let count = try count(4_000_000, 2); try addWork(count * 2)
            var entries: [Pele.PublicGraph.Entry] = []; entries.reserveCapacity(count)
            for _ in 0..<count { entries.append(.init(key: try atom(), value: try atom())) }
            n = .map(entries: entries, properties: [])
        case 4:
            let count = try count(8_000_000, 1); try addWork(count)
            var values: [Pele.PublicGraph.Atom] = []; values.reserveCapacity(count)
            for _ in 0..<count { values.append(try atom()) }
            n = .set(values: values, properties: [])
        case 5: n = .date(milliseconds: Double(bitPattern: try integer(UInt64.self)), properties: [])
        default: throw invalidBinary()
        }
        n.properties = try properties(); return n
    }
    mutating func decode() throws -> PublicBinaryExchange {
        for byte in "PELGBIN1".utf8 { guard try integer(UInt8.self) == byte else { throw invalidBinary() } }
        let version = try integer(UInt32.self)
        guard version == 1 else { throw invalidBinary() }
        let revision = try integer(UInt64.self), metadataLength = Int(try integer(UInt32.self))
        guard metadataLength <= end - offset else { throw invalidBinary() }
        end -= metadataLength
        let root = try atom(), stringCount = try count(1_000_000, 4)
        var strings: [Pele.JSString] = []; strings.reserveCapacity(stringCount)
        for _ in 0..<stringCount {
            let length = try count(16_000_000, 2)
            guard length <= 16_000_000 - units else { throw binaryLimit() }; units += length
            var values: [UInt16] = []; values.reserveCapacity(length)
            for _ in 0..<length { values.append(try integer(UInt16.self)) }
            strings.append(.init(codeUnits: values))
        }
        let nodeCount = try count(1_000_000, 5)
        var nodes: [Pele.PublicGraph.Node] = []; nodes.reserveCapacity(nodeCount)
        for _ in 0..<nodeCount { nodes.append(try node()) }
        guard offset == end else { throw invalidBinary() }
        func check(_ value: Pele.PublicGraph.Atom) throws {
            switch value {
            case .ref(let i): guard i < nodes.count else { throw invalidBinary() }
            case .string(let i): guard i < strings.count else { throw invalidBinary() }
            default: break
            }
        }
        try check(root)
        for node in nodes {
            for p in node.properties { guard p.key < strings.count else { throw invalidBinary() }; try check(p.value) }
            switch node {
            case .map(let entries, _): for entry in entries { try check(entry.key); try check(entry.value) }
            case .set(let values, _): for value in values { try check(value) }
            default: break
            }
        }
        return .init(revision: revision, graph: .init(version: version, root: root, strings: strings, nodes: nodes), metadata: Data(data[end...]))
    }
}
extension Pele {
    static func binaryResult(_ handle: OpaquePointer?) throws -> PublicBinaryExchange {
        guard let handle else { throw PeleError(code: "internal", message: "Native result allocation failed") }
        defer { pele_result_free(handle) }
        if pele_result_status(handle) != 0 {
            let _: ConfigValue = try compatibilityResult(handle, as: ConfigValue.self, release: false)
            throw invalidBinary()
        }
        let count = pele_result_len(handle)
        guard count <= binaryBudget, let pointer = pele_result_data(handle) else { throw binaryLimit() }
        var reader = BinaryReader(data: Data(bytes: pointer, count: count), end: count)
        return try reader.decode()
    }
}
