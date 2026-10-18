# Native render-number transport and bounded orchestration

Status: implementation in progress; acceptance pending. Architectural decision: retain v9
legacy output dispatch for tag 3, exact dispatch for tags 0/1/2; bound new v9 output
independently. Target Pele 1.6.0 / 458322a5b8abb1ae57ddf127c32017ab9fb14a92.

## 1. Contract and scope

Transport the five existing RenderSettings f64 option fields without JSON number
loss: fontSize, maxWidth, directionBreakpoint, padding, now. Preserve their
existing Rust Option<f64> fields and Swift Double? properties. A present number is
never clamped, rounded, converted to a string, or rejected merely for being
nonfinite. Absent packet bits leave the corresponding JSON decoder authoritative.

Add three C v9 rendering endpoints: immutable source, immutable document, mutable
render operation. Swift's existing render overloads use them; existing C exports
and Rust render methods retain their semantic dispatch. Add source/document Swift
OutputLimit overloads so the new transport composes with the existing enum.

This layer does not change parse limits, Config transport, string/UTF-16 execution,
custom providers, constructors, asynchronous loading or view lifecycle. Exact
source/edge limits continue through existing parseMutable(limits:) followed by
render. A source-render call still uses its current typed parse option contract.

Evidence: upstream src/types.ts RenderOptions; src/diagrams/gantt/db.ts clock;
src/diagrams/gantt/render.ts renderGantt; current compat/module.rs RenderSettings,
compat/engine.rs RenderSession, FFI calendar.rs/public.rs, Swift Engine.swift and
MutableDocument.swift.

## 2. C packet

Declare in include/pele.h, mirror with repr(C) in new FFI render_numbers.rs:

```c
typedef struct PeleRenderNumbersV9 {
    uint32_t version;                 /* 9 */
    uint32_t struct_size;             /* sizeof(PeleRenderNumbersV9) == 56 */
    uint64_t present;                 /* only bits 0..4 */
    uint64_t font_size_bits;          /* bit 0: fontSize */
    uint64_t max_width_bits;          /* bit 1: maxWidth */
    uint64_t direction_breakpoint_bits; /* bit 2: directionBreakpoint */
    uint64_t padding_bits;            /* bit 3: padding */
    uint64_t now_bits;                /* bit 4: now */
} PeleRenderNumbersV9;
```

Payload values are host-native uint64_t containing IEEE-754 binary64 bits, not a
portable serialized byte stream. C/Rust/Swift on the supported arm64/x86_64 native
ABIs share this layout. Use named bit constants; assert layout in C integration.

- NULL packet means no numeric overrides. Non-NULL packet is synchronously
  borrowed, never retained, copied into a small internal value before dispatch.
- Caller guarantees readable header and the full declared structure when size is
  valid. Validate header before reading payload; do not form a full Rust reference
  before size validation. Header read/copy must respect C alignment contract (or
  use unaligned reads). FFI cannot validate arbitrary dangling pointers.
- Version must equal 9; struct_size must equal 56. Unknown sizes/versions fail.
  Do not accept arbitrary trailing data under this version; a later version can
  specify an extension. This bounds packet reads to constant space.
- Reject present & ~0x1f != 0. Unselected payload slots are ignored, need not be
  zero, and do not create defaults. Any bits in a selected slot are accepted.
- Use f64::from_bits and Double.bitPattern. Preserve signed zero and NaN payload
  during transport; do not promise payload identity after floating arithmetic.
- There is no packet null/undefined tag. Selected fields always mean Number.
  Unselected JSON null/absent retain legacy Option behavior. Runtime JS now:null
  coerces to zero, unlike legacy JSON null; null is not in declared number|Date
  input and remains explicitly outside this slice.

Errors are semantic transport errors, with stable messages distinguishing bad
version, bad size and unknown numeric mask. No packet error invokes callbacks,
parses source or advances a public document revision.

## 3. New exports and ownership

