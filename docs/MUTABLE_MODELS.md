# Development mutable model interfaces

The source-built development branch has all 33 diagram renderers. All 33
families expose mutable public models through Rust, Swift/C and browser WASM.
The final browser batch adds Venn, Sequence, Gantt, C4, Requirement, Usecase,
Cynefin, Railroad, Wardley and Block. Full API release verification remains open.
Methods are available where inventoried. All thirty-four AgentFlow method
descriptors now have native implementations, including returned Error values.
The published v0.1.0 binary remains the earlier flowchart subset.

## Browser

Build `scripts/build-wasm.sh` first. Use the extracted npm package or a bundler
that serves its WASM asset. The adapter owns JS values, prototypes and lifetimes;
Rust owns diagram state transitions, validation, layout and SVG generation.

```js
import {loadRenderer, metricsMeasurer} from '@unlocalhosted/pele-rs/full';
const renderer = await loadRenderer();
const model = renderer.parse('journey\nsection Core\nBuild:4:Swift');
model.addSection('Verify');
model.addTask('Browser', ':5: Web, Swift');
model.title = 'Shared native methods';
console.log(model.getActors());
const result = renderer.render(model, {measurer: metricsMeasurer});
renderer.dispose(model);
```

Map/array edits and replacements preserve retained aliases. Methods publish
partial edits before throwing, matching the tested native operations. Function
identity is shared across documents of one schema. Kanban and Mindmap also
share the imported outline helper across schemas. Flowchart and Swimlane share
the actual FlowDb prototype and all nineteen methods; Swimlane's retained render
result remains `swimlane` while its model type remains `flowchart`. The imported
outline helper also retains its ordinary constructible-function prototype and
constructor backlink; class methods remain nonconstructible. Model constructors and mutation of
prototype definitions report an unavailable capability. Methods require a live
owned document. Every built-in family uses the mutable owned model path.

`PeleError` is exported by both development entries with the upstream
message/code/info constructor. `PeleCompatibilityError` aliases the same class;
native failures support `instanceof PeleError`. Host-generated stack locations
remain specific to each runtime.

Host callback exceptions follow the pinned public render guard: RangeError
becomes PeleError with code `limit`; every other thrown JS value retains identity,
including undefined, null and false. This differs from calling a model method
directly, where native JavaScript error categories remain operation results.

## Swift

Run `scripts/build-native.sh` and use `bindings/swift`, or the root package with
its locally built XCFramework. Mutable documents are confined to one owner/actor.
Explicit graph edits preserve aliases, cycles, UTF-16 code units, IEEE bits and
data-property flags without converting the model through JSON value projections.

```swift
let document = try Pele.Engine().parseMutable("pie\n\"Swift\":2")
var snapshot = try document.snapshot()
let title = try snapshot.graph.intern("Shared core")
try snapshot.graph.setField("title", value: .string(title))
try document.apply(snapshot.graph, expectedRevision: snapshot.revision)
let result = try document.render()
try document.close()
```

Array edits use `setArrayElement(_:value:on:)` and `setArrayLength(_:on:)`.
`setField` routes canonical numeric indexes and `length` through the same checked
operations, so appends are rendered and truncation removes the right elements.
These are transactional Swift graph edits: readonly/nonconfigurable structural
failures leave the graph unchanged. They do not emulate JavaScript's partial
Array-length assignment failures. Non-index fields remain ordinary properties.

`schema()` describes native method IDs. Returned Errors use
`PublicGraph.Node.error(errorType:properties:)`, keeping eight runtime brands,
raw UTF-16 fields, descriptor flags and graph reference identity. They are values,
separate from thrown operation failures.

