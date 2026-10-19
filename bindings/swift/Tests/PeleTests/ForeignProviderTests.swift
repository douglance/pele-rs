import Foundation
import XCTest
import CPele

// Prototype consumer uses only exported C entry points. No @testable or engine internals.
private final class ProviderMarker: Error {}
private struct ProviderFailure: Error { let message: String }
private final class ProviderModel {
    var width = 40.0
    var renders = 0
    let released: () -> Void
    init(released: @escaping () -> Void = {}) { self.released = released }
    deinit { released() }
}
private final class ProviderBox {
    var keepComments = false
    var section: String? = "pie"
    var generation = "A"
    var measureText = false
    var measuredWidth = 0.0
    var parseCount = 0
    var renderCount = 0
    var seenSource = ""
    var seenTitle: String?
    var seenOffset: UInt64 = 0
    var seenEdges = 0.0
    var seenOptions: [String: Any] = [:]
    var parseBody: () throws -> ProviderModel = { ProviderModel() }
    var renderBody: ((ProviderModel) throws -> Void)?
    var onRelease: () -> Void = {}
    deinit { onRelease() }
}
private final class ProviderInvocation {
    var error: (any Error)?
    func fail(_ error: any Error, reply: UnsafeMutablePointer<PeleForeignReplyV10>) {
        self.error = error; reply.pointee.status = 2; reply.pointee.exception = 1
    }
}
private func providerBytes(_ pointer: UnsafePointer<UInt8>?, _ count: Int) -> Data {
    guard count > 0, let pointer else { return Data() }; return Data(bytes: pointer, count: count)
}
private func providerText(_ pointer: UnsafePointer<UInt8>?, _ count: Int) -> String {
    String(decoding: providerBytes(pointer, count), as: UTF8.self)
}
private func releaseProviderBytes(_ pointer: UnsafeMutableRawPointer?) {
    if let pointer { _ = Unmanaged<NSData>.fromOpaque(pointer).takeRetainedValue() }
}
private func ownedProviderBytes(_ data: Data) -> PeleForeignBytesV10 {
    let storage = data as NSData
    return PeleForeignBytesV10(owner: Unmanaged.passRetained(storage).toOpaque(),
        data: storage.bytes.assumingMemoryBound(to: UInt8.self), len: storage.length, release: releaseProviderBytes)
}
private func retainProvider(_ pointer: UnsafeMutableRawPointer?) {
    if let pointer { _ = Unmanaged<ProviderBox>.fromOpaque(pointer).retain() }
}
private func releaseProvider(_ pointer: UnsafeMutableRawPointer?) {
    if let pointer { Unmanaged<ProviderBox>.fromOpaque(pointer).release() }
}
private func releaseProviderModel(_ context: UnsafeMutableRawPointer?, _ pointer: UnsafeMutableRawPointer?) {
    if let pointer { Unmanaged<ProviderModel>.fromOpaque(pointer).release() }
}
private func providerInvocation(_ call: OpaquePointer?) -> ProviderInvocation {
    Unmanaged<ProviderInvocation>.fromOpaque(pele_foreign_call_context_v10(call)!).takeUnretainedValue()
}
private func describeProvider(_ context: UnsafeMutableRawPointer?, _ call: OpaquePointer?, _ phase: UInt32,
                              _ reply: UnsafeMutablePointer<PeleForeignReplyV10>?) {
    guard let context, let reply else { return }
    let provider = Unmanaged<ProviderBox>.fromOpaque(context).takeUnretainedValue()
    do {
        var fields: [String: Any] = ["keepComments": provider.keepComments]
        if let section = provider.section { fields["section"] = section }
        reply.pointee.payload = ownedProviderBytes(try JSONSerialization.data(withJSONObject: fields))
    } catch { providerInvocation(call).fail(error, reply: reply) }
}
private func parseProvider(_ context: UnsafeMutableRawPointer?, _ call: OpaquePointer?,
                           _ request: UnsafePointer<PeleProviderParseV10>?, _ reply: UnsafeMutablePointer<PeleForeignReplyV10>?) {
    guard let context, let request, let reply else { return }
    let provider = Unmanaged<ProviderBox>.fromOpaque(context).takeUnretainedValue()
    provider.parseCount += 1
    provider.seenSource = providerText(request.pointee.source, request.pointee.source_len)
    provider.seenTitle = request.pointee.has_title == 0 ? nil : providerText(request.pointee.title, request.pointee.title_len)
    provider.seenOffset = request.pointee.line_offset
    provider.seenEdges = Double(bitPattern: request.pointee.edge_bits)
    do { reply.pointee.model = Unmanaged.passRetained(try provider.parseBody()).toOpaque() }
    catch { providerInvocation(call).fail(error, reply: reply) }
}
private func renderProvider(_ context: UnsafeMutableRawPointer?, _ pointer: UnsafeMutableRawPointer?, _ call: OpaquePointer?,
                            _ request: UnsafePointer<PeleProviderRenderV10>?, _ reply: UnsafeMutablePointer<PeleForeignReplyV10>?) {
    guard let context, let pointer, let request, let reply else { return }
    let provider = Unmanaged<ProviderBox>.fromOpaque(context).takeUnretainedValue()
    let model = Unmanaged<ProviderModel>.fromOpaque(pointer).takeUnretainedValue()
    provider.renderCount += 1; model.renders += 1
    do {
        provider.seenOptions = try JSONSerialization.jsonObject(with: providerBytes(request.pointee.options, request.pointee.options_len)) as! [String: Any]
        if provider.measureText {
            let text = Data("Provider width".utf8)
            var bits: UInt64 = 0
            _ = try text.withUnsafeBytes { bytes in
                try providerResult(pele_foreign_text_width_v10(call, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count, 13.0.bitPattern, 0, &bits))
            }
            provider.measuredWidth = Double(bitPattern: bits)
        }
        try provider.renderBody?(model)
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"\(model.width)\" height=\"12\"><rect data-provider=\"\(provider.generation)\" width=\"\(model.width)\" height=\"12\"/></svg>"
        reply.pointee.payload = ownedProviderBytes(Data(svg.utf8))
        reply.pointee.width_bits = model.width.bitPattern; reply.pointee.height_bits = 12.0.bitPattern
    } catch { providerInvocation(call).fail(error, reply: reply) }
}
private func providerResult(_ result: OpaquePointer?, invocation: ProviderInvocation? = nil) throws -> [String: Any] {
    guard let result else { throw ProviderFailure(message: "Missing result") }
    defer { pele_result_free(result) }
    let envelope = try JSONSerialization.jsonObject(with: providerBytes(pele_result_data(result), pele_result_len(result))) as! [String: Any]
    guard envelope["ok"] as? Bool == true else {
        throw ProviderFailure(message: (envelope["error"] as? [String: Any])?["message"] as? String ?? "Native failure")
    }
    let value = envelope["result"]
    if let dictionary = value as? [String: Any] {
        if let operation = dictionary["result"] as? [String: Any], let error = operation["Err"] as? [String: Any] {
            if error["kind"] as? String == "hostException", let original = invocation?.error { throw original }
            throw ProviderFailure(message: (error["value"] as? [String: Any])?["message"] as? String ?? "Provider operation failure")
        }
        return dictionary
    }
    return [:]
}
private final class PrototypeRegistry {
    var handle: OpaquePointer?
    init() throws { _ = try providerResult(pele_registry_create_v8(0, &handle)) }
    deinit { close() }
    func close() { if let handle { pele_registry_free_v8(handle); self.handle = nil } }
    func register(_ provider: ProviderBox) throws {
        var table = PeleDiagramProviderV10()
        table.version = 10; table.struct_size = UInt32(MemoryLayout<PeleDiagramProviderV10>.size)
        table.context = Unmanaged.passUnretained(provider).toOpaque()
        table.retain = retainProvider; table.release = releaseProvider; table.release_model = releaseProviderModel
        table.describe = describeProvider; table.parse = parseProvider; table.render = renderProvider
        let id = Data("pie".utf8)
        _ = try id.withUnsafeBytes { bytes in
            try providerResult(pele_registry_register_provider_v10(handle, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count, &table))
        }
    }
    func parse(_ source: String, sourceLimit: Double? = nil, edges: Double? = nil) throws -> PrototypeDocument {
        let input = Data(source.utf8), invocation = ProviderInvocation()
        var document: OpaquePointer?
        let result = input.withUnsafeBytes { bytes in
            pele_registry_parse_provider_v10(handle, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count,
                nil, 0, sourceLimit == nil ? 0 : 2, sourceLimit?.bitPattern ?? 0,
                edges == nil ? 0 : 2, edges?.bitPattern ?? 0, Unmanaged.passUnretained(invocation).toOpaque(), &document)
        }
        do {
            _ = try withExtendedLifetime(invocation) { try providerResult(result, invocation: invocation) }
            guard let document else { throw ProviderFailure(message: "Missing document") }
            return PrototypeDocument(document)
        } catch { if let document { pele_provider_document_free_v10(document) }; throw error }
    }
}
private final class PrototypeDocument {
    var handle: OpaquePointer?
    var revision: UInt64 = 0
    init(_ handle: OpaquePointer) { self.handle = handle }
    deinit { close() }
    func close() { if let handle { pele_provider_document_free_v10(handle); self.handle = nil } }
    func model() throws -> ProviderModel {
        var model: UnsafeMutableRawPointer?
        _ = try providerResult(pele_provider_model_v10(handle, &model))
        guard let model else { throw ProviderFailure(message: "Missing model") }
        return Unmanaged<ProviderModel>.fromOpaque(model).takeUnretainedValue()
    }
    func status() throws -> UInt64 {
        let result = try providerResult(pele_provider_status_v10(handle))
        return UInt64(result["revision"] as! String)!
    }
    func render(outputLimit: Double? = nil, options: [String: Any] = [:]) throws -> [String: Any] {
        let data = try JSONSerialization.data(withJSONObject: options), invocation = ProviderInvocation()
        let result = data.withUnsafeBytes { bytes in
            pele_provider_render_v10(handle, revision, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count,
                nil, outputLimit == nil ? 0 : 2, outputLimit?.bitPattern ?? 0, nil, Unmanaged.passUnretained(invocation).toOpaque())
        }
        do {
            let decoded = try withExtendedLifetime(invocation) { try providerResult(result, invocation: invocation) }
            revision = UInt64(decoded["revision"] as! String)!
            return (decoded["result"] as! [String: Any])["Ok"] as! [String: Any]
        } catch { revision = (try? status()) ?? revision; throw error }
    }
}