```c
PeleResult *pele_render_with_numbers_v9(
    const uint8_t *source, size_t source_len,
    const uint8_t *options, size_t options_len,
    const PeleRenderNumbersV9 *numbers,
    uint32_t output_limit_kind, uint64_t output_limit_bits,
    const PeleHostResourcesV3 *resources);

PeleResult *pele_render_document_with_numbers_v9(
    const PeleResult *document,
    const uint8_t *options, size_t options_len,
    const PeleRenderNumbersV9 *numbers,
    uint32_t output_limit_kind, uint64_t output_limit_bits,
    const PeleHostResourcesV3 *resources);

PeleResult *pele_public_render_operation_with_numbers_v9(
    PelePublicDocument *document, uint64_t expected_revision,
    const uint8_t *options, size_t options_len,
    const PeleRenderNumbersV9 *numbers,
    uint32_t output_limit_kind, uint64_t output_limit_bits,
    const PeleHostResourcesV3 *resources);
```

Output limit tags: 0 = OutputLimit::Undefined, 1 = Null, 2 = Number(from_bits),
3 = None/use legacy options.outputLimit; all other tags reject. Bits are ignored
for 0/1/3. Tag 3 is not conversion of usize into f64 and must not use exact hooks.
Tags 0/1/2 override only JSON outputLimit, using existing exact semantics.

All inputs/resources/callback contexts are borrowed only until return. Result is
one owned PeleResult, freed by existing pele_result_free. Immutable document is
borrowed, never consumed; mutable handle uses its existing try-lock/revision
protocol. No new ownership handle or out pointer. No concurrent free. New calls stay
inside v9-only bounded_call panic containment.

Immutable results use bounded version-2 success/error envelopes and existing
render_result_transport, preserving width/height IEEE sidecars. Mutable results
use existing binary graph/revision/JSON-metadata exchange; reuse bounded Output
writer, not serde_json::to_vec/string for an unbounded intermediate metadata blob.
Share helpers with public.rs without duplicating serialization formats.

All three v9 exports also stream top-level errors through the bounded writer.
An error that exceeds 64 MiB after JSON escaping becomes a small native limit
envelope; do not build its complete JSON string first or truncate source fields.
Under-budget errors retain the full canonical envelope. Panic containment covers
dispatch and error serialization. This v9 boundary leaves old exports unchanged
and does not promise a universal peak-allocation ceiling.

Pre-dispatch order for all v9 entry points:

1. Check declared options length <=1 MiB and, for source calls, source length
   <=16 MiB, before borrowing their byte buffers. Options length check wins when
   both lengths exceed budget. These are transport guards, outside Pele semantics.
2. Validate/copy optional packet; validate output-limit tag.
3. Borrow/decode options UTF-8 and JSON through the shared filtered decoder below.
4. Source call: borrow/validate source UTF-8. Immutable call: validate document
   provenance. Mutable call: acquire existing document try-lock, then stale
   revision check through the existing operation. Keep these checks before host
   callbacks and module work.
5. Dispatch once, then existing output/theme/error/state/result-publication flow.

Do not change old-export malformed-input precedence. The v9 order above is its
explicit native boundary contract. Source parse errors precede rendering even
when a supplied numeric option would be nonsensical for that diagram.

## 4. One shared masked decoder

Private FFI seam:

```rust
fn render_number_options(
    json: &str,
    numbers: ValidatedRenderNumbers,
    output_limit: Option<OutputLimit>,
) -> Result<RenderOptions, PeleError>;
```

Mask 0 plus None must delegate directly to existing render_options. Otherwise
filter only selected top-level names and, when Some, outputLimit. Consume those
values as IgnoredAny, then deserialize every remaining key using existing
RenderOptions deserialization; finally assign selected f64 fields.

Use a streaming top-level MapAccess filter/DeserializeSeed around existing serde
JSON decoding, or an equivalently bounded decoder. Do not round-trip through
serde_json::Value/Object: that can collapse duplicate keys and normalize numbers.
No 32-way family/struct decoder generation. Preserve existing acceptance/error
behavior for all unmasked fields, parse fields and unknown keys. Reject duplicate
known overridden keys as well; skipping their values does not authorize ambiguous
transport. Call deserializer.end() to reject trailing input.