`schema()` describes native method IDs. `invoke(method:receiver:arguments:)`
returns graph, revision and either a returned atom or a typed operation error.
Inspect the returned graph even after an operation error: preceding edits remain.
`renderOperation()` returns the post-render graph, revision and rendered value or
typed operation error. The `render()` convenience also advances the revision on
dispatched success or failure. Callback exceptions preserve their original Swift
identity after the revision is published. Graph strings use `Pele.JSString`; conversion to Swift String is explicitly fallible
for isolated surrogate code units. Native host errors retain Swift error identity.

C owns the same documents through additive `pele_public_*_v4` operations in
`include/pele.h`. Each result and document has one explicit free operation.
Concurrent/reentrant use rejects; freeing an actively used handle is invalid.

## Native extension contract

`PublicDiagramModule` augments `DiagramModule` with a typed mutable state,
explicit export/apply/render operations, optional schemas and native method
invocation. `PublicEngine::register` and source-free
`PublicDocument::from_module` retain the exact handler/configuration. Schema and
strong method/prototype IDs describe the foreign view without reconstructing a
contract from serde output. Old typed immutable models remain usable.

Flat graph version 1 transports object/array/Map/Set/Date identity, UTF-16 strings,
undefined/null, number bits and property flags. Default array length is implicit;
a readonly length uses its nondefault own `length` property descriptor. Revisions
reject stale edits before dispatch. C/WASM binary graph exchanges have a 64 MiB default byte ceiling. Legacy JSON
graph operations remain additive; source/options and schema metadata still use
their bounded JSON lane.
The default foreign model lane uses binary identity tables. Both large Class
style models previously blocked by JSON export now round-trip in Swift and all
three browsers without raising budgets. Two separate Class namespace cases still
exceed the native 16M-unit graph budget. Exact source hashes/errors are checked
separately from parity. A post-call
export/transport failure can leave native state advanced; only outcomes
whose graph fits the output budget can carry complete post-call state.

## Owned native registry

Source-built Swift consumers can own an empty or complete registry, then select
individual built-ins through the shared Rust module interface:

```swift
let registry = try Pele.Registry.empty()
try registry.registerBuiltin("flowchart")
let document = try registry.parseMutable("flowchart LR\nA --> B")
try registry.close()
defer { try? document.close() }
let drawing = try document.render()
```

`allBuiltins()` installs all 33. `registered`, `capabilities`, `detectType` and
`supports` query that owner. IDs are exact canonical strings; unknown registration
fails. Detection and support preprocess without checking grammar or the default
parse threshold. Native transport byte budgets remain active. Mutable documents
retain their handler, schema and config after replacement or registry closure.
The class is deliberately not Sendable; confine it to one owner or actor. Existing
`Pele.Engine` convenience APIs retain their behavior. See the
[registry architecture](LLD_NATIVE_REGISTRY.md) for C ownership and extension seams.

## Remaining compatibility boundary

The additive Rust/C/Swift [source-limit API](SOURCE_LIMITS.md) now handles
undefined, null, fractions, NaN and infinities using the original UTF-16 source
length. The [output-limit API](OUTPUT_LIMITS.md) compares exact numbers after
rendering and preserves state/revision on operation errors. The composable
[parse-limit API](PARSE_LIMITS.md) independently overrides source and edge limits,
including fractional Cartesian expansion, nonfinite numbers and mutable
`options.maxEdges`. Independent native byte and generation budgets stay active.
General options coercion retains its existing boundary.

This checkpoint does not certify complete Pele public API parity. Active
lone-surrogate source/text rendering, numeric options outside the current JSON
lane, constructors, exotic getter/proxy/coercion behavior, arbitrary overridden
callback dispatch, sparse collections in some codecs and some render-time error
classes remain open. Native graph/layout/text budgets are explicit platform
differences. Full source-free foreign model construction, live registry,
lazy/mount/zoom and final release/distribution checks also remain open.

See [progress](FULL_PARITY_PROGRESS.md), [public model LLD](LLD_PUBLIC_MODELS.md)
and [acceptance matrix](PELE_PARITY.md) for current evidence and release gates.
