# v0.1.0 verification

Checked 2026-10-18. Covers the documented first flowchart release, not full
Mermaid/Pele compatibility. Engine and adapters are independent of Kamui.

## Local execution

Host: macOS 26.6 arm64; Rust 1.98.1, Swift 6.4, Node 25.9.0.

- Five Rust public-interface integration tests pass. Shared corpus: 22 fixtures,
  10 rendered and 12 documented errors. Further cases cover four directions,
  joins/cycles/parallel links, 32 varied workflows, 128 separately attached
  parallel edges, UTF-8 locations, XML validity, independent resvg pixels, work
  budgets, invalid options and recovery.
- `cargo package` verifies the distributable crate. Separate Cargo consumer
  executes it, including embedded font, without repository resource lookup.
- Real C executable exercises 400 result lifetimes, NULL handling, invalid UTF-8,
  limits, errors and recovery. arm64 and x86_64 execute; Intel runs under Rosetta.
- Swift runs the shared corpus through typed wrapper and C ABI. Separate SwiftPM
  application consumes root package. AppKit decodes SVG, draws a 416 × 789 PNG
  and verifies 6,846 non-background pixels.
- Ten fixture SVG/geometry results match exactly between Rust and Swift/C. Pele
  1.6.0 agrees on checked node/edge semantics; normalization/exclusions in UPSTREAM.md.
- Actual browser WASM and extracted **npm tarball** pass in Chrome 155.0.8059.80,
  Firefox 157.0 and WebKit 27.2: 22 fixtures each, exact native envelope comparison,
  painted pixels, concrete UI errors/recovery, injection-shaped labels, dark theme,
  local server traversal rejection. Zero browser exceptions. Pixel witnesses:
  7,569 / 7,722 / 7,732 respectively.
- `cargo fmt --all --check` and `git diff --check` pass.

Reproduce with `scripts/verify.sh`. Browser workers are serialized. Temporary
JSON, PNG and logs live in `dist/evidence/`. Different rasterizers need not produce
identical pixels; SVG and geometry equality is asserted exactly.

## Distribution and CI

Assets: `.crate`, npm `.tgz`, universal `CPele-macos-universal.xcframework.zip`,
`SHA256SUMS`. Native archive contains arm64 + x86_64, minimum macOS 14.0.
Font licenses accompany distributions. Swift manifest pins exact archive checksum.

Clean runner: [Shared flowchart conformance](https://github.com/vijayksingh/pele-rs/actions/runs/39113453156).
Conclusion: **success** for source commit `b1b5cd4`. Workflow executes Rust, C, Swift, AppKit and all three web engines, and retains
evidence as an Actions artifact. Its live conclusion is authoritative CI status.

GitHub asset publication and fresh remote Swift consumer are verified after
publication. Compilation alone does not prove release download/install.

## Limits of proof

No physical Intel Mac, macOS 14 machine, iOS, Windows, mobile Safari or Kamui
integration exercised. WebKit automation is not physical Safari-device proof.
No full Mermaid differential parser suite, fuzz campaign or performance comparison.
Unsupported grammar/scripts fail explicitly; no complete compatibility/speed claim.
Distribution is GitHub assets; crates.io/npm registry publication remains separate.
