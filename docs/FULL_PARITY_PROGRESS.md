# Full-library port: development evidence

Acceptance remains every row of `PELE_PARITY.md`: all 33 families and public APIs.
The public v0.1.0 release remains the legacy flowchart subset. No full-library
release or new GIF has been published from this branch.

## Implemented renderers

`Engine::with_builtins()` currently registers all thirty-three actual Rust diagram modules.
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
| state | 10,222 | 7,209 | 3,013 |
| gantt | 18,994 | 8,048 | 10,946 |
| sequence | 19,166 | 6,702 | 12,464 |
| gitGraph | 17,125 | 5,050 | 12,075 |
| architecture | 18,780 | 4,593 | 14,187 |
| sankey | 11,616 | 4,946 | 6,670 |
| treemap | 15,370 | 9,946 | 5,424 |
| c4 | 18,603 | 6,278 | 12,325 |
| info | 8,053 | 1,491 | 6,562 |
| requirement | 10,358 | 3,714 | 6,644 |
| block | 12,294 | 4,584 | 7,710 |
| railroad | 12,288 | 4,424 | 7,864 |
| cynefin | 7,209 | 3,011 | 4,198 |
| treeView | 6,800 | 4,301 | 2,499 |
| agentflow | 14,990 | 5,372 | 9,618 |
| wardley | 7,730 | 3,205 | 4,525 |
| eventmodeling | 6,953 | 2,437 | 4,516 |
| usecase | 16,744 | 3,424 | 13,320 |
| venn | 22,121 | 4,581 | 17,540 |
| ishikawa | 20,586 | 12,418 | 8,168 |
| swimlane | 10,405 | 4,568 | 5,837 |
| Total | 437,833 | 167,475 | 270,358 |

There are 172,247 complete model comparisons. Parsed models are retained when a
subsequent render fails. Six class stress cases explicitly exceed the JSON
projection depth/size budget; their typed models and exact source renders/errors
remain covered. These transport exceptions do not earn full public API parity.

These fixtures run through Rust, Swift/C and real browser WASM. One Venn error is an explicit native work-ceiling extension whose upstream render was deliberately not executed; it is not an upstream error-parity result. Available model projections and all errors
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
  all eighteen families, including full flowchart rendering. The consolidated
  release-profile run passed 50 public integration tests at the fifteen-family checkpoint; seven more GitGraph/Architecture/Sankey integration tests pass in release mode. No unit tests added.
- Source preparation: all 5,345 scenarios, including 761 examples from 34 upstream
  corpora; extension links: 1,536 entity/link scenarios.
- All seven Swift integration tests pass. The additional-diagram test exercises
  every packet/radar/quadrant/timeline/journey/flowchart/kanban/mindmap/class/er/xychart/state/gantt/sequence/gitGraph/architecture/sankey fixture through owned documents,
  callbacks and source rendering; pie and preparation have their own full corpora.
- Fifty-one browser integration checks pass with one worker in Chrome, Firefox
  and WebKit, using the extracted npm tarball. Includes exact corpus comparisons,
  actual SVG pixels, browser Canvas/font behavior and callback error propagation.
  The consolidated run passed 48 checks; the final three XYChart checks passed
  after its targeted tick-portability fix. The State/Gantt/Sequence batch adds
  twelve passing three-browser checks (full source/owned-model corpora and native
  SVG decoder pixels). Fifteen more checks compare Gantt in five actual browser
  timezones, including gaps/folds, half-hour changes, leap/month arithmetic,
  excluded weekends, ancient dates and Samoa's skipped day. The subsequent
  GitGraph/Architecture/Sankey batch passes fifteen additional three-browser checks:
  exact full corpora, native decoder pixels, and non-finite Sankey dimensions.
- C executable passes NULL/UTF-8/limits/ownership checks: 400 legacy lifetimes plus
  200 owned pie document lifetimes.
- Rust crate packaging and independent downstream Cargo consumer pass.
- Formatting/whitespace checks pass for completed slices. Shared graph/layout,
  shapes and edge helpers are exercised through actual public flowchart rendering.

