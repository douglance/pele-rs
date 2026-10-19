# Native low-level diagram entries

The source-built C v13/Swift interface exposes each named built-in's actual
low-level parser and renderer. It shares the Rust entry points used by web
`./diagrams/*`; it does not parse source a second time or implement diagrams in
Swift. This development API requires a matching local SDK.

```swift
let engine = Pele.Engine()
var context = Pele.PreparedParseOptions()
context.title = "Prepared title"
let model = try engine.parsePrepared("pie", source: "pie\n\"One\":1\n", options: context)
defer { try? model.close() }
let drawing = try model.render()
```

`PreparedParseOptions` supplies explicit config, optional title, a nonnegative
integer line offset and exact edge policy. Source passes verbatim: no core source
preparation, type detection, config merge, entity encoding or appended newline.
Raw undefined edge policy disables the semantic cap, while independent native
source/graph/work bounds remain. It differs from core's undefined/default 5,000.

`PreparedDocument` is an independent owner type and cannot be passed to Swift
`Engine.render`. It exposes native snapshot/apply/schema/invoke, retains its exact
handler, and frees the same public-document owner on close. `render` takes direct
configuration and render options, without core responsive defaults, theme
conversion, link decoding or semantic source/output limits. Core-only fields in
`Engine.Options` are ignored on this raw lane. The native result additionally
includes its retained diagram type; web's raw result has no type member.
All present scalar render numbers and returned dimensions retain their IEEE bits.
The independent 64 MiB returned-SVG and graph-transport bounds remain.

Bound host functions use the existing `HostFunction`/`ActiveModel` API through
`bindHostFunction`. There is no second binding table. Original native delegation,
callback errors, partial model edits and revised snapshots use the same owner as
ordinary mutable rendering. Revision publication precedes original Swift errors.

`Engine.createPrepared(type, graph:config:)` constructs a model directly from a
caller-owned graph. All 33 built-ins implement explicit graph constructors; no
source parse, empty parse, hidden DB prototype or parser history is injected.
`Engine.preparedSchema(for:)` discovers its schema without creating a document.
Graph aliases, cycles, UTF-16 strings, IEEE values and caller extras remain owned
by that model. Bind host functions on the resulting document and apply their
returned atoms before rendering. Invalid model data fails at the corresponding
render operation, preserving partial state and the pinned error boundary.

The additive C exports are `pele_parse_prepared_v13`,
`pele_render_prepared_v13`, `pele_create_prepared_v13` and
`pele_prepared_schema_v13`. Construction accepts a bounded binary graph exchange
with revision zero and no metadata, and copies its model into the new owner. The
Rust extension seam is `PublicDiagramModule::create_public_from_graph`;
`PublicEngine::create_from_graph_as` retains its actual registered provider.
The default extension implementation explicitly rejects unsupported construction. The optional 48-byte parse packet uses version 13,
zero reserved fields, explicit title presence and the existing raw numeric edge
tags. Successful parse returns an owned `PelePublicDocument`; existing graph,
schema, invocation and free functions apply. Packet validation and byte ceilings
precede borrowed input access. Input/config/title strings remain well-formed UTF-8;
config retains the existing JSON-object domain. Graph strings use the existing
lossless UTF-16 binary transport.

Actual ARM64 verification: 99 pinned complete-model/raw-SVG/callback vectors and
67 caller-built model vectors cover all 33 families. Swift reconstructs each
caller graph, binds its callable fields, and compares complete before/after
models, SVG, dimension bits, errors and ordered callbacks. Additional scenarios
cover raw edge/context policies, construction without parsing, aliases/cycles,
invalid input, stale revisions, ownership and close.

The combined Swift batch passes 16/16 (seven prepared/construction features,
five host-function features, four presentation features). The new clock feature
catches the original Swift clock error inside a captured Gantt method, then
recovers in that same callback. Strict C construction/ownership/packet/policy
checks pass. Root's combined Rust construction/entry/callback batch passes 9/9.
These tests use a fresh isolated ARM64 debug SDK incorporating the current core
and v12/v13 helpers. Its archive SHA-256 is
`7d4180858a7bd09271ab3fc822a10dece20babb0b51b1d667134e01a43745203`;
header SHA-256 is
`bcfa5583ae1f57e6451251954addf65c5744c6f9865a16d647c5b2d185cc9267`.
Matching universal release packaging remains a separate gate.
