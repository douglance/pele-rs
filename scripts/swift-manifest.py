"""Generate the Swift distribution manifest from the exact release archive hash."""
from pathlib import Path
import sys
import re

if len(sys.argv) != 3:
    raise SystemExit('Expected archive checksum and release version')
checksum, version = sys.argv[1:]
if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", version):
    raise SystemExit('Expected semantic release version')
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
        url: "https://github.com/vijayksingh/pele-rs/releases/download/v''' + version + '''/CPele-macos-universal.xcframework.zip",
        checksum: "''' + checksum + '''")

let package = Package(name: "Pele", platforms: [.macOS(.v14)],
    products: [.library(name: "Pele", targets: ["Pele"])],
    targets: [binary, .target(name: "Pele", dependencies: ["CPele"],
                            path: "bindings/swift/Sources/Pele")])
''')