AppKit decodes and paints current native exports. White proof backgrounds belong
only to raster evidence, while exported diagrams retain SVG transparency.

| Diagram | Native pixels | Additional text proof |
| --- | --- | --- |
| venn | 319 x 252; 48,377 painted | Intersections and 342 title pixels |
| ishikawa | 356 x 272; 6,991 painted | Fishbone branches and 445 title pixels |
| swimlane | 467 x 340; 10,323 painted | Lane routing and 472 title pixels |
| wardley | 846 x 610; 7,521 painted | Map, dependencies and 507 title pixels |
| eventmodeling | 418 x 263; 12,858 painted | Workflow lanes and 530 title pixels |
| usecase | 344 x 164; 4,207 painted | Actor, boundary and 362 title pixels |
| cynefin | 656 x 492; 77,124 painted | Five domains and 496 title pixels |
| treeView | 199 x 276; 4,477 painted | Files, nesting guides and 301 title pixels |
| agentflow | 620 x 100; 11,164 painted | Semantic workflow and 488 title pixels |
| requirement | 228 x 414; 7,895 painted | Tables, markers and 604 title pixels |
| block | 170 x 64; 1,390 painted | Grid nodes and routed arrow |
| railroad | 154 x 116; 1,271 painted | Alternatives, tracks and labels |
| treemap | 656 x 416; 242,303 painted | Hierarchy and 520 title pixels |
| c4 | 548 x 168; 4,989 painted | Elements, relation and 437 title pixels |
| info | 127 x 72; 1,106 painted | Pinned version and 419 title pixels |
| legacy flowchart | 416 x 789; 6,846 painted | Legacy outlined font path |
| pie | 358 x 296; 44,733 colored | Native compatibility export |
| class | 140 x 240; 3,904 painted | Members, relations and labels |
| er | 158 x 279; 4,467 painted | Attribute tables and cardinalities |
| xychart | 700 x 500; 81,821 painted | Bars, line, axes and 260 title pixels |
| state | 316 x 424; 5,636 painted | Composite states, arrows and 372 title pixels |
| gantt | 801 x 137; 14,583 painted | Task states, axes and 382 title pixels |
| sequence | 187 x 240; 4,492 painted | Messages, activation and 470 title pixels |
| gitGraph | 204 x 191; 4,812 painted | Commit symbols, branches and 491 title pixels |
| architecture | 155 x 128; 2,522 painted | Services, ports and 502 title pixels |
| sankey | 616 x 416; 224,041 painted | Flow bands and labelled nodes |
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

## Additional host transport evidence

Sankey can produce a NaN result height from subnormal positive link weights in
the pinned implementation. JSON alone erases that numeric value to null. Render
transport now includes exact IEEE bits only for dimensions JSON cannot preserve;
Swift and web adapters restore those numbers and hide transport metadata. Swift
Codable round-trips the same bits. Ordinary finite result envelopes are unchanged.
Full non-finite source options and model graphs remain separate required API work.

Architecture icon tests also exposed an empty-string resource distinction: an
explicit empty icon suppresses its builtin, while a missing icon selects fallback.
Swift now supplies a non-null zero-length buffer for an explicit empty icon;
all five previously failing cases pass, with nil/error ownership unchanged.

The new GitGraph/Architecture/Sankey native corpus run passed all cases except
those five icon cases; the complete Architecture corpus and all seven Swift test
methods pass after that correction. These are cumulative host checks, not a new
consolidated eighteen-family Swift corpus run. Independent Rust/Swift package
consumers were last exercised at the fifteen-family checkpoint; final all-family
package and CI gates remain mandatory.

## Treemap, C4 and Info checkpoint

Eight additional release Rust integration tests pass. The complete three-family
corpora pass through Swift/C source and owned-document rendering, and through
Chrome, Firefox and WebKit (nine corpus checks). All three browser SVG decoder
checks pass, with Info's small version label required to paint at least 500 pixels.
AppKit paints each current native export; all seven Swift integration test methods
pass in this checkpoint. This remains cumulative family coverage, with the final
unfiltered all-family run required before release.

