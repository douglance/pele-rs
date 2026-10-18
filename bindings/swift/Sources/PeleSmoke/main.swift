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
