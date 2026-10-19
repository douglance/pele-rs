# Exact output-limit API

The additive native output-limit lane follows Pele `core.ts` at revision
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`. Rendering runs first; the engine then
compares the completed SVG's UTF-16 length with the selected threshold before
decoding returned link IDs.

`OutputLimit::Undefined` and `OutputLimit::Null` both select 4,000,000. This
nullish default differs from the source-limit API. `Number(f64)` compares directly:
fractions retain their value, equality passes, NaN and positive infinity disable
the Pele comparison, and negative infinity rejects even an empty SVG. Negative
zero prints `0`. Errors retain the document's registered diagram type.

```rust
use pele::compat::{Engine, OutputLimit, RenderOptions, text::MetricsResources};
let engine = Engine::with_builtins();
let drawing = engine.render_source_with_output_limit(
    "info", &RenderOptions::default(), OutputLimit::Number(f64::INFINITY),
    &mut MetricsResources,
)?;
```

`Engine::render_document_with_output_limit` renders a retained typed document.
`PublicDocument::render_operation_with_output_limit` returns the post-operation
graph and revision, including when rendering or the final output check throws.
Stale revisions reject before dispatch. Graph export/transport failures retain
their existing boundary and may prevent publication of state already advanced.

```swift
let document = try Pele.Engine().parseMutable("info")
defer { try? document.close() }
let result = try document.render(outputLimit: .number(.infinity))
let outcome = try document.renderOperation(outputLimit: .number(0.5))
// outcome carries the output-limit error and the advanced graph/revision.
```

The Swift overloads share existing outcome decoding and host-error identity
handling. `pele_public_render_operation_output_limit_v6` transports the explicit
argument as an undefined/null/number tag and IEEE bits, independently of bounded
options JSON. The explicit argument overrides only the old `outputLimit` field;
other options retain their normal decoder. C metadata uses a capped writer before
the existing binary exchange budget check. These are additive APIs; the legacy
Swift binary fallback excludes the new source file.

Class has a separate explicit exact-render entry that shares its normal geometry
but bypasses the old caller-limit preflight. A throwing measurement callback
therefore precedes a small/negative output limit, as in Pele. Legacy entry points
retain their existing integer options and preflight behavior.

Extension providers retain the same concrete model and registration ownership.
New default `DiagramModule::render_with_output_limit` and
`PublicDiagramModule::render_public_operation_with_output_limit` hooks forward
to the previous render hooks. Providers that privately enforce the legacy
integer limit must override the exact hook; forwarding cannot translate private
provider policy. The shared engine owns the final exact comparison. Output
policy is never hidden in host resources or global state.

The new lane independently rejects returned SVG larger than 64 MiB of UTF-8,
both before and after optional native-theme resolution. Class may reject early
when its escaped-ID byte lower bound proves this native ceiling exceeded. Such
errors say `Native render output exceeds byte budget`; NaN and infinity cannot
disable the ceiling. This bounds accepted output, not peak allocation inside a
trusted renderer. Existing source, graph, layout, work and binary transport
budgets remain active.

This layer covers well-formed output strings and number-valued output limits.
It does not close arbitrary options coercion, malformed UTF-16 SVG, numeric edge
limits, constructors or the other full-runtime/release gates. Verification passes
27 focused Rust release integration tests, 19 Swift release integration tests,
strict C integrations and separate arm64/Intel Rosetta checks. The pinned oracle
has 192 complete output outcomes plus two callback-error ordering vectors; every
case checks whole SVG/error fields, measurements and retained mutable state.
See `FULL_PARITY_PROGRESS.md` for the artifact fingerprint and platform boundary.
