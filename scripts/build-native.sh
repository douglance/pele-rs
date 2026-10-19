#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --locked --release -p pele-ffi
if [[ "$(uname -s)" == Darwin ]]; then
    library="$repo_dir/target/release/libpele_ffi.a"
    if [[ "${1:-}" == --universal ]]; then
        cargo build --locked --release -p pele-ffi --target aarch64-apple-darwin
        cargo build --locked --release -p pele-ffi --target x86_64-apple-darwin
        mkdir -p target/universal
        xcrun lipo -create target/aarch64-apple-darwin/release/libpele_ffi.a \
          target/x86_64-apple-darwin/release/libpele_ffi.a -output target/universal/libpele_ffi.a
        library="$repo_dir/target/universal/libpele_ffi.a"
    elif [[ -n "${1:-}" ]]; then
        echo 'Usage: scripts/build-native.sh [--universal]' >&2; exit 2
    fi
    artifact="$repo_dir/bindings/swift/Artifacts/CPele.xcframework"
    mkdir -p "$(dirname "$artifact")"
    # Only generated output belonging to this repository is replaced.
    if [[ -d "$artifact" ]]; then rm -rf "$artifact"; fi
    xcodebuild -create-xcframework -library "$library" \
      -headers "$repo_dir/include" -output "$artifact"
    cp third_party/v8/LICENSE "$artifact/V8-LICENSE"
    cp third_party/fdlibm/LICENSE "$artifact/FDLIBM-LICENSE"
    cp NOTICE "$artifact/NOTICE"
    cp crates/pele/assets/RUST-DEPENDENCY-LICENSES.txt "$artifact/RUST-DEPENDENCY-LICENSES.txt"
    cp crates/pele/assets/RUST-STDLIB-LICENSES.txt "$artifact/RUST-STDLIB-LICENSES.txt"
    cp LICENSE "$artifact/LICENSE"
    cp third_party/PELE-LICENSE "$artifact/PELE-LICENSE"
    cp third_party/MERMAID-LICENSE "$artifact/MERMAID-LICENSE"
    cp crates/pele/assets/fonts/OFL.txt "$artifact/FONT-OFL.txt"
fi
