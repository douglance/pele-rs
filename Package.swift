// swift-tools-version: 6.0
import PackageDescription
import Foundation

let local = "bindings/swift/Artifacts/CPele.xcframework"
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let binary: Target = FileManager.default.fileExists(atPath: root.appendingPathComponent(local).path)
    ? .binaryTarget(name: "CPele", path: local)
    : .binaryTarget(name: "CPele",
        url: "https://github.com/vijayksingh/pele-rs/releases/download/v0.2.0/CPele-macos-universal.xcframework.zip",
        checksum: "bda602dd529e038ecaa89875b3c0f3d90ab8305215c7d893f2f1acdb8f165c8b")

let package = Package(name: "Pele", platforms: [.macOS(.v14)],
    products: [.library(name: "Pele", targets: ["Pele"])],
    targets: [binary, .target(name: "Pele", dependencies: ["CPele"],
                            path: "bindings/swift/Sources/Pele")])
