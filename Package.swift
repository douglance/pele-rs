// swift-tools-version: 6.0
import PackageDescription
import Foundation

let local = "bindings/swift/Artifacts/CPele.xcframework"
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let hasLocalCore = FileManager.default.fileExists(atPath: root.appendingPathComponent(local).path)
let binary: Target = hasLocalCore
    ? .binaryTarget(name: "CPele", path: local)
    : .binaryTarget(name: "CPele",
        url: "https://github.com/vijayksingh/pele-rs/releases/download/v0.1.0/CPele-macos-universal.xcframework.zip",
        checksum: "51427f1379de92f0335fdbe4dcfd7b9a3c92c56b7fd02677d0794111baf6c45f")

let package = Package(name: "Pele", platforms: [.macOS(.v14)],
    products: [.library(name: "Pele", targets: ["Pele"])],
    targets: [binary, .target(name: "Pele", dependencies: ["CPele"],
                            path: "bindings/swift/Sources/Pele",
                            exclude: hasLocalCore ? [] : ["SourceInspection.swift", "Engine.swift", "HostResources.swift", "CalendarResources.swift", "PublicGraph.swift", "MutableDocument.swift", "PublicMethods.swift", "PublicBinary.swift", "SourceLimit.swift", "OutputLimit.swift"])])
