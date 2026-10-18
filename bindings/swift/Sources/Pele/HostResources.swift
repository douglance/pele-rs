import Foundation
import CPele

extension Pele {
    public struct TextMeasurement: Sendable {
        public let text: String
        public let size: Double
        public let style: UInt8
        public let fontFamily: String?
        public let fontFamilyMono: String?
    }
    public struct TrustedSVG: Sendable {
        public let markup: String
        /// Host-owned trusted markup. Never pass authored diagram text here.
        public init(trustedMarkup: String) { markup = trustedMarkup }
    }
    public struct HostResources: Sendable {
        public var textWidth: (@Sendable (TextMeasurement) throws -> Double)?
        public var icon: (@Sendable (String) throws -> TrustedSVG?)?
        public var nowMillis: (@Sendable () throws -> Double)?
        public init() {}
        func withNativeResources(_ invoke: (UnsafePointer<PeleHostResources>) -> OpaquePointer?) throws -> OpaquePointer? {
            let bridge = ResourceBridge(self)
            var callbacks = PeleHostResources(context: Unmanaged.passUnretained(bridge).toOpaque(),
                width: textWidth == nil ? nil : resourceWidth,
                icon: icon == nil ? nil : resourceIcon,
                now: nowMillis == nil ? nil : resourceClock)
            let result = withExtendedLifetime(bridge) { withUnsafePointer(to: &callbacks, invoke) }
            if let error = bridge.error { pele_result_free(result); throw error }
            return result
        }
    }
}
private final class ResourceBridge {
    let resources: Pele.HostResources
    var error: (any Error)?
    var iconBytes = Data()
    var iconStorage: NSData?
    init(_ resources: Pele.HostResources) { self.resources = resources }
}
private func text(_ data: UnsafePointer<UInt8>?, _ count: Int) -> String? {
    guard let data else { return nil }
    return String(decoding: UnsafeBufferPointer(start: data, count: count), as: UTF8.self)
}
private func resourceWidth(_ context: UnsafeMutableRawPointer?, _ source: UnsafePointer<UInt8>?, _ length: Int,
                           _ size: Double, _ style: UInt32, _ font: UnsafePointer<UInt8>?, _ fontLength: Int,
                           _ mono: UnsafePointer<UInt8>?, _ monoLength: Int, _ output: UnsafeMutablePointer<Double>?) -> UInt32 {
    guard let context, let output else { return 1 }
    let bridge = Unmanaged<ResourceBridge>.fromOpaque(context).takeUnretainedValue()
    do {
        let request = Pele.TextMeasurement(text: text(source, length) ?? "", size: size, style: UInt8(truncatingIfNeeded: style),
            fontFamily: text(font, fontLength), fontFamilyMono: text(mono, monoLength))
        output.pointee = try bridge.resources.textWidth!(request)
        return 0
    } catch { bridge.error = error; return 1 }
}
private func resourceIcon(_ context: UnsafeMutableRawPointer?, _ name: UnsafePointer<UInt8>?, _ count: Int,
                          _ output: UnsafeMutablePointer<PeleBorrowedBytes>?) -> UInt32 {
    guard let context, let output else { return 1 }
    let bridge = Unmanaged<ResourceBridge>.fromOpaque(context).takeUnretainedValue()
    do {
        guard let icon = try bridge.resources.icon!(text(name, count) ?? "") else {
            output.pointee = PeleBorrowedBytes(data: nil, len: 0); return 0
        }
        bridge.iconBytes = Data(icon.markup.utf8)
        // NSData owns stable backing storage until the next callback. Rust copies synchronously.
        let bytes = bridge.iconBytes as NSData
        bridge.iconStorage = bytes
        output.pointee = PeleBorrowedBytes(data: bytes.bytes.assumingMemoryBound(to: UInt8.self), len: bytes.length)
        return 0
    } catch { bridge.error = error; return 1 }
}
private func resourceClock(_ context: UnsafeMutableRawPointer?, _ output: UnsafeMutablePointer<Double>?) -> UInt32 {
    guard let context, let output else { return 1 }
    let bridge = Unmanaged<ResourceBridge>.fromOpaque(context).takeUnretainedValue()
    do { output.pointee = try bridge.resources.nowMillis!(); return 0 }
    catch { bridge.error = error; return 1 }
}
