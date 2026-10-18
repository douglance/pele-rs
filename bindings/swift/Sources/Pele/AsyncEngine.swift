extension Pele {
    /// Actor-owned lazy registration and rendering. Built-ins are linked into
    /// the native library; loading selects them rather than downloading code.
    /// Registry and mutable documents never cross this actor's boundary.
    public actor AsyncEngine {
        private let registry: Registry

        public init() throws {
            registry = try Registry.empty()
        }

        /// Concurrent requests share this actor's registration state. Once a
        /// synchronous native call starts it completes without cancellation.
        public func load(_ type: String?) async throws -> Bool {
            try Task.checkCancellation()
            return try registry.load(type)
        }

        public func registered(_ type: String) throws -> Bool {
            try registry.registered(type)
        }

        public func capabilities() throws -> [DiagramCapability] {
            try registry.capabilities()
        }

        /// Detect and select before parsing, matching Pele's lazy entry. Parse
        /// and render errors retain successful registrations for later calls.
        /// Explicit limits override integer options independently. Otherwise
        /// integer options become JavaScript numbers, with Pele defaults for
        /// absent values. Output limits always compare after rendering.
        /// Resources run synchronously here. Only cancellation before dispatch
        /// is observed.
        public func renderAsync(
            _ source: String,
            limits: ParseLimits = .init(),
            outputLimit: OutputLimit? = nil,
            options: Engine.Options = .init(),
            resources: HostResources = .init()
        ) async throws -> FullRenderedDiagram {
            try Task.checkCancellation()
            // Detection failure is reported by parse, after the loading step,
            // just as lazy.ts does. Detection itself does not apply parse limits.
            let type = try? registry.detectType(source)
            _ = try registry.load(type)
            var parseOptions = Engine.ParseOptions()
            if let limit = options.limit { parseOptions.limit = limit }
            if let maxEdges = options.maxEdges { parseOptions.maxEdges = maxEdges }
            parseOptions.config = options.config
            let effectiveLimits = ParseLimits(
                source: limits.source ?? options.limit.map { .number(Double($0)) } ?? .undefined,
                maxEdges: limits.maxEdges ?? options.maxEdges.map { .number(Double($0)) } ?? .undefined
            )
            let effectiveOutput = outputLimit ?? options.outputLimit.map { .number(Double($0)) } ?? .undefined
            let document = try registry.parseMutable(source, limits: effectiveLimits, options: parseOptions)
            defer { try? document.close() }
            // Parsed config and policy belong to the retained document. Pele
            // ignores parse-only options when rendering a parsed model.
            var renderOptions = options
            renderOptions.limit = nil; renderOptions.maxEdges = nil; renderOptions.config = [:]
            return try document.render(outputLimit: effectiveOutput, options: renderOptions, resources: resources)
        }

        /// Idempotent cleanup remains available from a cancelled task. An
        /// in-flight native operation finishes first; pending requests may then
        /// observe the closed registry. Actor scheduling does not promise FIFO.
        public func close() throws {
            try registry.close()
        }
    }
}
