# Development mutable model interfaces

The source-built development branch has all 33 diagram renderers. Sixteen families
expose mutable public models through Rust, Swift/C and browser WASM: Pie, Info,
Sankey, Packet, Radar, Quadrant, Treemap, Journey, XYChart, Kanban, Mindmap,
Timeline, Flowchart, Swimlane, ER and Architecture. Actual native model methods
are available where inventoried; field-only opaque schemas remain explicit until
their descriptor inventory is complete. The published v0.1.0 binary remains the
earlier flowchart subset. GitGraph, AgentFlow, Ishikawa, Class, EventModeling, TreeView, State and Venn also expose native Rust
mutable codecs, bringing native coverage to twenty-four; their foreign adapter
checkpoint is pending. AgentFlow has thirty-four actual descriptors with one
explicitly unavailable Error-valued return operation.

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
owned document. Unimplemented family codecs keep the existing immutable owned
model path; their renderers remain available.

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
reject stale edits before dispatch. C/WASM JSON crossings have a 64 MiB limit.
A post-call export/transport failure can leave native state advanced; only outcomes
whose graph fits the output budget can carry complete post-call state.

## Remaining compatibility boundary

This checkpoint does not certify complete Pele public API parity. Active
lone-surrogate source/text rendering, numeric options outside the current JSON
lane, constructors, exotic getter/proxy/coercion behavior, arbitrary overridden
callback dispatch, sparse collections in some codecs and some render-time error
classes remain open. Native graph/layout/text budgets are explicit platform
differences. Full source-free foreign model construction, live registry,
lazy/mount/zoom and final release/distribution checks also remain open.

See [progress](FULL_PARITY_PROGRESS.md), [public model LLD](LLD_PUBLIC_MODELS.md)
and [acceptance matrix](PELE_PARITY.md) for current evidence and release gates.
