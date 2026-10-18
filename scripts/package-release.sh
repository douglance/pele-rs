#!/usr/bin/env bash
# Package already-verified sources. Publishing is a separate explicit action.
set -euo pipefail
source "$(dirname "$0")/env.sh"
release_version="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "pele-rs"))')"
web_version="$(node -p 'require("./bindings/web/package.json").version')"
if [[ "$release_version" != "$web_version" ]]; then
    echo 'Rust and web release versions must match' >&2; exit 1
fi
scripts/build-native.sh --universal
scripts/build-wasm.sh
cargo package --locked --allow-dirty -p pele-rs
mkdir -p dist/release
cp "target/package/pele-rs-$release_version.crate" dist/release/
archive_dir="$(mktemp -d "$repo_dir/dist/release/.native-package.XXXXXX")"
ditto -c -k --sequesterRsrc --keepParent bindings/swift/Artifacts/CPele.xcframework "$archive_dir/CPele-macos-universal.xcframework.zip"
mv "$archive_dir/CPele-macos-universal.xcframework.zip" dist/release/CPele-macos-universal.xcframework.zip
rmdir "$archive_dir"
checksum="$(swift package compute-checksum dist/release/CPele-macos-universal.xcframework.zip)"
python3 scripts/swift-manifest.py "$checksum" "$release_version"
swift run --package-path tests/swift-consumer
(cd dist/release && shasum -a 256 "pele-rs-$release_version.crate" \
    "unlocalhosted-pele-rs-$release_version.tgz" CPele-macos-universal.xcframework.zip > SHA256SUMS)
