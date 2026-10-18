#!/usr/bin/env bash
# Package already-verified sources. Publishing is a separate explicit action.
set -euo pipefail
source "$(dirname "$0")/env.sh"
scripts/build-native.sh --universal
scripts/build-wasm.sh
cargo package --locked --allow-dirty -p pele-rs
mkdir -p dist/release
cp target/package/pele-rs-0.1.0.crate dist/release/
cd bindings/swift/Artifacts
ditto -c -k --sequesterRsrc --keepParent CPele.xcframework "$repo_dir/dist/release/CPele-macos-universal.xcframework.zip"
cd "$repo_dir"
checksum="$(swift package compute-checksum dist/release/CPele-macos-universal.xcframework.zip)"
python3 scripts/swift-manifest.py "$checksum"
swift run --package-path tests/swift-consumer
(cd dist/release && shasum -a 256 ./*.crate ./*.tgz ./*.zip > SHA256SUMS)
