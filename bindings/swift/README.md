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
