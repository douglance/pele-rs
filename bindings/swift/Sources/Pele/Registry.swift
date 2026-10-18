import Foundation
import CPele

extension Pele {
    /// Owned selective Rust registry. Confine to one owner/actor; not Sendable.
    /// Parsed documents retain their handlers and survive registry closure.
    public final class Registry {
        private var handle: OpaquePointer?
        private var active = false
        private init(handle: OpaquePointer) { self.handle = handle }
        deinit { if let handle { pele_registry_free_v8(handle) } }
        private static func create(_ mode: UInt32) throws -> Registry {
            var handle: OpaquePointer?
            let result = pele_registry_create_v8(mode, &handle)
            do {
                let created = try Pele.compatibilityResult(result, as: Bool.self)
                guard created, let handle else { throw PeleError(code: "internal", message: "Invalid registry creation result") }
                return Registry(handle: handle)
            } catch {
                if let handle { pele_registry_free_v8(handle) }
                throw error
            }
        }
        public static func empty() throws -> Registry { try create(0) }
        public static func allBuiltins() throws -> Registry { try create(1) }
        public func close() throws {
            guard !active else { throw PeleError(code: "semantic", message: "Registry is already in use") }
            if let handle { pele_registry_free_v8(handle); self.handle = nil }
        }
        private func withHandle<T>(_ operation: (OpaquePointer) throws -> T) throws -> T {
            guard let handle else { throw PeleError(code: "semantic", message: "Registry is closed") }
            guard !active else { throw PeleError(code: "semantic", message: "Registry is already in use") }
            active = true; defer { active = false }
            return try withExtendedLifetime(self) { try operation(handle) }
        }
        public func capabilities() throws -> [DiagramCapability] {
            try withHandle { try Pele.compatibilityResult(pele_registry_capabilities_v8($0), as: [DiagramCapability].self) }
        }
        public func registered(_ id: String) throws -> Bool {
            let bytes = Data(id.utf8)
            return try withHandle { handle in try bytes.withUnsafeBytes { input in
                try Pele.compatibilityResult(pele_registry_registered_v8(handle, input.bindMemory(to: UInt8.self).baseAddress, input.count), as: Bool.self)
            }}
        }
        public func registerBuiltin(_ id: String) throws {
            let bytes = Data(id.utf8)
            try withHandle { handle in try bytes.withUnsafeBytes { input in
                let registered = try Pele.compatibilityResult(pele_registry_register_builtin_v8(handle, input.bindMemory(to: UInt8.self).baseAddress, input.count), as: Bool.self)
                guard registered else { throw PeleError(code: "internal", message: "Invalid registry registration result") }
            }}
        }
        /// Preprocess and detect without applying parse limits or grammar checks.
        public func detectType(_ source: String) throws -> String? {
            struct Detection: Decodable { let type: String? }
            let bytes = Data(source.utf8)
            return try withHandle { handle in try bytes.withUnsafeBytes { input in
                try Pele.compatibilityResult(pele_registry_detect_type_v8(handle, input.bindMemory(to: UInt8.self).baseAddress, input.count), as: Detection.self).type
            }}
        }
        /// Detection plus registration; preprocessing failures return false.
        /// Native source-byte/UTF-8 transport errors still throw.
        public func supports(_ source: String) throws -> Bool {
            let bytes = Data(source.utf8)
            return try withHandle { handle in try bytes.withUnsafeBytes { input in
                try Pele.compatibilityResult(pele_registry_supports_v8(handle, input.bindMemory(to: UInt8.self).baseAddress, input.count), as: Bool.self)
            }}
        }
        public func parseMutable(_ source: String, limits: ParseLimits = .init(), options: Engine.ParseOptions = .init()) throws -> MutableDocument {
            let bytes = Data(source.utf8), settings = try JSONEncoder().encode(options)
            let sourceLimit = limits.source?.nativeValue ?? (kind: UInt32(3), bits: UInt64(0))
            let edgeLimit = limits.maxEdges?.nativeValue ?? (kind: UInt32(3), bits: UInt64(0))
            return try withHandle { handle in
                var document: OpaquePointer?
                let result = bytes.withUnsafeBytes { input in settings.withUnsafeBytes { output in
                    pele_registry_parse_limits_v8(handle, input.bindMemory(to: UInt8.self).baseAddress, input.count,
                        output.bindMemory(to: UInt8.self).baseAddress, output.count,
                        sourceLimit.kind, sourceLimit.bits, edgeLimit.kind, edgeLimit.bits, &document)
                }}
                return try ownedPublicDocument(result, document: document)
            }
        }
    }
}