Examples: masking padding permits JSON padding:"ignored" and applies packet
padding. It must not permit fontSize:"bad", negative JSON limit, invalid
nativeTheme, or invalid JSON syntax. Output tag 3 must still reject an invalid
JSON outputLimit. Tags 0/1/2 may ignore its value but no other limit field. A mask
for padding must not strip config.padding or any nested property.

Empty/whitespace JSON remains defaults. Current byte/depth/parser resource guards
remain. A masked value still must be syntactically valid JSON; no raw byte surgery.

## 5. Minimal Rust bounded orchestration

No required public fields, RenderResources policy, new module trait hooks, or
usize sentinel. Add these methods using the existing option types:

```rust
Engine::render_source_bounded(
    &self, source: &str, options: &RenderOptions,
    output_limit: Option<OutputLimit>, resources: &mut dyn RenderResources,
) -> Result<RenderResult, PeleError>;
Engine::render_document_bounded(
    &self, document: &ParsedDocument, options: &RenderOptions,
    output_limit: Option<OutputLimit>, resources: &mut dyn RenderResources,
) -> Result<RenderResult, PeleError>;
PublicDocument::render_operation_bounded(
    &mut self, expected_revision: u64, options: &RenderOptions,
    output_limit: Option<OutputLimit>, host: &mut dyn RenderResources,
) -> Result<RenderOutcome, PeleError>;
```

Internally extend shared render_registered_policy/private operation plumbing with
an independent enforce_native_bytes boolean. Existing legacy methods pass false;
existing exact-output methods and new bounded methods pass true. Some/None still
select exactly the existing exact/legacy module hooks. Custom Rust providers use
the same retained registration; no new provider requirement.

For new bounded calls, check actual SVG bytes <=64 MiB immediately after module
render and again after native theme resolution. Preserve existing error types and
diagram ID. Perform semantic output-limit comparison in its existing selected
lane, then link-ID decoding. Module/host errors already returned win before any
output comparison. Class legacy early semantic rejection remains for None;
Class exact native-byte preflight remains for Some. Do not convert the legacy
integer threshold to an exact-number threshold.

This bounds accepted SVG and transport output, not every renderer's peak memory.
Keep existing graph/work budgets. Bounded methods are additive; old API output
policy is unchanged. Public operations still advance/publish revision and graph
on dispatched operation errors; pre-dispatch errors do not. Snapshot/export
failure can leave native state advanced, as documented today.

`render_source_bounded` rejects sources above 16 MiB before preparation or semantic
source-limit comparison; existing source-render methods retain their previous
contract.

## 6. Clock normalization

Change only RenderSession clock seeding:

```rust
clock: settings.now.filter(|value| value.is_finite()).map(Ok)
```

Keep raw options.now unchanged. Gantt already accepts a finite explicit value or
asks resources.now_millis; public getTasks performs the equivalent numeric
fallback. The session caches the first fallback result/error lazily. Pie/Flowchart
and other consumers that never request a clock must not trigger a clock callback.
Do not read system time in Swift, during packet decoding, or eagerly for all types.

Pinned clock(now) returns finite numbers unchanged, including fractional millis
and -0. Omitted/nonfinite/invalid-Date input falls back to Date.now. With a finite
host clock this occurs once per Gantt render. Host errors propagate before the
schedule begins; later text/icon errors retain prior scheduling mutations.

Do not generalize the once-per-render claim to a hostile Date.now replacement
returning NaN/Infinity. Pinned getTasks may request fallback again in that case;
existing native RenderSession caches host results. Matching arbitrary nonfinite
host-clock behavior is outside this selected explicit-option slice. Native host
clock tests use finite values or throw, and record that precondition.

## 7. Thin Swift adapter

New internal RenderNumbers.swift contains an encoding value/helper returning
(optionsJSON: Data, packet: PeleRenderNumbersV9, outputKind: UInt32, outputBits:
UInt64). It copies Engine.Options, sets one packet bit for each non-nil of the five
Double fields, copies bitPattern, then sets ONLY those copied properties to nil
before JSONEncoder. Values need not be nonfinite to travel in packet. This keeps
signed-zero/fraction behavior uniform and avoids deciding routes by numeric value.

