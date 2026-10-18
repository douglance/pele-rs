# Full-library port: development evidence

Acceptance remains every row of `PELE_PARITY.md`: all 33 families and public APIs.
The public v0.1.0 release remains the legacy flowchart subset. No full-library
release or new GIF has been published from this branch.

## Implemented renderers

`Engine::with_builtins()` currently registers twelve actual Rust diagram modules.
`Engine::empty()` and typed `DiagramModule<Model>` remain extension interfaces.
Registration never certifies a global parity flag; all public API gates remain open.

| Family | Public scenarios | SVG results | Errors |
| --- | ---: | ---: | ---: |
| pie | 3,678 | 1,179 | 2,499 |
| quadrantChart | 10,614 | 1,586 | 9,028 |
| packet | 12,774 | 2,924 | 9,850 |
| radar | 8,431 | 1,791 | 6,640 |
| timeline | 11,932 | 8,017 | 3,915 |
| journey | 10,725 | 3,244 | 7,481 |
| flowchart | 18,175 | 8,952 | 9,223 |
| kanban | 11,681 | 5,290 | 6,391 |
| mindmap | 11,335 | 4,204 | 7,131 |
| class | 24,758 | 8,746 | 16,012 |
| er | 17,437 | 7,535 | 9,902 |
| xychart | 9,886 | 3,705 | 6,181 |
| Total | 151,426 | 57,173 | 94,253 |

There are 57,569 complete model comparisons. Parsed models are retained when a
subsequent render fails. Six class stress cases explicitly exceed the JSON
projection depth/size budget; their typed models and exact source renders/errors
remain covered. These transport exceptions do not earn full public API parity.

These fixtures run through Rust, Swift/C and real browser WASM. Available model projections and all errors
compare completely. SVG compares byte-for-byte, or by SHA256 of every UTF-8 byte
for large outputs. Extractors pin the upstream revision and source-file hashes.

The actual parsers/models/layouts/renderers are translated, including narrow
adaptation, configuration, accessibility and host resources. There is no upstream
JavaScript renderer in distributed Rust/native/WASM code. JavaScript is a
build/test oracle and browser font/resource adapter only.

Full flowchart now uses the actual shared layered/compound, shape/edge geometry,
graph normalization and SVG renderer. Its 47,917-edge typed stress document also
passes foreign model projection. See `FLOWCHART_PORT.md` for declared-model and
private-cache scope. Kanban and mindmap share the translated outline/icon parser;
Rust owns all branch/column geometry and responsive behavior.

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
  all twelve families, including full flowchart rendering. The consolidated
  release-profile run passed 42 public integration tests after the numeric fix. No unit tests added.
- Source preparation: all 5,345 scenarios, including 761 examples from 34 upstream
  corpora; extension links: 1,536 entity/link scenarios.
- All four Swift integration tests pass. The additional-diagram test exercises
  every packet/radar/quadrant/timeline/journey/flowchart/kanban/mindmap/class/er/xychart fixture through owned documents,
  callbacks and source rendering; pie and preparation have their own full corpora.
- Fifty-one browser integration checks pass with one worker in Chrome, Firefox
  and WebKit, using the extracted npm tarball. Includes exact corpus comparisons,
  actual SVG pixels, browser Canvas/font behavior and callback error propagation.
  The consolidated run passed 48 checks; the final three XYChart checks passed
  after its targeted tick-portability fix.
- C executable passes NULL/UTF-8/limits/ownership checks: 400 legacy lifetimes plus
  200 owned pie document lifetimes.
- Rust crate packaging and independent downstream Cargo consumer pass.
- Formatting/whitespace checks pass for completed slices. Shared graph/layout,
  shapes and edge helpers are exercised through actual public flowchart rendering.

AppKit decodes and paints current native exports. White proof backgrounds belong
only to raster evidence, while exported diagrams retain SVG transparency.

| Diagram | Native pixels | Additional text proof |
| --- | --- | --- |
| legacy flowchart | 416 x 789; 6,846 painted | Legacy outlined font path |
| pie | 358 x 296; 44,733 colored | Native compatibility export |
| class | 140 x 240; 3,904 painted | Members, relations and labels |
| er | 158 x 279; 4,467 painted | Attribute tables and cardinalities |
| xychart | 700 x 500; 81,821 painted | Bars, line, axes and 260 title pixels |
| full flowchart | 115 x 336; 3,338 painted | Nested group and resolved title halo |
| kanban | 428 x 105; 3,069 painted | Columns and card labels |
| mindmap | 269 x 73; 1,241 painted | Root and both branches |
| packet | 1035 x 98; 6,053 painted | 456 title pixels |
| radar | 470 x 419; 35,281 painted | 159 title pixels |
| timeline | 408 x 266; 49,449 painted | 295 title pixels |
| journey | 340 x 327; 4,719 painted | 413 title pixels |
| quadrantChart | 500 x 500; 8,718 painted | 373 title pixels |

Packet has one additional stress scenario: 40,000-character label shared across
10,000 rows. Rust verifies every typed row and exact rendered SVG hash. Foreign
hosts verify a bounded projection error and successful exact source-render hash.
The 400 MB JSON projection is deliberately not built.

## Numeric portability

Host comparisons found two failures that the original Rust JSON oracle comparisons
had masked. `serde_json` now enables `float_roundtrip`, preserving decimal values
through `Value::Number` serialization instead of moving some extreme values by one
ulp. WASM's libc `pow(10, integer)` also differed from the pinned JS engine, changing
XYChart tick strings and label margins. Integer decimal powers now use exact
string-to-f64 conversion; all 633 finite exponent cases from -324 through 308 were
checked against the development JS oracle. The public XYChart corpus passes on
native Rust, Swift/C and all three browser engines after both corrections.

Pinned XYChart explicitly clamped equal bounds can still emit NaN path geometry;
that upstream behavior is preserved and does not constitute a universal finite-SVG
claim. Ordinary diagrams have XML and real raster evidence.

## Remaining acceptance work

- Twenty-one family renderers, remaining shared behavior and the complete
  registration/lazy/mount/zoom/public model interface remain required.
- JavaScript model projections currently do not apply mutations back into the
  retained Rust document. Prototype methods/private enumerable state also need
  explicit compatibility decisions; full host parity is not claimed.
- Lone UTF-16 surrogate transport remains unresolved. Radar records two and
  quadrant three and mindmap seven unrepresentable mutations explicitly; they are not passing cases.
  Init-JSON's explicit 64-level work cap, extreme radix rounding and non-finite/
  negative numeric host budgets also require broader conformance.
- The 16 MiB JSON projection cap is an explicit transport extension. Class also
  guards deep repeated namespace projections. Large typed
  models/source rendering work without that expanded projection. Mindmap also
  preflights depth 48 before recursive model encoding. Its depth 80, 310 and 3,000
  source renders match exact SVG hashes in Rust, Swift/C and all browser engines.
- No new CI run, Intel execution, GPU acceleration or full-parity release is asserted.
  Initial four-family Chrome timings show slower current WASM public calls than
  Pele JS; see `PERFORMANCE.md`. Rust/GPU choices are not speedup evidence.

Development Swift APIs require source-built native artifacts. Root SwiftPM using
old v0.1 downloaded binaries excludes newer symbols. WASM needs a source build.
No replacement binary is substituted under the old release tag.
