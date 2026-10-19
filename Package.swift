// swift-tools-version: 6.0
import PackageDescription
import Foundation

let local = "bindings/swift/Artifacts/CPele.xcframework"
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
guard FileManager.default.fileExists(atPath: root.appendingPathComponent(local).path) else {
    fatalError("Build the matching native SDK first: scripts/build-native.sh --universal")
}
let package = Package(name: "Pele", platforms: [.macOS(.v14)],
    products: [.library(name: "Pele", targets: ["Pele"])],
    targets: [.binaryTarget(name: "CPele", path: local),
              .target(name: "Pele", dependencies: ["CPele"], path: "bindings/swift/Sources/Pele")])
