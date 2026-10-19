// swift-tools-version: 6.0
import PackageDescription
import Foundation

let packageURL = ProcessInfo.processInfo.environment["PELE_PACKAGE_URL"]
let versionString = ProcessInfo.processInfo.environment["PELE_PACKAGE_VERSION"] ?? "0.1.0"
guard let packageVersion = Version(versionString) else { fatalError("Invalid PELE_PACKAGE_VERSION: \(versionString)") }
let dependency: Package.Dependency
if let url = packageURL {
    dependency = .package(url: url, exact: packageVersion)
} else {
    dependency = .package(name: "pele-rs", path: "../..")
}
// Published 0.1 exposes only the original geometry API. Both local source and
// explicitly selected 0.2+ releases must exercise the full current API surface.
let usesCurrentAPI = packageURL == nil || packageVersion.major > 0 || packageVersion.minor >= 2
let package = Package(name: "PeleConsumer", platforms: [.macOS(.v14)],
    dependencies: [dependency],
    targets: [.executableTarget(name: "PeleConsumer", dependencies: [.product(name: "Pele", package: "pele-rs")],
        swiftSettings: usesCurrentAPI ? [.define("PELE_SOURCE_CORE")] : [])])