With explicit OutputLimit, remove ONLY the copied options.outputLimit and pass its
existing nativeValue. Without it, leave that JSON field intact and use tag 3.
Do not change the caller's Options, global JSONEncoder strategy or ConfigValue.
Invalid unmasked JSON-encoded data, including nonfinite Config values, still fails.
Keep Options Codable behavior unchanged outside render calls.

Existing source/document Engine.render signatures delegate through one private
implementation to v9. Add overloads with an explicit outputLimit argument:

```swift
render(_ source: String, outputLimit: Pele.OutputLimit,
       options: Options = .init(), resources: Pele.HostResources = .init()) throws -> FullRenderedDiagram
render(_ document: Pele.Document, outputLimit: Pele.OutputLimit,
       options: Options = .init(), resources: Pele.HostResources = .init()) throws -> FullRenderedDiagram
```

MutableDocument.renderOperation/render keep existing signatures and explicit
OutputLimit overloads, using the same helper and new mutable v9 endpoint. Preserve
withCapturedNativeResources, decode exchange, update revision, then rethrow the
original Swift callback Error object. Never replace it with a C-generated error
or throw before decoding the returned state. Immutable methods keep existing
withNativeResources ownership/error identity and withExtendedLifetime(document).

Borrow the packet with withUnsafePointer around the entire synchronous call. Keep
Data buffers and resources bridge alive through that call. Keep old packaged
binary compatibility exclusions; a legacy package must not compile source using
v9 symbols unless its CPele artifact provides them.

Optional small Date convenience, if included: Engine.Options.setNow(date: Date)
assigns JS-TimeClip(date.timeIntervalSince1970 * 1000) to now. Finite values with
abs(ms)<=8.64e15 truncate toward zero and normalize zero to +0; out-of-range or
nonfinite values assign NaN so Gantt uses host time. Do not apply TimeClip to raw
Double now. Date object identity is irrelevant to clock's numeric coercion.
This helper is optional; exact numeric transport does not depend on it.

## 8. Integration acceptance and bounded risky inputs

Oracle extractor: pinned npm Pele with recorded bundle/source hashes. Owned Node
worker per bounded fixture, small source, timeout/output cap, deterministic text
measurer and finite clock; terminate timed-out workers. No builds, timing claims,
or unbounded source/geometry stress in fixture generation. Whole SVG plus IEEE
width/height, callback traces, errors, and selected full Gantt graph/history prove
public behavior. Preserve normalization only where existing corpus defines it.

Representative fixture matrix:

- Each of five fields: omitted, ordinary finite, -0, +0, positive/negative
  fraction, NaN, +Infinity, -Infinity. Exercise selected-mask override of a
  conflicting finite legacy value. Record upstream explicit undefined and null
  separately; now:null is excluded boundary, other nullish defaults match legacy.
- Pie (tiny one/two-slice source): fontSize and padding; finite constant measurer
  logs request sizes. Flowchart (tiny chain at a real direction threshold):
  maxWidth and directionBreakpoint, with autoDirection true and false controls.
  Non-Gantt now values must produce no clock calls.
- Gantt (short explicit dates plus one relative/unscheduled task): each now value,
  finite fractional host fallback, repeated render, invalid Date and clipped-Date
  equivalents. Compare private now and scheduled Date/alias state. Finite explicit
  now calls clock zero times; absent/nonfinite input with finite host calls once.
- Error ordering: throwing clock before scheduling; throwing measurer after
  scheduling; exact outputLimit 0/NaN/+Infinity/negative threshold after callbacks;
  legacy Class precheck with tiny integer limit retained under tag 3. Mutable
  graph/revision survive dispatched host/output errors, and next operation uses
  published revision. Swift checks original callback Error identity.
- C protocol: NULL packet; exact version/size; bad version/short/oversize header;
  unknown mask; unused payload garbage; all five bits together; masked invalid
  JSON field accepted vs identical unmasked field rejected; nested/other invalid
  options unchanged; duplicate/trailing/malformed JSON; bad output tag; source and
  options byte guards before buffer reads; null document/stale revision; repeated
  result ownership and no callback/revision on pre-dispatch rejection.
