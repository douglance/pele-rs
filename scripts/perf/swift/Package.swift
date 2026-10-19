// swift-tools-version: 6.0
import PackageDescription
import Foundation
let root = ProcessInfo.processInfo.environment["PELE_SWIFT_PERF_ROOT"] ?? "../../.."
let package = Package(name: "PeleSwiftPerf", platforms: [.macOS(.v14)], dependencies: [.package(name: "pele-rs", path: root)], targets: [.executableTarget(name: "PeleSwiftPerf", dependencies: [.product(name: "Pele", package: "pele-rs")])])
