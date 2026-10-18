# Exact source-limit API

The additive source-limit lane implements Pele `core.ts` at revision
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`. It compares the original source's
UTF-16 length before BOM removal, newline normalization, frontmatter,
preprocessing or diagram detection.

`SourceLimit` has three cases: `Undefined` selects 50,000; `Null` compares as
zero but retains `null` in error text; `Number(f64)` compares directly without
integer rounding. NaN and positive infinity disable the Pele comparison;
negative infinity rejects even empty source. Negative zero prints as `0`.
Existing JavaScript number formatting supplies the original threshold in errors.

An independent 16 MiB UTF-8 source-byte budget runs first. It matches the existing
C input budget and cannot be disabled by a number-valued Pele threshold. The new
Rust entry points apply this same native boundary before preprocessing. Legacy
Rust entry points keep their existing caller-controlled integer behavior.

```rust
use pele::compat::{SourceLimit, public::PublicEngine};
let engine = PublicEngine::with_builtins();
let document = engine.parse_with_source_limit(
    "info", &Default::default(), SourceLimit::Number(f64::INFINITY),
)?;
```

Immutable `Engine` also supplies `inspect_with_source_limit`,
`parse_with_source_limit` and `parse_as_with_source_limit`. `PublicEngine` supplies
both parse variants, including explicit extension-language dispatch. These reuse
one preparation body and the existing typed module interface.

```swift
let document = try Pele.Engine().parseMutable(
    "info", sourceLimit: .number(.infinity)
)
try document.close()
```

The new Swift argument overrides only `options.limit`. Config and the integer
`maxEdges` contract remain intact. Numbers cross the additive
`pele_public_parse_source_limit_v6` C entry point as IEEE bits with a separate
undefined/null/number tag; they never pass through JSON number encoding. Handle
ownership, error cleanup and subsequent binary graph operations match v4/v5.
Existing calls and the published v0.1 Swift package remain compatible.

This lane accepts well-formed Rust/Swift strings and C UTF-8. It does not close
malformed UTF-16 source or SVG output, general source/options transport,
number-valued edge/output limits, arbitrary coercions, constructors or callback
compatibility. The global full-parity/release gate remains open.

The development oracle generator is `scripts/parity/extract-source-limit.mjs`;
`fixtures/pele/source-limit.json` records the pinned 207-case matrix. Native,
actual C and Swift integration checks exercise comparison order, original error
fields, nonfinite values, Unicode boundaries, native budgets, retained config,
extension dispatch, ownership and recovery. Verification passed: 19 focused Rust
release integration tests; strict C source-limit, mutable and legacy integration;
16 Swift release integration tests; and separate arm64/Intel Rosetta runs of all
207 source-limit cases, the C v6 integration and all 33 diagram consumers.
The actual released v0.1 fallback also builds and runs through a separate consumer.
See `FULL_PARITY_PROGRESS.md` for the artifact fingerprint and runtime boundary.
