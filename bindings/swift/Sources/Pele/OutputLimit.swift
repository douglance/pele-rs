extension Pele {
    /// Output is compared by UTF-16 length after native rendering and callbacks.
    /// Undefined and null use 4,000,000; numbers preserve IEEE bits exactly.
    /// The independent returned-SVG 64 MiB byte cap and transport budgets remain
    /// active. These bound accepted output, not all renderer peak allocations.
    public enum OutputLimit: Sendable {
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
