import Foundation
import CryptoKit
import Pele

private let stages = ["parseMutable", "snapshot", "apply", "render", "lifetime"]
private func invalid(_ message: String) -> PeleError { PeleError(code: "benchmark", message: message) }
private struct Workload: Decodable {
    let id: String
    let family: String
    let scale: String
    let units: Int
    let source: String
    let options: [String: ConfigValue]
}
private struct Request: Decodable {
    let op: String
    let workload: Workload?
    let caseID: String?
    let stage: String?
    let count: Int?
}
private struct Output: Encodable {
    let type: String
    let widthBits: String
    let heightBits: String
    let svgSha256: String
    let svgBytes: Int
    init(_ result: FullRenderedDiagram) {
        type = result.type
        widthBits = String(format: "%016llx", result.width.bitPattern)
        heightBits = String(format: "%016llx", result.height.bitPattern)
        let data = Data(result.svg.utf8)
        svgSha256 = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
        svgBytes = data.count
    }
}
private struct Verification: Encodable {
    let caseID: String
    let outputs: [Output]
    let arenaNodes: Int
    let arenaStrings: Int
}
private struct Measurement: Encodable {
    let caseID: String
    let stage: String
    let count: Int
    let elapsedNs: UInt64?
    let checksum: String
    let arenaBefore: Int?
    let arenaAfter: Int?
    let output: Output
}
private struct Envelope<T: Encodable>: Encodable {
    let ok: Bool
    let value: T?
    let error: String?
}
private final class Prepared {
    let workload: Workload
    let parseOptions: Pele.Engine.ParseOptions
    let options: Pele.Engine.Options
    let resources: Pele.HostResources
    let engine = Pele.Engine()
    init(_ workload: Workload) throws {
        guard workload.units > 0 else { throw invalid("Invalid work units") }
        self.workload = workload
        var fields = workload.options
        if fields["config"] == nil { fields["config"] = .object([:]) }
        let decodedOptions = try JSONDecoder().decode(Pele.Engine.Options.self, from: JSONEncoder().encode(fields))
        options = decodedOptions
        var parse = Pele.Engine.ParseOptions()
        parse.limit = decodedOptions.limit ?? 50_000; parse.maxEdges = decodedOptions.maxEdges ?? 5_000; parse.config = decodedOptions.config
        parseOptions = parse
        var host = Pele.HostResources()
        guard let utc = TimeZone(secondsFromGMT: 0) else { throw invalid("UTC unavailable") }
        host.timeZone = utc
        let now = decodedOptions.now ?? 1_704_067_200_000
        host.nowMillis = { now }
        host.textWidth = { request in Double(request.text.utf16.count) * request.size * (request.style & 1 != 0 ? 0.71 : 0.6) }
        resources = host
    }
    func document() throws -> Pele.MutableDocument { try engine.parseMutable(workload.source, options: parseOptions) }
    var maximumBatch: Int { max(1, min(128, 25_000 / workload.units)) }
    func warmDocument() throws -> Pele.MutableDocument {
        let document = try document()
        do { _ = try document.render(options: options, resources: resources); return document }
        catch { try? document.close(); throw error }
    }
    func verify() throws -> Verification {
        let document = try document()
        defer { try? document.close() }
        let initial = try document.render(options: options, resources: resources)
        var snapshot = try document.snapshot()
        let raw = try snapshot.graph.intern(Pele.JSString(codeUnits: [0xd800, 65, 0xdc00]))
        try snapshot.graph.setField("__swiftPerfValue", value: .string(raw))
        try snapshot.graph.setField("__swiftPerfSelf", value: snapshot.graph.root)
        try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
        let applied = try document.render(options: options, resources: resources)
        let saved = try document.snapshot().graph
        guard case .string(let text) = try saved.field("__swiftPerfValue"), saved.strings[Int(text)].codeUnits == [0xd800, 65, 0xdc00],
              case .ref(let reference) = try saved.field("__swiftPerfSelf"), case .ref(let root) = saved.root, reference == root else {
            throw invalid("Owned graph lost raw UTF16 or reference identity")
        }
        let rerendered = try document.render(options: options, resources: resources)
        return Verification(caseID: workload.id, outputs: [Output(initial), Output(applied), Output(rerendered)], arenaNodes: saved.nodes.count, arenaStrings: saved.strings.count)
    }
    private func consume(_ result: FullRenderedDiagram) -> UInt64 {
        UInt64(result.svg.utf8.count) &+ result.width.bitPattern &+ result.height.bitPattern
    }
    private func parseOnce() throws -> UInt64 {
        let document = try document()
        defer { try? document.close() }
        return UInt64(document.type.utf8.count)
    }
    private func lifecycleOnce() throws -> UInt64 {
        let document = try document()
        defer { try? document.close() }
        let snapshot = try document.snapshot()
        try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
        let result = try document.render(options: options, resources: resources)
        return consume(result) &+ UInt64(snapshot.graph.nodes.count)
    }
    func exercise(stage: String, count: Int, timing: Bool) throws -> Measurement {
        guard stages.contains(stage), count > 0, count <= maximumBatch else { throw invalid("Invalid stage or batch count") }
        guard !timing || ProcessInfo.processInfo.environment["PELE_PERF_ALLOW_TIMING"] == "1" else { throw invalid("Timing requires coordinated --quiet-run") }
        let retained = ["snapshot", "apply", "render"].contains(stage) ? try warmDocument() : nil
        defer { try? retained?.close() }
        let baseline = stage == "apply" ? try retained!.snapshot().graph : nil
        let arenaBefore = try retained?.snapshot().graph.nodes.count
        var checksum: UInt64 = 0
        let start = timing ? DispatchTime.now().uptimeNanoseconds : 0
        for _ in 0..<count {
            let contribution: UInt64 = try autoreleasepool {
                switch stage {
                case "parseMutable": return try parseOnce()
                case "snapshot":
                    let snapshot = try retained!.snapshot()
                    return UInt64(snapshot.graph.nodes.count) &+ UInt64(snapshot.graph.strings.count) &+ snapshot.revision
                case "apply": return try retained!.apply(baseline!, expectedRevision: retained!.revision)
                case "render": return consume(try retained!.render(options: options, resources: resources))
                case "lifetime": return try lifecycleOnce()
                default: throw invalid("Unknown stage")
                }
            }
            checksum &+= contribution
        }
        let elapsed = timing ? DispatchTime.now().uptimeNanoseconds - start : nil
        let arenaAfter = try retained?.snapshot().graph.nodes.count
        // Output hashing and verification rendering never enter the timer.
        let verificationOwner = try retained ?? document()
        defer { if retained == nil { try? verificationOwner.close() } }
        let output = Output(try verificationOwner.render(options: options, resources: resources))
        return Measurement(caseID: workload.id, stage: stage, count: count, elapsedNs: elapsed, checksum: String(checksum), arenaBefore: arenaBefore, arenaAfter: arenaAfter, output: output)
    }
}