Treemap retains one deep-model JSON projection limit; C4 retains eighteen cyclic
or deep projections. Typed models and exact source renders remain tested. C4's
five header forms are one family, with a dedicated layout and obstacle router.
Info reports the pinned upstream version rather than the Rust package version.
Treemap classes and C4 extra attributes are restored as Maps by the web adapter.

Swift fixture models now decode directly from oracle bytes with JSONDecoder.
Foundation's Any projection exposed false numeric comparison failures and stripped
a leading U+FEFF in a model string; direct decoding verifies the public binary64
values and exact Unicode strings. No fixture cases were removed.

## Requirement, Block and Railroad checkpoint

Eight additional release Rust integration tests pass. All 34,940 new public
scenarios pass Swift/C source and owned-document paths; all seven Swift test
methods pass. Twelve checks pass across Chrome, Firefox and WebKit, including
complete corpora and actual SVG decoder pixels. AppKit also paints all three.
The native smoke sample uses Requirement's supported frontmatter title syntax.

Block's three and Railroad's thirty-three explicit deep JSON projections remain
bounded; their concrete models and exact source renders stay covered. Railroad
implements IR, EBNF, ABNF and PEG as one family. Its cached unclosed-comment
scanner avoids repeatedly searching the same suffix. No projection exclusion
counts as completed mutable/lossless public API parity.

## Cynefin, TreeView and AgentFlow checkpoint

Ten new release Rust feature tests and four extension-contract checks pass.
The complete three-family corpora pass through Swift/C; the selected corpus
method passed again after adding per-case and per-family autorelease scopes.
Sampling the earlier run showed Foundation temporaries accumulating until test
exit. The fixture reader now releases that temporary work as each case completes;
no cases or assertions were dropped. Native AppKit exports paint all three.

The browser checkpoint passes twelve cumulative checks across Chrome, Firefox
and WebKit. A retained read-only query interface verifies every AgentFlow semantic
model through Rust erasure, C/Swift and WASM; custom modules keep their query
handler after registry replacement. Query names/arguments are bounded, and the
builtin semantic projection uses the existing model-output budget. This is an
additive development query interface, not completion of AgentFlow's mutable
`semanticModel()` prototype contract.

TreeView preserves per-row icon callback order. Its three deep JSON projections
and twenty-seven isolated UTF-16 input mutations remain explicit ledger entries.
AgentFlow records four metadata surrogate exceptions; Cynefin has none. These
exceptions are not counted as completed public string/API cases.

## Wardley, EventModeling and Usecase checkpoint

Nine new release Rust integration tests pass. The three complete host corpora
pass the native Swift/C source and retained-document paths, plus twelve
cumulative browser checks across Chrome, Firefox and WebKit. AppKit also paints
all three native exports. EventModeling preserves its required source comments;
Usecase Map fields and JSON property-order maps are restored in the web adapter.

Four Usecase projections exceed JSON byte/depth budgets. The foreign checks
require those projection limits, then separately verify successful source renders
or the exact expected render-output limit. Projection-budget labels are diagnostic
fixture metadata, not render-result fields. No case is skipped. Usecase keeps one
JSON isolated-surrogate exception; EventModeling keeps two isolated output
surrogate exceptions from otherwise valid source. Those remain full-API gates.
Usecase JSON error text follows the pinned Node/V8 oracle; browser-engine-specific
JSON.parse wording is not separately certified by these canonical fixture checks.

## Calendar and deep models

Gantt uses the translated V8 date parser for native fallback strings, attributed
under its BSD license. Browser rendering calls its actual host Date parser.
Firefox and WebKit differ from Node on 883 and 1,980 legacy-date corpus inputs;
those cases compare against the same pinned Pele library running in that browser,
with the same host resources. This is host parity, not fixture date injection.

Native civil arithmetic remains proleptic Gregorian Rust. Swift supplies timezone
offsets through the additive v3 C resource interface, preserving historical seconds;
Rust resolves gaps forward and repeated times to their first occurrence. Foundation
calendar extraction was rejected after ancient-date corpus mismatches. Forty public
scheduling scenarios pass in UTC, New York, Kolkata, Lord Howe and Apia. Host-error
identity, invalid calendar data and recovery also pass through the Swift bridge.

