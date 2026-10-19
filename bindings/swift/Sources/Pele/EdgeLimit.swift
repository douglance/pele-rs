extension Pele {
    /// Exact JavaScript numeric edge threshold. Undefined selects the default;
    /// null compares as zero. Numbers retain their IEEE-754 bits unchanged.
    /// Edge accounting remains diagram-specific; native work budgets still apply.
    public enum EdgeLimit: Sendable {
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
