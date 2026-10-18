#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
if [[ "$(wasm-bindgen --version)" != 'wasm-bindgen 0.2.128' ]]; then
    echo 'Install matching CLI: cargo install wasm-bindgen-cli --version 0.2.128 --locked' >&2
    exit 1
fi
# rust-lld on macOS needs the LLVM library shipped in the toolchain's lib/.
DYLD_FALLBACK_LIBRARY_PATH="$rust_bin/../lib:${DYLD_FALLBACK_LIBRARY_PATH:-/usr/local/lib:/usr/lib}" \
CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_OPT_LEVEL=s \
  cargo build --locked --release -p pele-wasm --target wasm32-unknown-unknown
mkdir -p dist/wasm dist/evidence
# Test oracle only, never included in the distributed WASM package.
cp node_modules/pele/dist/pele.js dist/evidence/pele-oracle.js
wasm-bindgen target/wasm32-unknown-unknown/release/pele_wasm.wasm \
  --target web --out-dir dist/wasm --out-name pele
mkdir -p bindings/web/pkg
cp dist/wasm/* bindings/web/pkg/
cp third_party/v8/LICENSE bindings/web/V8-LICENSE
cp third_party/fdlibm/LICENSE bindings/web/FDLIBM-LICENSE
cp LICENSE bindings/web/LICENSE
cp NOTICE bindings/web/NOTICE
cp third_party/PELE-LICENSE bindings/web/PELE-LICENSE
cp third_party/MERMAID-LICENSE bindings/web/MERMAID-LICENSE
cp crates/pele/assets/fonts/OFL.txt bindings/web/FONT-OFL.txt
mkdir -p dist/release dist/package-web
npm pack ./bindings/web --pack-destination dist/release --silent
tar -xzf dist/release/unlocalhosted-pele-rs-0.1.0.tgz -C dist/package-web