State retains deeply nested documents with iterative clone/drop. Eight State model
projections exceed the bounded JSON view; typed models and exact renders remain
covered. Sequence retains its deep menu JSON and shared actor boxes without the
expanded foreign projection. One deep-menu projection is explicitly bounded.
Two State inline-note cases and two Sequence menu-link cases produce isolated
UTF-16 output surrogates in upstream from otherwise well-formed input. Their exact
code units and upstream output hashes are recorded in exclusion ledgers. They do
not count as passing public string/API cases. The required mutable/lossless API
architecture is [LLD_PUBLIC_MODELS.md](LLD_PUBLIC_MODELS.md).

## Remaining acceptance work

- Three family integrations, remaining shared behavior and the complete
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


## All 33 renderer checkpoint

Venn, Ishikawa and Swimlane complete the family renderer inventory. Eight new
release-profile Rust integration slices pass; their 53,112 cases also pass through
Swift/C owned documents and source rendering, and Chrome, Firefox and WebKit WASM.
The targeted Swift corpus run takes 57.469 seconds. Three additional browser
checks decode and paint real SVG, and AppKit decodes all 33 family examples.
These targeted batch results do not replace the final consolidated release run.

Swimlane retains the real shared Flowchart model (`model.type === 'flowchart'`)
and dedicated lane renderer (`result.type === 'swimlane'`). Map hydration follows
the retained module, preserving this distinction. Ishikawa's four deep models
explicitly exceed JSON projection depth; their source render remains tested.

Venn uses translated circle intersections, overlap fitting and text geometry.
Pinned numeric oracle is Node 25.9 on darwin arm64; explicit fused operations and
fdlibm/V8 math preserve the tested native/WASM results. Required Sun/fdlibm notices
ship with Rust, Swift and web packages. Three finite decimal inputs overflow Venn
geometry into NaN, preserved by result-number transport rather than rasterized.
A Rust-only ceiling rejects more than 3,000,000 inferred pair iterations. The
2,450-set case verifies pinned parsing/model plus the native limit; its upstream
render was deliberately not executed. This one error is counted in the corpus
total as a platform extension, not an upstream render/error parity claim.

All public API gates remain open: mutable model fields/identity, real methods and
prototypes, lossless source/options transport, live registry, lazy/mount/zoom,
final independent consumers and distribution checks. The v0.1.0 release remains
the earlier limited flowchart API. No new release or GIF is published here.


## Mutable native model layer

Nine families now opt into `PublicDiagramModule`: Pie, Info, Sankey, Packet,
Radar, Quadrant, Treemap, Journey and XYChart. This is an additive native layer;
the immutable typed model API and released v1 contract remain available. Flat
model arenas preserve object IDs, aliases/cycles, Map/Set/Date values, absence,
undefined, UTF-16 strings, IEEE number bits and data-property flags. Typed native
module implementations own mutations and geometry. Existing documents retain
their registered codec/configuration after registry changes.

The three method-bearing codecs use explicit schemas; field-only codecs retain
opaque descriptors pending full inventory. Schema tables declare field, prototype and method IDs without recovering
contracts from serde projections. Journey exposes six actual methods, Quadrant
thirteen and XYChart thirteen. Calls publish the post-call graph and revision
even when a JavaScript or Pele operation error follows partial edits; stale or
invalid calls reject before dispatch. UTF-16 Pele errors preserve message, type
and snippet without lossy conversion, while ordinary errors keep their old form.
Custom extension integration tests exercise the same retained native dispatcher.

This layer does not certify foreign mutable APIs or complete model parity.
Constructors, arbitrary getter/proxy/coercion and overridden callback behavior,
active lone-surrogate rendering, sparse active collections in some codecs and
render-time JavaScript error categories remain explicit capability gaps. Native
work/projection ceilings also differ from unbounded JavaScript behavior. The
Swift/C and WASM mutable adapters are a separately verified layer in progress.