- Immutable source and parsed-document results must equal mutable results for
  non-stateful fixtures; Gantt source/immutable/mutable must match their existing
  ownership distinction without losing output/clock semantics. Repeat ordinary
  finite cases through old C exports to prove unchanged dispatch/ABI.
- Native output budget: reuse bounded-output regression strategy, cover tag 3 and
  an exact tag, pre-theme and post-theme paths, plus bounded result metadata.
  A trusted oversized icon/provider fixture may exercise this native boundary;
  no 1M-node or giant-font stress is necessary for proving the byte guard.

Risk policy: never silently clamp/reject the newly representable IEEE values to
make tests pass. Start with tiny diagrams and fixed finite measurer widths so
transport/clock behavior is isolated. Run genuinely extreme finite magnitudes in
separate timeout-controlled workers only if needed; a timeout is evidence of an
unresolved renderer work bound, not a passing compatibility case. Fix a confirmed
unbounded loop with an actual work guard; do not impose an invented global finite
number restriction. Report any unsupported boundary rather than claiming universal
numeric-domain parity from this representative transport slice.

Validation once per coherent slice: focused Rust integration, C consumer tests,
Swift tests through rebuilt native artifact, arm64 and x86_64 ABI/package build,
existing relevant output-limit/host-error/Gantt/Registry regressions, diff check.
No unit tests or unrelated full-suite loops.

## 9. File ownership

Core owner: compat/engine.rs and compat/public/engine.rs (bounded methods/private
flag, finite clock seed), focused core integration. No module.rs public fields or
new DiagramModule hooks required.

C/Swift owner: new crates/pele-ffi/src/render_numbers.rs, lib.rs module/export,
public.rs helper reuse/mutable endpoint, include/pele.h; new Swift internal
RenderNumbers.swift, Engine.swift, MutableDocument.swift; package-generation
legacy exclusions only if needed; C and Swift integration tests. Reuse
calendar::with_resources and current output writer; avoid another host bridge.

Oracle owner: extractor/fixture under isolated temporary directory until root
approves adoption. Root integrates one coherent slice and performs one consolidated
build/verification after source freeze. No performance candidate adoption here.

## All-family numeric acceptance checkpoint (2026-10-19)

The corrected pinned oracle contains 1,023 cases: 33 families, baseline and each
of five numeric settings set to NaN, positive/negative infinity, negative zero,
0.5 and 13.25. The generator runs an owned, bounded worker per case. All cases
return SVG in the pinned library; no native timeout or callback budget is counted
as an oracle error. The gzip fixture SHA-256 is
`732d7a45b958563712e12134ce283db8ed354b5c158a16aacf231b89e2be2ac8`.

`public_all_render_numbers` passed all 3,069 renders through source, retained
immutable document and public mutable document APIs. It compares complete SVG,
dimension bits, ordered text-measurement callbacks and complete canonical mutable
graphs before/after rendering. Independent BFS canonicalization retains genuine
nested ClassMember prototypes; method source/minifier spelling is excluded.
NaN arithmetic results compare as NaN; input payloads retain separate strict
bit-pattern transport checks. Signed zero and infinities compare exactly.

The fixes preserve JavaScript Math.min/Math.max/Math.round semantics at the actual
math call sites. Conditional extrema and ternary clamps keep their different
semantics. Positive fit-loop predicates also preserve NaN termination, preventing
the infinite-font Treemap loop. Timeline preserves measurements of all period
labels before all events and the original NaN fit branch.

The four original `public_render_numbers` feature tests also passed, including
exact/legacy dispatch, lazy clock behavior, source byte guards, original callback
errors, retained state and independent native SVG output budgets. These are Rust
proofs. New C v9, actual Swift SDK and browser acceptance remain separate gates.

The rebuilt ARM64 C v9 archive passes all five strict C integration programs
(numeric transport, registry, exact parse/source/output limits). The copied Swift
SDK compiles and passes all four RenderNumberTests, including all 3,069 all-family
renders and the original 861 numeric renders, Date/raw ownership, exact output and
original throwing callbacks. Registry and limit suites also pass. The independent
x86_64/Rosetta and fresh external-consumer gates are still pending at this point.
