import Foundation
import CPele

/// Native packet for every present scalar render number. JSON retains all other
/// options and their existing encoders; callers' Options values remain untouched.
struct RenderNumberEncoding {
    let optionsJSON: Data
    private let packet: PeleRenderNumbersV9
    let outputKind: UInt32
    let outputBits: UInt64
    init(_ original: Pele.Engine.Options, outputLimit: Pele.OutputLimit?) throws {
        var options = original
        var numbers = PeleRenderNumbersV9(version: 9, struct_size: 56, present: 0,
            font_size_bits: 0, max_width_bits: 0, direction_breakpoint_bits: 0, padding_bits: 0, now_bits: 0)
        if let value = options.fontSize { numbers.present |= 1 << 0; numbers.font_size_bits = value.bitPattern; options.fontSize = nil }
        if let value = options.maxWidth { numbers.present |= 1 << 1; numbers.max_width_bits = value.bitPattern; options.maxWidth = nil }
        if let value = options.directionBreakpoint { numbers.present |= 1 << 2; numbers.direction_breakpoint_bits = value.bitPattern; options.directionBreakpoint = nil }
        if let value = options.padding { numbers.present |= 1 << 3; numbers.padding_bits = value.bitPattern; options.padding = nil }
        if let value = options.now { numbers.present |= 1 << 4; numbers.now_bits = value.bitPattern; options.now = nil }
        if outputLimit != nil { options.outputLimit = nil }
        let limit = outputLimit?.nativeValue ?? (kind: UInt32(3), bits: UInt64(0))
        outputKind = limit.kind; outputBits = limit.bits
        optionsJSON = try JSONEncoder().encode(options); packet = numbers
    }
    func withPacket<T>(_ body: (UnsafePointer<PeleRenderNumbersV9>) throws -> T) rethrows -> T {
        var numbers = packet
        return try withUnsafePointer(to: &numbers, body)
    }
}
extension Pele.Engine.Options {
    /// Convert Foundation Date to JavaScript Date's TimeClip value. Raw Double
    /// now remains unmodified by this helper. Invalid dates select host fallback.
    public mutating func setNow(date: Date) {
        let milliseconds = date.timeIntervalSince1970 * 1000
        guard milliseconds.isFinite, abs(milliseconds) <= 8.64e15 else { now = .nan; return }
        let clipped = milliseconds.rounded(.towardZero)
        now = clipped == 0 ? 0 : clipped
    }
}