The frozen nine-codec native batch passes 57 release-profile integration tests
across 14 targets, including retained extension contracts and Packet/Quadrant
source-corpus regressions. Rust formatting and staged whitespace checks pass.


## Mutable host adapters checkpoint

The same nine native codecs now have additive C v4 owned handles, Swift
`parseMutable`/graph/schema/invoke APIs and browser arena hydration with real
prototype methods. Swift's six mutable-document integration tests pass, including
actual native method calls, partial errors, lossless extra values, nine edited
family SVG decoders and callback recovery. Strict C linkage/ownership tests pass
with 200 repeated documents, revision/input rejection and callback reentry.

Across Chrome, Firefox and WebKit, the 51 selected browser checks are covered by
the initial batch plus an 18-check focused correction run. Full nine-family source
and model corpora, changed model fields, method prototypes/arity/function shape,
shared function identity, partial failures, retained aliases, native SVG pixels
and browser Canvas resources are exercised. The corrections preserve Pele's
RangeError-to-limit guard, retain falsy host throws, align Firefox function key
order and render Packet's formerly oversized model through the flat graph.
Treemap's previous projection exception also renders through the new flat model.

JSON serialization bounds public output while writing, rather than allocating an
intermediary tree first. Native transport and post-call export limits remain
explicit boundaries. The TypeScript consumer slice passes strict checking with
TypeScript 4.9.5; this is a recorded local consumer check, not a CI compiler claim.
No full mutable API/release parity claim is made; see `MUTABLE_MODELS.md`.


## Twelve mutable native codecs

Kanban, Mindmap and Timeline add native mutable codecs, bringing this layer to
twelve families. Kanban and Mindmap expose their real own `getType` callable and
builder methods; Timeline exposes eight prototype methods, including its exact
`getCommonDb()` receiver identity. Sankey now exposes its two real methods and
lookup behavior, including partial-failure cache updates. Ordered own fields,
private stacks, aliases, detached values and data-property descriptors remain
part of the exported graph rather than a normalized JSON projection.

An additive shared-function descriptor identifies Kanban and Mindmap's imported
outline helper across schemas. Existing extension schemas remain source
compatible; registration validates aliases and retained documents keep their
original handler. Foreign adapters still expose the previously verified nine
codecs until their separate host checkpoint passes.

The frozen native batch passes 46 release-profile integration tests across
eleven targets, including all four affected historical corpora and shared
registration contracts. Native mutation SVG vectors and bounded-work recovery
are included. Constructors, active lone-surrogate rendering, arbitrary callback/
coercion behavior and each codec's documented collection limits remain open.


## Sixteen mutable native codecs

Flowchart, Swimlane, ER and Architecture add four more native codecs. Flowchart's
nineteen actual methods and private caches are reused by Swimlane with the real
shared schema, prototype identity and lane renderer. ER exposes ten methods with
entity/attribute/relationship aliases and ordered private group claims; Architecture
exposes six with real Map values, copied records and retained layout-hint arguments.

The frozen native batch passes 41 release-profile integration tests across eight
targets, including the complete affected source/render corpora, Flowchart and
Swimlane's pinned full graph/method vectors and native raster checks. All sixteen
codecs remain subject to their explicit constructor, callback/coercion, UTF-16
rendering and work-budget limits. The separately verified foreign adapter
checkpoint remains nine families while the expanded host checks run.


## Sixteen mutable codecs through Swift and web

The expanded C registry and WASM typed-model conversion expose the same sixteen
mutable codecs. Swimlane selection precedes the shared Flowchart model downcast,
so models keep the common FlowDb prototype while rendering real swimlanes. Shared
callable metadata makes Kanban and Mindmap's imported helper one host function;
disposing a later document does not break earlier live borrowed calls. Host
registrations retain weak document owners without retaining native handles.

