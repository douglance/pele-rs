import Foundation
import CPele

private struct PublicDocumentMetadata: Decodable {
    let type: String
    let revision: String
}
private struct AppliedRevision: Decodable { let revision: String }
extension Pele {
    /// Owned mutable Rust model. Confine to one owner/actor; this class is not Sendable.
    /// Binary graph crossings are bounded to 64 MiB. Source convenience accepts Swift String;
    /// graph strings preserve isolated UTF-16 units, whose rendering remains codec-dependent.
    public final class MutableDocument {
        private var handle: OpaquePointer?
        private var active = false
        public let type: String
        public private(set) var revision: UInt64
        fileprivate init(handle: OpaquePointer, type: String, revision: UInt64) {
            self.handle = handle; self.type = type; self.revision = revision
        }
        deinit { if let handle { pele_public_document_free_v4(handle) } }
        public func close() throws {
            guard !active else { throw PeleError(code: "semantic", message: "Mutable document is already in use") }
            if let handle { pele_public_document_free_v4(handle); self.handle = nil }
        }
        private func withHandle<T>(_ operation: (OpaquePointer) throws -> T) throws -> T {
            guard let handle else { throw PeleError(code: "semantic", message: "Mutable document is closed") }
            guard !active else { throw PeleError(code: "semantic", message: "Mutable document is already in use") }
            active = true; defer { active = false }
            return try withExtendedLifetime(self) { try operation(handle) }
        }
        public func snapshot() throws -> PublicSnapshot {
            try withHandle { handle in
                struct Metadata: Decodable { let type: String }
                let exchange = try Pele.binaryResult(pele_public_snapshot_binary_v5(handle))
                let metadata = try JSONDecoder().decode(Metadata.self, from: exchange.metadata)
                let snapshot = PublicSnapshot(type: metadata.type, revision: exchange.revision, graph: exchange.graph)
                guard snapshot.type == type else { throw PeleError(code: "internal", message: "Mutable document type changed") }
                revision = snapshot.revision; return snapshot
            }
        }
        /// Explicit full-graph edit boundary. Stale revisions leave the native state unchanged.
        @discardableResult
        public func apply(_ graph: PublicGraph, expectedRevision: UInt64) throws -> UInt64 {
            let bytes = try graph.binaryBytes(revision: expectedRevision)
            guard bytes.count <= 64 * 1024 * 1024 else { throw PeleError(code: "limit", message: "Public binary graph exceeds transport budget") }
            return try withHandle { handle in
                let result = bytes.withUnsafeBytes { input in
                    pele_public_apply_binary_v5(handle, input.bindMemory(to: UInt8.self).baseAddress, input.count)
                }
                let applied = try Pele.compatibilityResult(result, as: AppliedRevision.self)
                guard let next = UInt64(applied.revision), String(next) == applied.revision else {
                    throw PeleError(code: "internal", message: "Invalid mutable revision")
                }
                revision = next; return next
            }
        }
        public func schema() throws -> PublicModelSchema {
            try withHandle { try Pele.compatibilityResult(pele_public_schema_v4($0), as: PublicModelSchema.self) }
        }
        /// Apply graph edits first when needed. Operation errors retain the graph
        /// and revision in the returned outcome. Preflight or transport failure
        /// throws; post-call export failure can leave native state advanced.
        public func invoke(method: UInt32, receiver: PublicGraph.Atom, arguments: [PublicGraph.Atom] = [], resources: HostResources = .init()) throws -> PublicInvocationOutcome {
            struct Call: Encodable { let method: UInt32; let receiver: PublicGraph.Atom; let arguments: [PublicGraph.Atom] }
            let bytes = try JSONEncoder().encode(Call(method: method, receiver: receiver, arguments: arguments))
            guard bytes.count <= 64 * 1024 * 1024 else { throw PeleError(code: "limit", message: "Public method exceeds 64 MiB JSON budget") }
            return try withHandle { handle in
                let response = resources.withCapturedNativeResources { host in bytes.withUnsafeBytes { input in
                    pele_public_invoke_binary_v5(handle, revision, input.bindMemory(to: UInt8.self).baseAddress, input.count, host)
                }}
                do {
                    let exchange = try Pele.binaryResult(response.handle)
                    let outcome = PublicInvocationOutcome(revision: exchange.revision, graph: exchange.graph, result: try JSONDecoder().decode(PublicOperationResult.self, from: exchange.metadata))
                    revision = outcome.revision
                    if let error = response.error { throw error }
                    return outcome
                } catch { if let original = response.error { throw original }; throw error }
            }
        }
        /// Stateful native rendering publishes graph/revision even on operation error.
        public func renderOperation(options: Engine.Options = .init(), resources: HostResources = .init()) throws -> PublicRenderOutcome {
            let settings = try JSONEncoder().encode(options)
            return try withHandle { handle in
                let response = resources.withCapturedNativeResources { host in settings.withUnsafeBytes { bytes in
                    pele_public_render_operation_binary_v5(handle, revision, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count, host)
                }}
                do {
                    let exchange = try Pele.binaryResult(response.handle)
                    let outcome = PublicRenderOutcome(revision: exchange.revision, graph: exchange.graph, result: try JSONDecoder().decode(PublicRenderResult.self, from: exchange.metadata))
                    revision = outcome.revision
                    if let error = response.error { throw error }
                    return outcome
                } catch { if let original = response.error { throw original }; throw error }
            }
        }
        public func render(options: Engine.Options = .init(), resources: HostResources = .init()) throws -> FullRenderedDiagram {
            switch try renderOperation(options: options, resources: resources).result {
            case .rendered(let rendered): return rendered
            case .thrown(.pele(let error)): throw error
            case .thrown(let error): throw PublicOperationFailure(value: error)
            }
        }
    }
}
extension Pele.Engine {
    /// Parse a supported mutable family. Unimplemented codecs report an explicit error.
    public func parseMutable(_ source: String, options: ParseOptions = .init()) throws -> Pele.MutableDocument {
        let source = Data(source.utf8), settings = try JSONEncoder().encode(options)
        var document: OpaquePointer?
        let result = source.withUnsafeBytes { input in settings.withUnsafeBytes { output in
            pele_public_parse_v4(input.bindMemory(to: UInt8.self).baseAddress, input.count,
                                 output.bindMemory(to: UInt8.self).baseAddress, output.count, &document)
        }}
        do {
            let metadata = try Pele.compatibilityResult(result, as: PublicDocumentMetadata.self)
            guard let handle = document, let revision = UInt64(metadata.revision), String(revision) == metadata.revision else {
                throw PeleError(code: "internal", message: "Invalid mutable document metadata")
            }
            return Pele.MutableDocument(handle: handle, type: metadata.type, revision: revision)
        } catch {
            if let document { pele_public_document_free_v4(document) }
            throw error
        }
    }
}
