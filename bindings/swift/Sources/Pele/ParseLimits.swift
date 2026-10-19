extension Pele {
    /// Independently override parse thresholds. Nil retains the corresponding
    /// legacy integer option; `.undefined` explicitly selects the default.
    public struct ParseLimits: Sendable {
        public var source: SourceLimit?
        public var maxEdges: EdgeLimit?
        public init(source: SourceLimit? = nil, maxEdges: EdgeLimit? = nil) {
            self.source = source
            self.maxEdges = maxEdges
        }
    }
}
