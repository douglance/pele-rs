# Composable exact parse limits

`ParseLimits` adds independent source and edge overrides without changing
`ParseOptions`, `ParseContext`, the exported convenience models or existing
entry points. `None` retains that field's legacy integer option.
`Some(Undefined)` explicitly selects Pele's default.

```rust
use pele::compat::{Engine, EdgeLimit, ParseLimits, ParseOptions, SourceLimit};
let document = Engine::with_builtins().parse_with_limits(
    "flowchart LR\nA --> B",
    &ParseOptions::default(),
    ParseLimits {
        source: Some(SourceLimit::Number(f64::INFINITY)),
        max_edges: Some(EdgeLimit::Number(1.5)),
    },
)?;
```

`Engine` and `PublicEngine` expose `parse_with_limits` and
`parse_as_with_limits`. The latter selects a registered extension explicitly.
The new lane always checks the independent 16 MiB source-byte budget first.
Source `None` then uses the existing `usize` comparison without conversion to
floating point; an explicit source override uses the [source-limit contract](SOURCE_LIMITS.md).

Exact edge limits follow pinned Pele `458322a5` before each attempted push:
undefined resolves to 5,000; null compares as zero and prints `null`; a numeric
argument compares the existing edge count directly using `>=`. A threshold of
1.5 therefore permits two edges and rejects the third. Zero, negative and
negative infinity reject the first attempted edge, while edgeless diagrams
succeed. NaN and positive infinity disable this semantic comparison.

Flowchart and AgentFlow share their existing parser bodies with an explicit
private policy. Swimlane delegates to Flowchart and retains Flowchart's edge-error
type. Other built-ins retain their normal parsing behavior. Config cannot raise
this host option. An independent 1,000,000-edge generation ceiling remains on
the exact path, after the semantic check and before push. It reports
`Native edge generation exceeds work budget`; the old integer path is unchanged.
Generation is incremental: a large Cartesian product is not preallocated or
rejected before an earlier semantic threshold. This does not bound all renderer
allocation; other native graph/layout/text/work budgets still apply.

The parsed document privately retains its explicit edge override, available from
`ParsedDocument::edge_limit()`. Existing exported `PublicOrigin.max_edges: usize`
fields remain the legacy convenience projection. Additive provider hooks
`DiagramModule::parse_with_edge_limit` and
`PublicDiagramModule::create_public_with_edge_limit` carry exact policy without
changing public struct layouts. Default hooks forward existing implementations;
providers that privately enforce `ParseContext.max_edges` must override the new
parse hook themselves.

The built-in codecs seed exact `options.maxEdges` before deriving public state.
Undefined input resolves to a Number(5000); null and numeric IEEE bits remain
intact. After creation, graph options are authoritative. Applying or mutating
them never reapplies parse provenance. In particular, deleting the property or
assigning undefined disables the mutable semantic check instead of restoring
5,000. Existing native arena limits stay active. A fractional Cartesian method
can commit several edges before throwing; the normal operation outcome publishes
those edges, collection aliases and the advanced revision.

```swift
let document = try Pele.Engine().parseMutable(
    "flowchart LR\nA --> B",
    limits: .init(source: .number(.infinity), maxEdges: .number(1.5))
)
defer { try? document.close() }
let drawing = try document.render()
```

Swift `ParseLimits` has optional `source` and `maxEdges` fields, both nil by
default. The additive `pele_public_parse_limits_v7` ABI transports independent
source/edge tags and IEEE bits: 0 undefined, 1 null, 2 number, 3 legacy/None.
Only an explicitly overridden JSON field bypasses its old integer decoder.
Nonoverridden malformed fields still reject. Config, handle ownership, source
and options transport budgets remain intact. Both new Swift files are excluded
when using the checksum-pinned legacy released binary.

This covers numeric source/edge policies for well-formed source. Arbitrary
coercion, constructors, malformed UTF-16 and other public-runtime/browser/release
gates remain separate. Verification passes 32 focused Rust release integrations
(31 together and the million-edge case separately), five strict C programs and
22 Swift release integrations. Separate arm64 and Intel Rosetta SDK consumers
each pass nine limit tests, all 712 combined limit cases, 24 method steps, two
callback-order cases, C v7 integration and all 33 immutable/mutable families.
The legacy published Swift fallback also builds and runs. See
[full verification record](FULL_PARITY_PROGRESS.md) for artifact hashes and the
physical Intel/macOS 14 boundary.
