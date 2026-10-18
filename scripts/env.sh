#!/usr/bin/env bash
# Sourced by build scripts; works even when rustup's proxy bin is not on PATH.
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
rust_bin="$(dirname "$(rustup which --toolchain 1.98.1 rustc)")"
export PATH="$rust_bin:$PATH"
cd "$repo_dir"
