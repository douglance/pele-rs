// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Pele",
    platforms: [.macOS(.v14)],
    products: [.library(name: "Pele", targets: ["Pele"]),
               .executable(name: "pele-smoke", targets: ["PeleSmoke"])],
    targets: [
        .binaryTarget(name: "CPele", path: "Artifacts/CPele.xcframework"),
        .target(name: "Pele", dependencies: ["CPele"]),
        .executableTarget(name: "PeleSmoke", dependencies: ["Pele"]),
        .testTarget(name: "PeleTests", dependencies: ["Pele"])
    ]
)
