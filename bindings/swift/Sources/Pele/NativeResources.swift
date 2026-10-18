import AppKit
import CoreText
import ImageIO
import UniformTypeIdentifiers
import CPele

extension Pele {
    /// Native mount resource owner. Never reads image files or fetches image URLs. Host code
    /// explicitly approves and supplies PNG/JPEG bytes. A generation owns bounded
    /// image/font caches; invalidate it when approved content or installed fonts change.
    @MainActor public final class NativeResources {
        public struct Limits: Sendable {
            public var imageBytes = 8 * 1024 * 1024
            public var imagePixels = 16_777_216
            public var totalImageBytes = 16 * 1024 * 1024
            public init() {}
        }
        public var imageLoader: ((String) throws -> Data?)?
        public private(set) var generation: UInt64 = 0
        public let limits: Limits
        private let fonts = NativeFontMetrics()
        private var images: [Data: Data] = [:]
        private var imageBytes = 0
        public init(limits: Limits = .init(), imageLoader: ((String) throws -> Data?)? = nil) {
            self.limits = limits; self.imageLoader = imageLoader
        }
        /// Clears failed/retained resource generations. Mounted.invalidateResources
        /// invokes this automatically before redrawing its retained Rust model.
        public func invalidate() {
            generation &+= 1; images.removeAll(keepingCapacity: true); imageBytes = 0; fonts.invalidate()
        }
        internal func resolved(_ options: Engine.Options, resources: HostResources) throws -> (Engine.Options, HostResources) {
            var options = options, resources = resources
            options.fontFamily = try Self.family(options.fontFamily, mono: false)
            options.fontFamilyMono = try Self.family(options.fontFamilyMono, mono: true)
            if resources.textWidth == nil {
                let fonts = fonts
                resources.textWidth = { try fonts.width($0) }
            }
            return (options, resources)
        }
        private static func family(_ input: String?, mono: Bool) throws -> String {
            let candidates = (input ?? (mono ? "Menlo" : "Helvetica")).split(separator: ",").map {
                $0.trimmingCharacters(in: .whitespacesAndNewlines).trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
            }
            for candidate in candidates {
                let mapped: String
                switch candidate.lowercased() {
                case "sans-serif", "system-ui": mapped = "Helvetica"
                case "serif": mapped = "Times"
                case "monospace": mapped = "Menlo"
                default: mapped = candidate
                }
                if let font = NSFont(name: mapped, size: 12), let family = font.familyName { return family }
                if let installed = NSFontManager.shared.availableFontFamilies.first(where: { $0.caseInsensitiveCompare(mapped) == .orderedSame }) { return installed }
            }
            throw PeleError(code: "resource", message: "Native font family is not installed")
        }
        fileprivate func image(_ uri: String) throws -> Data {
            let key = Data(uri.utf8)
            if let image = images[key] { return image }
            guard uri.utf8.count <= 4096, images.count < 128 else { throw Self.limit("Native image reference exceeds resource budget") }
            guard let imageLoader, let input = try imageLoader(uri) else {
                throw PeleError(code: "resource", message: "Native image requires explicit host approval")
            }
            let byteCap = min(8 * 1024 * 1024, max(0, limits.imageBytes))
            let totalCap = min(16 * 1024 * 1024, max(0, limits.totalImageBytes))
            guard input.count <= byteCap, input.count <= totalCap - min(totalCap, imageBytes) else { throw Self.limit("Native image exceeds encoded byte budget") }
            guard let source = CGImageSourceCreateWithData(input as CFData, [kCGImageSourceShouldCache: false] as CFDictionary),
                  CGImageSourceGetCount(source) == 1, let type = CGImageSourceGetType(source),
                  type == UTType.png.identifier as CFString || type == UTType.jpeg.identifier as CFString,
                  let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
                  let width = properties[kCGImagePropertyPixelWidth] as? NSNumber,
                  let height = properties[kCGImagePropertyPixelHeight] as? NSNumber else {
                throw PeleError(code: "resource", message: "Native image must decode as a single PNG or JPEG")
            }
            let w = width.int64Value, h = height.int64Value
            guard w > 0, h > 0, w <= 8192, h <= 8192,
                  w * h <= Int64(min(16_777_216, max(0, limits.imagePixels))) else { throw Self.limit("Native image exceeds decoded pixel budget") }
            guard let decoded = CGImageSourceCreateImageAtIndex(source, 0, [kCGImageSourceShouldCacheImmediately: true] as CFDictionary) else {
                throw PeleError(code: "resource", message: "Native image pixels could not be decoded")
            }
            // Re-encoding a decoded bitmap removes external references and rejects
            // truncated payloads before the SVG decoder sees the resource.
            let output = NSMutableData()
            guard let destination = CGImageDestinationCreateWithData(output, UTType.png.identifier as CFString, 1, nil) else {
                throw PeleError(code: "resource", message: "Native image encoder is unavailable")
            }
            CGImageDestinationAddImage(destination, decoded, nil)
            guard CGImageDestinationFinalize(destination), output.length <= byteCap,
                  output.length <= totalCap - min(totalCap, imageBytes) else { throw Self.limit("Native image exceeds encoded byte budget") }
            let data = output as Data; images[key] = data; imageBytes += data.count
            return data
        }
        private static func limit(_ message: String) -> PeleError { PeleError(code: "limit", message: message) }
        internal func captured<T>(_ invoke: (UnsafePointer<PeleNativeResourcesV14>) throws -> T) rethrows -> (T, (any Error)?) {
            let bridge = NativeImageBridge(owner: self)
            var packet = PeleNativeResourcesV14()
            packet.version = 14; packet.struct_size = UInt32(MemoryLayout<PeleNativeResourcesV14>.size)
            packet.context = Unmanaged.passUnretained(bridge).toOpaque(); packet.image_loader = nativeImageCallback
            packet.max_image_bytes = UInt64(max(0, limits.imageBytes)); packet.max_image_pixels = UInt64(max(0, limits.imagePixels))
            packet.max_total_bytes = UInt64(max(0, limits.totalImageBytes))
            let value = try withExtendedLifetime(bridge) { try withUnsafePointer(to: &packet, invoke) }
            return (value, bridge.callbackError)
        }
    }
}

