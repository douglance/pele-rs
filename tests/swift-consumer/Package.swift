// swift-tools-version: 6.0
import PackageDescription
import Foundation
let dependency: Package.Dependency
if let url = ProcessInfo.processInfo.environment["PELE_PACKAGE_URL"] {
    dependency = .package(url: url, exact: "0.1.0")
} else {
    dependency = .package(path: "../..")
}
let package = Package(name: "PeleConsumer", platforms: [.macOS(.v14)],
    dependencies: [dependency],
    targets: [.executableTarget(name: "PeleConsumer", dependencies: [.product(name: "Pele", package: "pele-rs")])])
