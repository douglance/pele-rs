# Swift

macOS 14+, Swift 6+. The v0.1.0 GitHub release distributes a universal arm64/x86_64
XCFramework. The repository root is the consumer Swift package; this directory
also has a local development package with the native smoke example and corpus tests.

```swift
// In your application's Package.swift:
.package(url: "https://github.com/vijayksingh/pele-rs.git", from: "0.1.0")
// Add .product(name: "Pele", package: "pele-rs") to your target dependencies.
```

```swift
import Pele
let diagram = try Pele.render("flowchart LR; A[Hello] --> B{Ready?}")
// diagram.svg, diagram.geometry.nodes, diagram.geometry.edges
```

For macOS presentation, `NSImage(data: Data(diagram.svg.utf8))` supports the simple
outlined SVG generated here. See `Sources/PeleSmoke/main.swift` for actual AppKit
rasterization and painted-pixel verification. Rendering unrelated SVG is outside
the library's contract. Geometry coordinates are pixels with a top-left origin.

Develop from source:

```sh
scripts/build-native.sh                   # Host architecture
scripts/build-native.sh --universal       # macOS arm64 + x86_64
swift test --package-path bindings/swift
swift run --package-path bindings/swift pele-smoke "$PWD" "$PWD/dist/evidence"
swift run --package-path tests/swift-consumer
```

Native code contains the font and all graph logic. The Swift layer only encodes
options, copies owned C results, frees them, and decodes typed values/errors.
No Kamui, web view, runtime JavaScript, or installed fonts are required.

Source-built development artifacts also expose all 33 renderers through
`Pele.Engine` and all 33 mutable family codecs through `parseMutable`. Flat
graph edits and schema-described native methods preserve aliases, UTF-16 code
units, number bits and partial-error state. See
[mutable APIs](../../docs/MUTABLE_MODELS.md) for supported families, ownership
and remaining compatibility boundaries. These APIs require current local
artifacts and do not replace the earlier v0.1.0 downloaded binary.

The source-built AppKit adapter provides `Pele.DiagramView`, `Pele.mount`,
`Pele.Mounted` and `Pele.enableZoom`. A mounted view retains its last good drawing
on failure, observes size/font changes, and supports explicit updates after
automatic observation is destroyed. Zoom uses native mouse, wheel, pinch,
keyboard and controls without rerendering the diagram.

An owned selective registry can mount through the same lifecycle:

```swift
@MainActor
func show(in view: Pele.DiagramView) async throws -> Pele.Mounted {
    let registry = try Pele.Registry.empty()
    let mounted = try await registry.mountAsync(view, source: "pie\n\"Rust\":3")
    // Later synchronous updates need their family selected explicitly.
    _ = try registry.load("info")
    try mounted.update("info")
    return mounted
}
```

Mounting retains the registry; closing it leaves existing parsed models renderable
but rejects later parsing. `Pele.AsyncEngine` supplies an independent actor-owned
background render API. AppKit mounting runs on MainActor. Neither API promises
native code downloading. Interactive diagram links, full diagram accessibility
and approved image-resource presentation remain open release requirements.