The current source-built arm64 C archive passes strict C ownership/transport
checks. Eight Swift feature integration tests pass, including all sixteen edited
SVG decoders, actual Timeline operations/partial errors, shared metadata and
Swimlane's retained renderer. Chrome, Firefox and WebKit pass fifteen model/method
checks, twenty-four affected full-corpus checks and three decoder/pixel checks.
All three former deep Mindmap projection exceptions now render through the flat
public graph in each browser. The strict TypeScript 4.9.5 consumer also passes.

These local results cover the frozen adapter layer. They do not assert final CI,
Intel execution, complete mutable/API parity, new package publication or release.


## GitGraph mutable native checkpoint

GitGraph is the seventeenth native mutable codec. Its actual returned value is a
plain object with no model prototype methods; no synthetic builder API is added.
Commit/branch Maps, head aliases, present undefined merge fields and own-key order
are retained. The concrete Git renderer consumes the edited graph with the same
23 pinned mutation SVG comparisons and all 17,125 historical outcomes (5,050 SVG
results and 12,075 errors), with zero corpus exclusions.

Eight public feature tests plus two baseline integration tests pass in release
profile. Default iterators, finite/infinite sequence ordering, inactive fields,
UTF-16 extras, native raster/resource recovery and bounded alias expansion are
covered. NaN/nonnumeric sorting, exotic callbacks/coercion, active sparse/isolated
UTF-16 text and some render-error classes remain explicit gaps. The verified
foreign mutable checkpoint remains sixteen families.


## Imported callable construction

Native function-shape descriptors distinguish ordinary constructible functions
from class methods. Kanban and Mindmap's shared outline helper keeps its real
prototype object, constructor backlink, arity and construction behavior; class
methods remain nonconstructible. This closes the imported-helper construction
gap without claiming model constructors. Shape/identity registration validation
is linear and bounded, and retained documents survive invalid replacement metadata.

Three native extension integration tests pass in release profile alongside the
GitGraph batch; all eight Swift mutable feature tests pass with shape metadata.
The consolidated eighteen browser model/method tests pass in Chrome, Firefox and
WebKit. An initial Firefox own-property ordering failure was corrected by retaining
the helper's intrinsic two-argument length rather than rewriting it.


## Public browser error constructor

The development core/full entries export the same `PeleError` constructor, with
Pele's message/code/info defaults, actual Error inheritance, ordered fields,
descriptors and raw UTF-16/IEEE values. `PeleCompatibilityError` remains an alias
for existing catch checks. Native parse/render/method errors and the RangeError
guard produce this same public prototype. Transport decoding is separate from
the public string constructor. Host-generated stack/location values are excluded
from cross-file equality; semantic Pele line/column fields remain checked.

The packed browser package passes all twenty-one error and mutable-model checks
in Chrome, Firefox and WebKit. Explicit class-field declarations preserve native
field ordering and WebKit's enumerable line/column descriptors. The strict
TypeScript 4.9.5 consumer passes. This does not close model constructors or the
remaining native Error-value arena seam.


## AgentFlow and Ishikawa mutable native checkpoints

AgentFlow and Ishikawa bring native mutable coverage to nineteen families. The
frozen batch passes nineteen release-profile integration tests across four
targets, including all 14,990 AgentFlow and 20,586 Ishikawa historical outcomes.
AgentFlow also verifies all 5,373 historical semantic-model results and 114 raw
graph cases/194 operations/226 complete SVG hashes. Its original parsed drawing
graph remains cached: root label/graph edits can stay visually stale, while
detached original style-array edits remain live. Four astral metadata projections
now work through the public identity graph without changing typed convenience
serialization. Retained registration, native pixels and bounded expansion recover.

AgentFlow exposes all thirty-four actual method descriptors and thirty-three
native implementations. Its private `shapeError()` returns a genuine Error
instance upstream; that one returned-value operation remains explicitly
unavailable until the Error arena seam lands. Builder shape failures already
throw real Pele errors. Arbitrary overrides/coercion, options.log callbacks,
active lone-surrogate SVG, some numeric provenance and render TypeError transport
remain recorded runtime gaps.