@MainActor private final class NativeImageBridge {
    // Only a String and integer address/length cross the actor boundary. The
    // actor-owned state retains NSData until the synchronous native call ends.
    nonisolated let action: @MainActor @Sendable (String) -> (UInt, Int, UInt32)
    init(owner: Pele.NativeResources) {
        let state = NativeImageState()
        action = { uri in
            do {
                let data = try owner.image(uri) as NSData
                state.storage = data
                return (UInt(bitPattern: data.bytes), data.length, 0)
            } catch { state.error = error; return (0, 0, 1) }
        }
        self.state = state
    }
    private let state: NativeImageState
    var callbackError: (any Error)? { state.error }
}
@MainActor private final class NativeImageState { var storage: NSData?; var error: (any Error)? }
private func nativeImageCallback(_ context: UnsafeMutableRawPointer?, _ source: UnsafePointer<UInt8>?, _ count: Int,
                                 _ output: UnsafeMutablePointer<PeleBorrowedBytes>?) -> UInt32 {
    guard Thread.isMainThread, let context, let source, let output, count >= 0, count <= 4096 else { return 1 }
    let action = Unmanaged<NativeImageBridge>.fromOpaque(context).takeUnretainedValue().action
    let uri = String(decoding: UnsafeBufferPointer(start: source, count: count), as: UTF8.self)
    let (address, length, status) = MainActor.assumeIsolated { action(uri) }
    output.pointee = PeleBorrowedBytes(data: UnsafePointer<UInt8>(bitPattern: address), len: length)
    return status
}

/// Every mutable cache and CoreText object is protected by lock; neither escapes.
private final class NativeFontMetrics: @unchecked Sendable {
    private let lock = NSLock()
    private struct Key: Hashable { let text: [UInt16]; let size: UInt64; let style: UInt8; let family: String }
    private struct FontKey: Hashable { let size: UInt64; let style: UInt8; let family: String }
    private var fontCache: [FontKey: CTFont] = [:]
    private var widths: [Key: Double] = [:]
    private var keyBytes = 0
    func invalidate() { lock.lock(); defer { lock.unlock() }; widths.removeAll(keepingCapacity: true); fontCache.removeAll(keepingCapacity: true); keyBytes = 0 }
    func width(_ request: Pele.TextMeasurement) throws -> Double {
        guard request.size.isFinite, request.size > 0, request.size <= 4096, request.text.utf16.count <= 65_536 else {
            throw PeleError(code: "limit", message: "Native text measurement exceeds resource budget")
        }
        let family = request.style & 4 != 0 ? request.fontFamilyMono ?? "Menlo" : request.fontFamily ?? "Helvetica"
        let key = Key(text: Array(request.text.utf16), size: request.size.bitPattern, style: request.style, family: family)
        lock.lock(); defer { lock.unlock() }
        if let width = widths[key] { return width }
        let fontKey = FontKey(size: request.size.bitPattern, style: request.style & 3, family: family)
        let font: CTFont
        if let cached = fontCache[fontKey] { font = cached } else {
            let base = CTFontCreateWithName(family as CFString, request.size, nil)
            var traits: CTFontSymbolicTraits = []
            if request.style & 1 != 0 { traits.insert(.boldTrait) }
            if request.style & 2 != 0 { traits.insert(.italicTrait) }
            font = CTFontCreateCopyWithSymbolicTraits(base, 0, nil, traits, traits) ?? base
            if fontCache.count < 128 { fontCache[fontKey] = font }
        }
        let attributed = NSAttributedString(string: request.text, attributes: [NSAttributedString.Key(kCTFontAttributeName as String): font])
        let width = CTLineGetTypographicBounds(CTLineCreateWithAttributedString(attributed), nil, nil, nil)
        let cost = request.text.utf16.count * 2 + family.utf8.count + 32
        if widths.count < 4096, cost <= 2 * 1024 * 1024 - keyBytes { widths[key] = width; keyBytes += cost }
        return width
    }
}
