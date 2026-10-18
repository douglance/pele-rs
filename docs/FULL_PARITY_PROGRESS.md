# Full-library port: development evidence

The acceptance target remains every row of `PELE_PARITY.md`. The public v0.1.0
release remains the documented legacy flowchart subset. No full-library release
or new GIF has been published from this work.

## Current layer

- Astra's LLD and source inventory: commit `67204fc`. All 33 families and all 287
  pinned upstream TypeScript modules are accounted for.
- Independent `compat` namespace and version-2 source-preparation contract;
  legacy Rust/C/Swift/WASM render interfaces keep version 1.
- Typed `DiagramModule<Model>` and per-render `RenderResources` interfaces;
  immutable documents retain handler, concrete model and resolved config.
  Consumer-module integration checks replacement, retained lifetime, model view,
  source comments/line offsets, host measurement, output limits and parse-once use.
  Open extension IDs with `parse_as` avoid changes to the built-in detector. Registration
  metadata is immutable; handlers receive render-only settings and retained config.
  Engine-owned resource sessions decode/cache icons and capture time per render.
  1,536 additional public consumer-link scenarios check named/numeric entities,
  malformed placeholders, decoded link IDs and upstream error behavior.
- Ordered configuration keeps JS numeric values (including NaN/Infinity
  internally), unsafe-key filtering, integer-key enumeration and JSON scalar
  formatting. Frontmatter/directives use document deep merge and host shallow
  precedence. Compatibility locations/source limits use UTF-16 units.
- Detector knows all 33 families. The empty core has zero registered built-in
  renderers and advertises no completed family parity gates.

## Verification corpus

`fixtures/pele/source.json`: 5,345 public preparation scenarios. Includes all 761
examples in 34 pinned upstream corpus files, upstream YAML metadata cases,
frontmatter/directives/comments, detector ordering/case/boundaries, Unicode,
malformed input and fixed-seed mutations. No source fixture is skipped.

Consolidated `scripts/verify.sh` passed on 2026-10-18 after the interface audit
fixes. Host: macOS 26.6 arm64, Rust 1.98.1, Swift 6.4, Node 25.9.0.

- Twelve Rust integration tests: legacy renderer checks, 5,345 source-preparation
  scenarios, 1,536 consumer-link entity scenarios, downstream module lifecycle,
  open IDs, metadata snapshot, ordered projection, per-render resources and limits.
- Packaged Rust crate verifies; independent Cargo consumer executes successfully.
- C executable passes ownership/NULL/UTF-8/budget checks and compatibility inspection.
- Both Swift integration tests pass, including every final preparation fixture.
- Current Swift/C + AppKit SVG proof: 416 x 789 image, 6,846 painted pixels.
- Independent root SwiftPM consumer runs; 10 legacy Rust/Swift/Pele comparisons pass.
- Six browser integration tests pass using the extracted npm tarball: legacy WASM
  rendering and all 5,345 preparation cases in Chrome, Firefox and WebKit.
- Rust formatting and Git whitespace checks pass. Browser workers remain serialized.

Source-preparation scenarios run through Rust, C/Swift and real browser WASM; the
extension module/resource/link scenarios currently run through the Rust interface.
This does not claim complete callback/module hosting in Swift or JavaScript. No
new CI run, Intel execution or full-parity release is asserted for this branch.

## Remaining acceptance work

Every full family parser/model/renderer, shared layout/text/shape/theme behavior,
complete resource adapters and DOM mount/zoom/lazy APIs remain required. Shared
preparation is not the grammar for 33 diagrams. Follow the LLD's one-complete-family
sequence, beginning with pie, then full flowchart and its shared layout consumers.

Known preparation gaps remain explicit: lone UTF-16 surrogates across UTF-8
transports/unpaired YAML escapes, the explicit 64-level init-JSON work cap, and extreme
radix rounding need further conformance. The present fixture evidence is bounded;
it is not a claim of exhaustive source-language parity.

Development Swift inspection APIs require `scripts/build-native.sh`; root SwiftPM
with the old downloaded v0.1 binary excludes those new symbols. WASM inspection
requires `scripts/build-wasm.sh`. No new release asset is substituted under v0.1.
