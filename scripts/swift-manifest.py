"""Generate the Swift distribution manifest from the exact release archive hash."""
from pathlib import Path
import sys

checksum = sys.argv[1]
if len(checksum) != 64 or any(c not in '0123456789abcdef' for c in checksum):
    raise SystemExit('Expected SHA-256 checksum')
Path('Package.swift').write_text('''// swift-tools-version: 6.0
import PackageDescription
import Foundation

let local = "bindings/swift/Artifacts/CPele.xcframework"
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let binary: Target = FileManager.default.fileExists(atPath: root.appendingPathComponent(local).path)
    ? .binaryTarget(name: "CPele", path: local)
    : .binaryTarget(name: "CPele",
        url: "https://github.com/vijayksingh/pele-rs/releases/download/v0.1.0/CPele-macos-universal.xcframework.zip",
        checksum: "''' + checksum + '''")

let package = Package(name: "Pele", platforms: [.macOS(.v14)],
    products: [.library(name: "Pele", targets: ["Pele"])],
    targets: [binary, .target(name: "Pele", dependencies: ["CPele"],
                            path: "bindings/swift/Sources/Pele")])
''')