Ishikawa exposes the real addNode method, stack/baseLevel bookkeeping and aliased
tree. Sixteen pinned wide/narrow mutation SVGs, deep flat round trips, partial
stack failures and native resource/pixel recovery pass. Caller-added accessibility
extras are retained but ignored, matching the actual upstream renderer. No
renderer changes are needed. Active missing/sparse/lone-surrogate node text and
cyclic/over-budget trees retain explicit boundaries. Foreign mutable coverage
remains the separately verified sixteen-family checkpoint.


## Class and EventModeling mutable native checkpoints

Class and EventModeling bring native mutable coverage to twenty-one families.
The consolidated native batch passes thirty-four release-profile integration
tests across nine targets, including source baselines, retained extension methods,
shared callables and Venn dimension-transport regressions. Foreign mutable
coverage remains the verified sixteen-family checkpoint.

Class exposes twenty-three root and four nested ClassMember methods, full
namespace/class/note aliases, arbitrary numeric relation markers and note IDs,
and ordered optional fields. Its 29 mutation vectors cover 114 native calls:
104 whole SVG checkpoints plus ten UTF-16 storage/return checks with explicit
active SVG gaps. Forty thousand actual member-builder inputs are checked against
pinned results. The original 24,758-case source corpus remains green. The mutable
lane matches 24,756 cases; two enormous namespace cases hit the 16M-unit graph
transport limit before upstream's 4M-character SVG limit. Exact case IDs, source
hashes and both errors are asserted, rather than counted as parity. The six
legacy projection exclusions remain explicit. Namespace-parent capture, exact
style compaction and a negative lexer-direction cache preserve valid stress
cases. Cross-document lineType/relationType identity remains open.

EventModeling preserves its real plain-object model and shared lane/box/relation
objects. All 6,953 historical outcomes and thirty pinned mutation SVGs pass.
Edited duplicate box IDs now resolve note/spec headings to the last matching
name, matching upstream. Two generated UTF-16 hard-wrap output gaps remain
explicitly outside corpus success. Both codecs retain their documented exotic
coercion/callback, active sparse/UTF-16, render-error and work-budget boundaries.


## Stateful render operation primitive

The additive native `PublicDocument::render_operation` publishes post-render
graph state and advances its revision after dispatched success or failure.
Modules can preserve native JavaScript error categories through an optional
trait override; existing modules retain their implementation by default.
Stale revisions reject before dispatch. Nested results retain exact non-finite
dimension bits. Three integration slices cover retained handlers, partial edits,
callback failure, output limits, recovery and invalid post-operation graphs;
all pass in the thirty-four-test release batch above. The old native render API
remains unchanged. Foreign adapter validation and pre-callback synchronization
remain separate pending work.


## TreeView mutable native checkpoint

TreeView brings native mutable coverage to twenty-two families; foreign
coverage remains sixteen. Its returned plain wrapper preserves the virtual root,
optional own fields and shared node identities without inventing private builder
methods. Seven public integration slices and four source baseline slices pass in
release mode: all 6,800 historical outcomes, twenty-eight pinned mutation/config
SVG comparisons, repeated icon callback traces, deep/DAG graphs, native pixels,
resource failures and recovery. Existing twenty-seven source UTF-16 exclusions
and three JSON projection ceilings remain explicit. Active sparse/lone-surrogate
text, exotic coercion/prototypes and cyclic or over-budget expanded trees remain
recorded gaps. Existing parser, model and renderer sources are unchanged.


## State mutable native checkpoint

State brings native mutable coverage to twenty-three families; foreign coverage
remains sixteen. All eighteen real StateDb getter/method entries operate on its
identity graph and retained caches. Forty-six complete graph vectors cover
eighty-three operations and eighty-eight complete SVGs, including explicit
rebuilds, detached style aliases, direction getters and post-error recovery.
Four public and two baseline release integration slices pass all 10,222 historical
outcomes, including repeated-class/description stress and eight deep source
cases through the flat public graph. Superseded builder scratch objects stay
private; existing 1M-item/8M-work/16M-text budgets are unchanged. The source model
only adds skipped divider-counter provenance. Constructor/override/coercion,
active sparse/nonstring renderer data, lone-surrogate rendering and legacy
render-error category boundaries remain explicit.
