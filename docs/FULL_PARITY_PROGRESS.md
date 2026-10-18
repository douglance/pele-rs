# Full-library port: development evidence

Acceptance remains every row of `PELE_PARITY.md`: all 33 families and public APIs.
The public v0.1.0 release remains the legacy flowchart subset. No full-library
release or new GIF has been published from this branch.

## Implemented renderers

`Engine::with_builtins()` currently registers six actual Rust diagram modules.
`Engine::empty()` and typed `DiagramModule<Model>` remain extension interfaces.
Registration never certifies a global parity flag; all public API gates remain open.

| Family | Public scenarios | Model/SVG results | Errors |
| --- | ---: | ---: | ---: |
| pie | 3,678 | 1,179 | 2,499 |
| quadrantChart | 10,614 | 1,586 | 9,028 |
| packet | 12,774 | 2,924 | 9,850 |
| radar | 8,431 | 1,791 | 6,640 |
| timeline | 11,932 | 8,017 | 3,915 |
| journey | 10,725 | 3,244 | 7,481 |
| Total | 58,154 | 18,741 | 39,413 |

These fixtures run through Rust, Swift/C and real browser WASM. Models and errors
compare completely. SVG compares byte-for-byte, or by SHA256 of every UTF-8 byte
for large outputs. Extractors pin the upstream revision and source-file hashes.

The actual parsers/models/layouts/renderers are translated, including narrow
adaptation, configuration, accessibility and host resources. There is no upstream
JavaScript renderer in distributed Rust/native/WASM code. JavaScript is a
build/test oracle and browser font/resource adapter only.

Full flowchart parser checkpoint: `76efca9`, 4,796 model/error scenarios, separately
from the legacy renderer. Actual shared layered/compound, shape/edge and full
flowchart rendering are in progress. The checkpoint alone adds no flowchart
handler to built-ins; see `FLOWCHART_PORT.md` for declared-model/private-cache scope.

## Shared interfaces and behavior

- Astra LLD/source inventory: `67204fc`; 33 families and 287 source files accounted for.
- Compatibility core: `e86bfc5`; legacy Rust/C/Swift/WASM interfaces remain v1.
  Pie/owned host adapters: `82a2791`; development model/render results are v2.
- Modules retain typed models and resolved configuration. Documents retain exact
  registration/metadata even after replacement. Open IDs and `parse_as` support
  consumer languages without changing the pinned built-in detector.
- Engine-owned resource sessions cache decoded icons and capture one clock value
  per render. Text/icon callbacks stay per-call; wrappers do not own grammar/layout.
- Ordered JS-compatible values, preprocessing, YAML and entities remain shared.
  Source locations and ordinary source/output limits use UTF-16 units.
- Shared hidden-YAML token matching now preserves the greedy optional-body branch
  found by packet differential tests; pie and radar use the same implementation.
- Typed packet rows share label strings. JSON projection stops while serializing
  above 16 MiB, before allocating a pathological 400 MB model projection. Rust
  typed models remain accessible, and source/document rendering remains usable.
- Native export resolves CSS theme tokens, moves rounded rectangle radii into
  style declarations, and repeats first-tspan coordinates on parent text. AppKit
  otherwise drops such text despite a valid SVG accepted by browsers.

## Verified delivery lanes (2026-10-18)

Host: macOS 26.6 arm64 / Apple M1 Pro, Rust 1.98.1, Swift 6.4, Node 25.9.

- Public Rust integration coverage passes across legacy, preparation, extension,
  all six families and the flowchart parser checkpoint. No unit tests added.
- Source preparation: all 5,345 scenarios, including 761 examples from 34 upstream
  corpora; extension links: 1,536 entity/link scenarios.
- All four Swift integration tests pass. The additional-diagram test exercises
  every packet/radar/quadrant/timeline/journey fixture through owned documents,
  callbacks and source rendering; pie and preparation have their own full corpora.
- Thirty-three browser integration tests pass with one worker in Chrome, Firefox
  and WebKit, using the extracted npm tarball. Includes exact corpus comparisons,
  actual SVG pixels, browser Canvas/font behavior and callback error propagation.
- C executable passes NULL/UTF-8/limits/ownership checks: 400 legacy lifetimes plus
  200 owned pie document lifetimes.
- Rust crate packaging and independent downstream Cargo consumer pass.
- Formatting/whitespace checks pass for completed slices. In-progress flow layout
  helpers are not certified until their real public diagram path is verified.

AppKit decodes and paints current native exports. White proof backgrounds belong
only to raster evidence, while exported diagrams retain SVG transparency.

| Diagram | Native pixels | Additional text proof |
| --- | --- | --- |
| legacy flowchart | 416 x 789; 6,846 painted | Legacy outlined font path |
| pie | 358 x 296; 44,733 colored | Native compatibility export |
| packet | 1035 x 98; 6,053 painted | 456 title pixels |
| radar | 470 x 419; 35,281 painted | 159 title pixels |
| timeline | 408 x 266; 49,449 painted | 295 title pixels |
| journey | 340 x 327; 4,719 painted | 413 title pixels |
| quadrantChart | 500 x 500; 8,718 painted | 373 title pixels |

Packet has one additional stress scenario: 40,000-character label shared across
10,000 rows. Rust verifies every typed row and exact rendered SVG hash. Foreign
hosts verify a bounded projection error and successful exact source-render hash.
The 400 MB JSON projection is deliberately not built.

## Remaining acceptance work

- Twenty-seven family renderers, remaining shared behavior and the complete
  registration/lazy/mount/zoom/public model interface remain required.
- JavaScript model projections currently do not apply mutations back into the
  retained Rust document. Prototype methods/private enumerable state also need
  explicit compatibility decisions; full host parity is not claimed.
- Lone UTF-16 surrogate transport remains unresolved. Radar records two and
  quadrant three unrepresentable mutations explicitly; they are not passing cases.
  Init-JSON's explicit 64-level work cap, extreme radix rounding and non-finite/
  negative numeric host budgets also require broader conformance.
- The 16 MiB JSON projection cap is an explicit transport extension. Large typed
  models/source rendering work without that expanded projection.
- No new CI run, Intel execution, GPU acceleration or full-parity release is asserted.
  Initial four-family Chrome timings show slower current WASM public calls than
  Pele JS; see `PERFORMANCE.md`. Rust/GPU choices are not speedup evidence.

Development Swift APIs require source-built native artifacts. Root SwiftPM using
old v0.1 downloaded binaries excludes newer symbols. WASM needs a source build.
No replacement binary is substituted under the old release tag.