private var prepared: [String: Prepared] = [:]
private let decoder = JSONDecoder(), encoder = JSONEncoder()
while let line = readLine() {
    do {
        let request = try decoder.decode(Request.self, from: Data(line.utf8))
        let response: Data
        if request.op == "prepare" {
            guard let workload = request.workload else { throw invalid("Missing workload") }
            let value = try Prepared(workload)
            let verification = try value.verify()
            prepared[workload.id] = value
            response = try encoder.encode(Envelope(ok: true, value: verification, error: nil))
        } else {
            guard let id = request.caseID, let value = prepared[id] else { throw invalid("Unknown prepared case") }
            switch request.op {
            case "verify": response = try encoder.encode(Envelope(ok: true, value: try value.verify(), error: nil))
            case "exercise", "measure":
                guard let stage = request.stage else { throw invalid("Missing stage") }
                response = try encoder.encode(Envelope(ok: true, value: try value.exercise(stage: stage, count: request.count ?? 1, timing: request.op == "measure"), error: nil))
            default: throw invalid("Unknown operation")
            }
        }
        FileHandle.standardOutput.write(response); FileHandle.standardOutput.write(Data([10]))
    } catch {
        let response = try encoder.encode(Envelope<String>(ok: false, value: nil, error: String(describing: error)))
        FileHandle.standardOutput.write(response); FileHandle.standardOutput.write(Data([10]))
    }
}
