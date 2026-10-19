// swift-tools-version: 6.0
import PackageDescription
import Foundation

let packageURL = ProcessInfo.processInfo.environment["PELE_PACKAGE_URL"]
let versionString = ProcessInfo.processInfo.environment["PELE_PACKAGE_VERSION"] ?? "0.3.0"
guard let packageVersion = Version(versionString) else { fatalError("Invalid PELE_PACKAGE_VERSION: \(versionString)") }
let dependency: Package.Dependency
if let url = packageURL {
    dependency = .package(url: url, exact: packageVersion)
} else {
    dependency = .package(name: "pele-rs", path: "../..")
}
let package = Package(name: "PeleConsumer", platforms: [.macOS(.v14)],
    dependencies: [dependency],
    targets: [.executableTarget(name: "PeleConsumer", dependencies: [.product(name: "Pele", package: "pele-rs")])])
