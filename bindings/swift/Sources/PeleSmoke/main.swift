import Foundation
import AppKit
import Pele

// Run with repository root and output directory. Verifies actual AppKit SVG pixels.
let arguments = CommandLine.arguments
guard arguments.count == 3 else { fatalError("Usage: pele-smoke REPOSITORY OUTPUT_DIRECTORY") }
let repository = URL(fileURLWithPath: arguments[1])
let output = URL(fileURLWithPath: arguments[2])
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
let source = try String(contentsOf: repository.appendingPathComponent("fixtures/branch.mmd"), encoding: .utf8)
let diagram = try Pele.render(source)
let svg = Data(diagram.svg.utf8)
try svg.write(to: output.appendingPathComponent("swift-flowchart.svg"))
guard let image = NSImage(data: svg), image.size.width > 0, image.size.height > 0 else {
    fatalError("AppKit failed to decode the produced SVG")
}
let width = Int(ceil(diagram.width)), height = Int(ceil(diagram.height))
guard width * height <= 16_000_000,
      let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
          bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
          colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
      let context = NSGraphicsContext(bitmapImageRep: bitmap) else { fatalError("Raster allocation failed") }
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = context
image.draw(in: NSRect(x: 0, y: 0, width: width, height: height))
NSGraphicsContext.restoreGraphicsState()
var painted = 0
for y in 0..<height { for x in 0..<width {
    if let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB), color.alphaComponent > 0.5, color.redComponent < 0.7 { painted += 1 }
}}
guard painted > 100, let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("SVG produced no meaningful native pixels")
}
try png.write(to: output.appendingPathComponent("swift-flowchart.png"))
print("Swift C ABI + AppKit SVG: \(width)x\(height), \(painted) painted pixels; \(output.path)")

// Source-built v2 path: actual native SVG with resolved palette and owned document reuse.
let engine = Pele.Engine()
let pieDocument = try engine.parse("pie title Shared Rust\n\"Capture\":2\n\"Keep\":5\n\"Use\":3")
var nativeOptions = Pele.Engine.Options(); nativeOptions.nativeTheme = .init()
let pie = try engine.render(pieDocument, options: nativeOptions)
try Data(pie.svg.utf8).write(to: output.appendingPathComponent("swift-pie.svg"))
guard let pieImage = NSImage(data: Data(pie.svg.utf8)), pieImage.size.width > 0 else { fatalError("AppKit failed to decode native pie SVG") }
let pieWidth = Int(ceil(pie.width)), pieHeight = Int(ceil(pie.height))
guard let pieBitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pieWidth, pixelsHigh: pieHeight,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
    let pieContext = NSGraphicsContext(bitmapImageRep: pieBitmap) else { fatalError("Pie raster allocation failed") }
NSGraphicsContext.saveGraphicsState(); NSGraphicsContext.current = pieContext
NSColor.white.setFill(); NSRect(x: 0, y: 0, width: pieWidth, height: pieHeight).fill()
pieImage.draw(in: NSRect(x: 0, y: 0, width: pieWidth, height: pieHeight))
NSGraphicsContext.restoreGraphicsState()
var colored = 0
for y in 0..<pieHeight { for x in 0..<pieWidth {
    if let color = pieBitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB), color.alphaComponent > 0.5 {
        let channels = [color.redComponent, color.greenComponent, color.blueComponent]
        if channels.max()! - channels.min()! > 0.15 { colored += 1 }
    }
}}
guard colored > 10_000, let piePNG = pieBitmap.representation(using: .png, properties: [:]) else { fatalError("Native pie palette did not paint") }
try piePNG.write(to: output.appendingPathComponent("swift-pie.png"))
print("Swift owned pie + AppKit: \(pieWidth)x\(pieHeight), \(colored) colored pixels")

for (type, source) in [
    ("packet", "packet-beta\ntitle Network header\n0-7: \"Version\"\n8-15: \"Flags\"\n16-31: \"Length\""),
    ("radar", "radar-beta\ntitle Skills\naxis a[\"Speed\"], b[\"Quality\"], c[\"Cost\"]\ncurve team[\"Team\"]{4, 3, 5}"),
    ("timeline", "timeline\ntitle Releases\nsection Core\n2025: Rust\n2026: Swift: Web"),
    ("journey", "journey\ntitle Build and ship\nsection Core\nImplement: 4: Rust\nVerify: 5: Swift, Web"),
    ("quadrantChart", "quadrantChart\ntitle Work priorities\nx-axis Low --> High\ny-axis Low --> High\nFast: [0.8, 0.7]\nCareful: [0.3, 0.4]")
] {
    let document = try engine.parse(source)
    let rendered = try engine.render(document, options: nativeOptions)
    guard rendered.type == type, !rendered.svg.contains("var(") else { fatalError("Unresolved \(type) native export") }
    try Data(rendered.svg.utf8).write(to: output.appendingPathComponent("swift-\(type).svg"))
    guard let image = NSImage(data: Data(rendered.svg.utf8)), image.size.width > 0 else { fatalError("AppKit failed to decode \(type)") }
    let width = Int(ceil(rendered.width)), height = Int(ceil(rendered.height))
    guard width > 0, height > 0, width * height <= 16_000_000,
        let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
        let context = NSGraphicsContext(bitmapImageRep: bitmap) else { fatalError("Native \(type) raster allocation failed") }
    NSGraphicsContext.saveGraphicsState(); NSGraphicsContext.current = context
    NSColor.white.setFill(); NSRect(x: 0, y: 0, width: width, height: height).fill()
    image.draw(in: NSRect(x: 0, y: 0, width: width, height: height))
    NSGraphicsContext.restoreGraphicsState()
    var painted = 0, titleInk = 0
    for y in 0..<height { for x in 0..<width {
        if let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB),
            color.alphaComponent > 0.5, min(color.redComponent, color.greenComponent, color.blueComponent) < 0.9 {
            painted += 1
            let inTitle = type == "packet" ? y >= height - 36 : y < 36
            if inTitle && max(color.redComponent, color.greenComponent, color.blueComponent) < 0.5 { titleInk += 1 }
        }
    }}
    guard painted > 100, let png = bitmap.representation(using: .png, properties: [:]) else { fatalError("No meaningful native \(type) pixels") }
    guard titleInk > 20 else { fatalError("AppKit dropped positioned title in \(type)") }
    try png.write(to: output.appendingPathComponent("swift-\(type).png"))
    print("Swift owned \(type) + AppKit: \(width)x\(height), \(painted) painted pixels, \(titleInk) title pixels")
}