final class ForeignProviderTests: XCTestCase {
    private let source = "---\ntitle: Title\nconfig:\n  pie:\n    useMaxWidth: false\n---\npie\n%% preserve this\n\"A\":1"
    func testSwiftAuthoredProviderPreservesIdentityReplacementLiveEditsAndOwnership() throws {
        let registry = try PrototypeRegistry()
        var a: ProviderBox? = ProviderBox(), b: ProviderBox? = ProviderBox()
        weak var weakA = a; weak var weakB = b
        a!.keepComments = true; a!.measureText = true; b!.generation = "B"
        try registry.register(a!)
        let first = try registry.parse(source), model = try first.model()
        XCTAssertTrue(model === (try first.model()))
        XCTAssertTrue(a!.seenSource.contains("%% preserve this")); XCTAssertTrue(a!.seenSource.hasSuffix("\n"))
        XCTAssertEqual(a!.seenTitle, "Title"); XCTAssertGreaterThan(a!.seenOffset, 0); XCTAssertEqual(a!.seenEdges, 5000)
        model.width = 91
        XCTAssertEqual(try first.render()["width"] as? Double, 91)
        XCTAssertEqual(a!.seenOptions["responsive"] as? Bool, false)
        XCTAssertGreaterThan(a!.measuredWidth, 0)
        try registry.register(b!); let second = try registry.parse(source)
        a!.generation = "A-live"; a!.section = "other"
        a!.renderBody = { $0.width += 1 }
        XCTAssertTrue((try first.render()["svg"] as! String).contains("A-live"))
        XCTAssertEqual(a!.seenOptions["responsive"] as? Bool, true)
        XCTAssertTrue((try second.render()["svg"] as! String).contains("B"))
        XCTAssertFalse(b!.seenSource.contains("%% preserve this"))
        a = nil; b = nil; registry.close()
        XCTAssertNotNil(weakA); XCTAssertNotNil(weakB)
        _ = try first.render(); _ = try second.render()
        second.close(); XCTAssertNil(weakB)
        first.close(); XCTAssertNil(weakA)
        XCTAssertEqual(model.width, 93) // User-owned model survives document closure.
    }
    func testOriginalSwiftErrorPartialMutationLimitsRecoveryAndBusyReentry() throws {
        let registry = try PrototypeRegistry(), provider = ProviderBox(), marker = ProviderMarker()
        try registry.register(provider)
        let document = try registry.parse(source), model = try document.model()
        defer { provider.renderBody = nil; document.close(); registry.close() }
        provider.renderBody = { _ in throw marker }
        do { _ = try document.render(outputLimit: 0); XCTFail("Expected original callback error") }
        catch { XCTAssertTrue(error as AnyObject === marker) }
        XCTAssertEqual(model.renders, 1); XCTAssertEqual(document.revision, 1)
        provider.renderBody = nil
        XCTAssertThrowsError(try document.render(outputLimit: 0)) { XCTAssertTrue(($0 as? ProviderFailure)?.message.contains("Diagram output is longer") == true) }
        XCTAssertEqual(model.renders, 2); XCTAssertEqual(document.revision, 2)
        provider.renderBody = { _ in
            XCTAssertThrowsError(try document.model()) { XCTAssertTrue(($0 as? ProviderFailure)?.message.contains("already in use") == true) }
        }
        _ = try document.render(outputLimit: .infinity, options: ["limit": -1, "maxEdges": -1])
        XCTAssertEqual(provider.seenOptions["limit"] as? Int, -1)
        provider.renderBody = nil
        let before = provider.parseCount
        XCTAssertThrowsError(try registry.parse(source, sourceLimit: -1))
        XCTAssertEqual(provider.parseCount, before)
        provider.parseBody = { throw marker }
        do { _ = try registry.parse(source); XCTFail("Expected parse error") }
        catch { XCTAssertTrue(error as AnyObject === marker) }
        provider.parseBody = { ProviderModel() }
        let recovered = try registry.parse(source, edges: .nan)
        XCTAssertTrue(provider.seenEdges.isNaN); _ = try recovered.render()
    }
}
