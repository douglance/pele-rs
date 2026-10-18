#!/usr/bin/env bash
# Sourced by build scripts; works even when rustup's proxy bin is not on PATH.
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rust_bin="$(dirname "$(rustup which --toolchain 1.98.1 rustc)")"
export PATH="$rust_bin:$PATH"
export DYLD_FALLBACK_LIBRARY_PATH="$rust_bin/../lib:${DYLD_FALLBACK_LIBRARY_PATH:-/usr/local/lib:/usr/lib}"
cd "$repo_dir"
