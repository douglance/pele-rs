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

The additive C exports are `pele_parse_prepared_v13` and
`pele_render_prepared_v13`. The optional 48-byte parse packet uses version 13,
zero reserved fields, explicit title presence and the existing raw numeric edge
tags. Successful parse returns an owned `PelePublicDocument`; existing graph,
schema, invocation and free functions apply. Packet validation and byte ceilings
precede borrowed input access. Input/config/title strings remain well-formed UTF-8;
config retains the existing JSON-object domain. Arbitrary caller-built models
require the separate per-codec graph construction layer.

Actual ARM64 verification: 99 pinned full-model/raw-SVG/callback vectors across
all 33 families, raw edge/context policies, native number transport, host failures
and recovery, and a bound Gantt method on the raw lane. The combined Swift batch
passes 12/12 (four prepared, four host-function, four presentation features), and
the strict C ownership/packet/policy scenario passes. These tests use a fresh
isolated SDK incorporating the current core and v12/v13 helpers; final matching
universal packaging remains a release gate.
