#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
cargo fmt --all --check
cargo test --locked -p pele-rs --test conformance --test source_compat --test module_contract --test pie_compat
cargo run --locked -p pele-rs --example corpus
cargo package --locked --allow-dirty -p pele-rs
cargo run --locked --manifest-path tests/rust-consumer/Cargo.toml
scripts/build-native.sh
cc -Wall -Wextra -Werror -I include tests/ffi.c target/release/libpele_ffi.a -o target/ffi-integration
target/ffi-integration
if [[ "$(uname -s)" == Darwin ]]; then
    swift test --package-path bindings/swift
    swift run --package-path bindings/swift pele-smoke "$repo_dir" "$repo_dir/dist/evidence"
    if [[ -f Package.swift ]]; then swift run --package-path tests/swift-consumer; fi
    npm run test:parity
fi
scripts/build-wasm.sh
npm run test:web
git diff --check
