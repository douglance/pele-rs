extension Pele {
    /// Pele's source limit compares the original UTF-16 length before preparation.
    /// Undefined uses 50,000; null compares as zero. Numbers preserve IEEE bits.
    /// The independent native 16 MiB UTF-8 source budget always remains active.
    public enum SourceLimit: Sendable {
        case undefined
        case null
        case number(Double)

        var nativeValue: (kind: UInt32, bits: UInt64) {
            switch self {
            case .undefined: return (0, 0)
            case .null: return (1, 0)
            case .number(let value): return (2, value.bitPattern)
            }
        }
    }
}
