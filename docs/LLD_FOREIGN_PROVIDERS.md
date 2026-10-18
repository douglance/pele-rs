# Swift-authored providers and retained host callables

Design target: Pele 1.6, upstream `458322a5b8abb1ae57ddf127c32017ab9fb14a92`.
Inspected and first provider slice executed 2026-10-19. Core/C v10 is implemented;
an isolated public Swift facade passes real ARM64 consumers. Root Swift SDK
integration and x86_64 verification remain release gates. Shared mutable Config,
lossless UTF-16 source/options and Gantt host callables remain separate completion
gates. Borrowed text, icon, clock, date and calendar resources now share the same
active Rust render session through additive C helpers.

The implementation record below supersedes the earlier proposed unified graph /
foreign document storage. v10 deliberately adds a separate provider-only handle;
existing built-in graph documents and all older C exports keep their contracts.

## Observable contract and current gap

Upstream `src/types.ts` declares `Diagram<Model>` with mutable `type`, `section`,
`keepComments`, `parse`, and `render` properties. `src/core.ts:21–31,59–89,111–136`
stores each Diagram object in an ordered registry and retains that exact object,
model, and effective config for a parsed model. Consequently:

- Re-registering a type changes future parses; existing models keep their old
  provider. Editing the old provider object's render function still affects its
  old models. `section` is read for each render; `keepComments` for each parse.
- The registration key is the type at registration. Later editing `diagram.type`
  does not move that entry or change an already parsed model's reported type.
- Source limits precede preprocessing, detection, lookup, then provider parsing.
  Parsing receives entity-encoded source with a trailing newline, effective
  config, optional title, removed-line offset, and number-valued maxEdges.
- Rendering a model ignores parse options. Rust must still resolve responsive
  defaults, provide decoded icon names, enforce output policy after the callback,
  and decode returned link IDs. Provider exceptions escape without replacement.
- The detector recognizes the fixed DiagramType union. This slice permits
  replacing those canonical IDs; arbitrary new language detection is not implied.

Upstream `tests/entries.test.ts` proves selective registration. Replacement and
live-property cases need focused oracle fixtures against the source contract;
do not describe them as existing upstream test coverage.

Rust already has `DiagramModule` and `PublicDiagramModule`, with retained Arc
registrations and concrete models. Their `Send + Sync`/`Send` bounds are useful
native contracts. C Registry v8 only registers built-ins; Swift cannot supply a
parser or renderer. `Atom::HostReference` transports identity but does not dispatch
it. Gantt explicitly rejects that atom at `getTasks` in
`crates/pele/src/compat/diagrams/gantt/public_model.rs`.

## First proof: one real provider, before a generic framework

Use an empty Registry and a Swift-authored replacement for canonical `pie`.
The test model is an ordinary reference class, with mutable width and renderCount.
Its parser records the supplied prepared source/config/title/offset/maxEdges and
returns that same class instance. Its renderer increments renderCount, optionally
calls the supplied measurer, and emits a small complete SVG rectangle. This is
user extension code, not a Swift implementation of the built-in Pie algorithm.
Keep all built-in parsing, layout, and drawing in Rust.

The integration consumer must:

1. Register provider A; parse document A; retrieve its typed Swift model; change
   its width; render and observe changed SVG/dimensions and the same object identity.
2. Register provider B under `pie`; parse B; prove A still calls A and B calls B.
   Change A.render and A.section afterward; A must observe both changes.
3. Close the Registry; render both documents; close them in opposite order.
   Weak-reference/deinit counters prove exact provider/model retention and release.
4. Throw a class-valued Swift error after changing renderCount. The caller catches
   the identical error object; the model retains its mutation and can render again.
5. Exercise prepared source, comments, responsive defaults, numeric policies,
   icon decoding, and output rejection through the normal engine policy path.

Implement this first with internal Swift trampolines and the proposed C packet.
Do not start with reflection, dynamic schema generation, a JavaScript interpreter,
or graph serialization of arbitrary Swift objects. A successful prototype is a
prerequisite for publishing the generic Swift convenience below.

## Ownership: a local foreign lane, shared Rust orchestration

Do not implement DiagramModule for an opaque Swift pointer using unsafe Send/Sync.
Do not make Swift Diagram, Registry, MutableDocument, or its model Sendable.
Provider calls execute synchronously on their caller's logical owner. An actor may
change OS threads between calls; a ThreadId check is not an actor-isolation check.
C callers must serialize access and destruction, as existing opaque handles require.

Keep foreign owners in the FFI crate, with no Rust Send/Sync implementation.
Extend private Registry state with an ordered effective registration selection:
either Builtin or `Rc<ForeignRegistration>`. Reuse PublicEngine for built-in
execution. A foreign entry shadows the built-in entry of the same ID;
registerBuiltin replaces that selection again. registered/supports/capabilities
must read this effective selection. Do not leave those methods consulting only
the underlying built-in engine after a foreign registration succeeds.

Keep the existing PelePublicDocument unchanged. Add a distinct opaque PeleProviderDocument:

```rust
struct ForeignDocument {
    provider: Rc<ForeignRegistration>,
    model: *mut c_void, // one callback-owned retain, released before provider
    config: Object,
    revision: u64,
}
struct PeleProviderDocument {
    document: Mutex<ForeignDocument>,
}
```

The registration retains the parse-time canonical type. v10 provider parsing and
rendering use this handle. Graph snapshot/apply/invoke APIs remain on the existing
built-in document. An older parse API encountering a shadowing foreign registration
reports that the v10 provider lane is required; it never silently uses the built-in.
The provider handle exposes model identity and small status metadata without
attempting graph reflection. This is an additive first slice, not full unified
source/model routing.

A document owns exactly one model retain and one retained registration reference.
The registration owns one provider retain. Drop the model before the document's
registration reference, so model release may safely use provider context. Parse
failure or failed result publication releases an unpublished returned model.
Registration replacement releases the old registry reference only after the new
selection and its success result are ready. Existing documents remain valid.

Keep try_lock/busy rejection for same-owner concurrent/reentrant operations;
never block waiting for a callback that is waiting on the same owner. Swift must
reject close while active. The C free contract still forbids freeing an active
handle. Callback reentry into a different idle document is allowed. Reentry into
the active Registry/document returns the explicit busy error before mutation.

Extract only two shared, typed core seams from existing engine code:

- An owned PreparedProviderCall built from the same inspection/limit logic as
  Engine.parse_with_limits. It exposes detected type for lookup, then prepares
  the selected provider's comment-preserving source, config, title, line offset,
  and exact EdgeLimit. Do not preprocess twice or create a fake typed model.
- A bounded provider render function accepting descriptor/config/options,
  resources, exact output policy, and one FnOnce callback returning Rendered.
  Delegate to render_registered_policy. Preserve typed PublicError/host exceptions
  with the existing operation-error side channel or a private generic helper;
  never expose its internal stop marker to the caller.

The FFI layer owns lookup and foreign lifetime; Rust owns preparation and render
policy. No second parser, theme resolver, output-limit implementation, or renderer
belongs in the adapter. Existing typed Rust Engine consumers keep their contracts.

## Implemented additive C contract

`include/pele.h` is the normative packet layout and function declaration. v10 uses
exact version/struct_size validation, zero reserved fields, copied vtables and
owned byte leases. The implemented callbacks are describe, parse and render;
parse/render requests use fixed-layout structs with borrowed pointer/length spans.
`PeleForeignReplyV10` carries status, exception token, opaque model, independent
owned payload/links leases, and raw width/height bits.

The exported owner operations are:

```c
pele_registry_register_provider_v10(registry, canonical_id, id_len, table);
pele_registry_parse_provider_v10(registry, source, source_len, options, options_len,
    source_kind, source_bits, edge_kind, edge_bits, invocation_context, out_document);
pele_provider_model_v10(document, out_model);
pele_provider_status_v10(document);
pele_provider_render_v10(document, expected_revision, options, options_len,
    numbers, output_kind, output_bits, resources, invocation_context);
pele_provider_document_free_v10(document);
pele_foreign_call_context_v10(active_call);
pele_foreign_text_width_v10(active_call, text, text_len, size_bits, style, out_bits);
```

These abbreviated calls document ownership and routing; exact parameter types and
packet fields are in the header. `PeleProviderDocument` is provider-only.
Built-ins continue to use v8 parse / v9 render exports. Borrowed text measurement
uses the active Rust render session and font settings. Icon/calendar/clock entry
points are not part of this first provider resource surface. C callers must not
retain active-call pointers or recursively enter a resource operation on the same
active call; the Swift context guards these conditions.

The actual current numeric packet typedef must be reused, without duplicating its
layout under a new name. The provider-render result includes revision plus rendered
value or PublicError, with IEEE dimension bits exactly as existing v9 results.
ProviderDocument.render dispatches here. Built-in MutableDocument and its
graph-returning renderOperation keep their existing API; a foreign graph codec is
not implied.

The new parse entry handles foreign selections and reuses the existing ParseLimits
decoder; a built-in selection reports the required built-in parse lane. There is
no second source-limit, preprocessing or output policy implementation. The
invocation_context is borrowed, never stored in a document. Swift supplies one
operation bridge with an exception table, kept alive until result decoding ends.
Trampolines access it through the active call. No thread-local/global “last error”
or provider-global error slot is needed. Older C parse exports stay callable for built-ins. A direct v10 caller without
an invocation bridge may receive structured errors but cannot issue host-exception
tokens. Reject an unbound token explicitly.
Initialize out_document/out_model before any fallible work. Successful parse
metadata includes type, decimal revision, and modelKind (builtin/foreign), so Swift
selects its private backend without probing model pointers. status returns the
same small metadata and never exports the model. New operation envelopes carry
PublicError, including a host token, through a dedicated Swift decoder; the old
PeleError-only compatibility decoder must not stringify or discard that token.

Register validates canonical ID, packet version/size, callbacks, reserved fields,
and budgets before retaining. For v10 require the exact known size; a later larger
version needs an explicit decoder. Required callbacks are never optional. Once the
engine takes its context retain, RAII releases it on every failure. Swift supplies
an unretained borrowed box during register; C takes its own retain through retain.

Before every callback zero-initialize out, including model/payload/token. Reject
inconsistent replies: success plus exception, failure plus model, unknown status,
nonzero size with null data, or an owned payload without its release callback.
Null/zero/no-owner is a valid absent payload where that operation permits it
(successful parse needs the owned model, not an additional result payload).
Release any owned
reply parts even when invalid. Null model is not a successful class model. Never
unwind across extern C; Swift trampolines catch and record, Rust contains panics.

`describe(phase=0)` reads current keepComments before parse; `phase=1` reads current
section before render. Its small payload also reports no unrelated fields.
The vtable is stable trampolines; those trampolines read the retained Swift Diagram's
current closures. This provides live provider edits without mutable C function-table
memory. A changed provider.type is considered only by a later register call.

The first slice transports finite Config snapshots and passive options as bounded
JSON. Five render-number fields use the existing raw IEEE v9 packet; source/edge/
output policies use their existing tagged raw-bit contracts. Nonfinite Config is
rejected explicitly instead of converted to null. Shared mutable Config and a
lossless Config arena are not implemented. Prepared source is a bounded UTF-8 span
in the parse request; title has explicit presence. Numeric atoms preserve raw IEEE bits, and maxEdges preserves
Null versus Number after applying the undefined-only default. Render options retain
presence, raw five numeric fields, raw output policy, and resolved responsive.
Do not semantically re-decode or apply parse-only fields on a model render. Unlike
the built-in render-only seam, a custom Diagram receives RenderOptions and can
inspect limit/maxEdges/config. Preserve supplied fields as passive values in its
callback options; decode render settings separately without an integer ParseOptions
decoder rejecting those fields. Swift's existing built-in-only render copy may
strip them, but the foreign branch must preserve them. Native explicit threshold
arguments define policy: output tag0 omits outputLimit, tag1 supplies Null, tag2
supplies its IEEE Number, and tag3 preserves the legacy field's supplied value or
absence. The engine's effective default is not inserted as an own callback property.
ProviderRenderContext exposes this distinction with OutputLimit plus presence-aware
option access, not a Double default. The parse callback's limits.maxEdges instead
contains the effective value: Undefined becomes 5000; Null and Number stay raw.
Resource callbacks are
capabilities on the active call, not serialized closures.

The callback-data lease survives until Rust finishes validating/copying that reply,
then release runs exactly once. No pointer remains valid merely until the next
callback: nested calls must not overwrite another live reply's buffer. Owned model
state is never encoded just to cross the boundary. Input/output text still follows
the existing well-formed Swift String lane; this proposal does not fix lone UTF-16.

Foreign SVG is provider-trusted markup, matching custom Diagram.render. The same
64 MiB native result budget applies to SVG, metadata and errors; output comparison
uses UTF-16 units after callback execution. Check returned spans before copying,
and SVG bytes before/after theme resolution. No claim bounds arbitrary provider
allocation, execution time, or malicious C pointers. A synchronous callback cannot
be force-cancelled safely. Existing source/options/graph/work budgets remain active.

## Tested Swift facade (isolated until matching SDK integration)

```swift
extension Pele {
    public final class Diagram<Model: AnyObject> {
        public var type: String
        public var section: String?
        public var keepComments: Bool
        public var parse: (ProviderParseContext) throws -> Model
        public var render: (Model, ProviderRenderContext) throws -> ProviderRendered
        // Explicit initializer; no @Sendable requirement or generated schema.
    }
    // ProviderRendered: SVG String, Double dimensions, and DiagramLink array.
    // Context values: prepared source/config/title/lineOffset/maxEdges, or
    // retained config/options plus borrowed active resource capabilities.
}
extension Pele.Registry {
    public func register<Model>(_ provider: Pele.Diagram<Model>) throws
}
extension Pele.Registry {
    public func parseProvider<Model: AnyObject>(_ source: String, as: Model.Type,
        limits: Pele.ParseLimits = .init(), options: Pele.Engine.ParseOptions = .init())
        throws -> Pele.ProviderDocument<Model>
}
// ProviderDocument<Model>: throwing `model` getter, type, revision, render,
// exact OutputLimit overload, idempotent guarded close. Not Sendable.
```

Supply public value initializers for ProviderRendered and link records so a
downstream package can actually construct a complete callback result. Test the
external package, not only @testable imports that hide initializer-access gaps.

Registry.parseMutable remains the built-in graph API. parseProvider selects the
effective Swift provider and returns a typed ProviderDocument. The model accessor
conditionally casts the retained object; a type mismatch during parse releases
the unpublished document and its owned model before throwing. This is the only required model
reflection. Return the same object, not a decoded copy; user references may outlive
document.close, while subsequent rendering through the closed document fails.
The C model pointer names a Swift model box retaining AnyObject, not an unchecked
cast directly to a generic Model. Its release trampoline destroys that box once;
providerModel unwraps and conditionally casts its retained object.

Context is valid only inside its callback. Copied config/options are ordinary
values; resource methods reject use after callback completion. The call's generation
and active flag live in a Swift lease object, preventing a dangling C context pointer
from being used by a captured closure. Use callback-scoped stacks for nested calls.

The initial built-in AsyncEngine remains independently actor-owned. Do not pass an
existing non-Sendable provider/model into it or add unchecked conformance. A later
actor-isolated provider factory can construct providers inside that actor if needed.
Likewise, Mount integration must explicitly receive/use the intended Registry;
merely adding register to Registry does not reroute Engine or Mount's built-in engine.
These are integration follow-ups, not reasons to duplicate provider machinery.

## Exceptions, partial mutation, and publication

One active foreign call owns an exception table. A Swift trampoline catches the
original Error and returns its token; the wrapper publishes revision/result state
before throwing that same Error. An error message is not an identity substitute.
No token remains in a document after the owning operation completes.

Resource calls made *by* a Swift provider need a callable borrowed bridge back to
the already active RenderSession. Expose text width, icon/direct icon, clock and
calendar operations there, preserving existing caching/decoding. If the provider
catches a resource error and returns a valid SVG, the render succeeds. Do not use
an unconditional “some callback failed” flag which later overrides that success.
Only the final uncaught token in the operation's reply is rethrown. A different
exception thrown after catching the first one must retain its own identity.
The custom provider's public icons capability delegates to direct_icon, preserving
repeated resolver calls; label-internal icon caching is a separate existing path.

### Implemented borrowed resource API

`ProviderRenderContext` exposes `textWidth(_:size:style:)`, `icon(_:)`,
`nowMillis()`, `parseDate(_:)`, `calendarFields(_:)` and `calendarInstant(_:)`.
`icon` returns `TrustedSVG?`, date/instant return `Double?`, and calendar fields
return `CalendarFields?`. Names and date strings use the existing UTF-8 boundary.

Five additive `pele_foreign_*_v10` exports extend the existing text-width helper;
no public provider struct layout changes. Numeric outputs use raw IEEE bits plus
an optional presence flag, initialized before failures. Icon and field results own
`{value: ...}` envelopes so missing values remain distinct from absent results.
Every result must be freed. Icon names/date strings are limited to 1 MiB, civil
JSON to 4096 bytes, returned icon markup to the existing 8 MiB host limit, and all
result envelopes to the existing bounded result limit.

All six capabilities use a shared Swift active-call guard and a Rust `RefCell`
borrow of the active session. Cross-resource reentry fails immediately, including
C callers which bypass Swift. Capturing a Swift context is safe: resource methods
reject it after callback completion. Raw C call pointers remain synchronous borrows
and must never be retained or used after the callback.

Icon lookup uses `direct_icon`: decode names on each invocation and do not add a
provider-level cache. Clock uses the session's explicit finite `now`, otherwise its
cached host clock result. Swift retains an original clock error for repeated calls
within that same context because Rust also caches clock failure; other resource
errors may be caught and retried independently. Every capability consumes its own
original Swift error, so a handled error cannot poison a later successful render.
Date parsing/instants reuse existing TimeClip truncation and range checks; calendar
fields reuse native input clipping and field validation. The portable defaults and
HostResources timezone conversion remain shared with built-in diagrams.


For a foreign document, begin an operation only after transport/revision/busy checks.
After provider dispatch begins, advance revision on success or exception, including
post-render output-limit failure. Provider mutations are already in the authoritative
Swift object; do not roll them back. Before-dispatch failures leave revision unchanged.
If result encoding fails afterward, state may have advanced: include a documented
revision/status query that does not require graph export, so recovery is possible.
Do not make a fake PublicSnapshot necessary to inspect that revision.

## Shared next primitive: Gantt.getTasks and host function references

Pinned `gantt/render.ts:70–90` resolves the clock, reads displayMode/topAxis, and
calls `model.getTasks(now)` once. The returned array, rather than model.tasks by
assumption, drives layout. `db.ts:214` normally mutates scheduling fields/cache and
returns model.tasks. This is a declared method; replacing it with a callable is a
concrete missing behavior. It does not require universal JavaScript metaprogramming.

Reuse the provider call lease, owned host context, reply bytes, exception-token
table, and original Swift Error recovery. Do not route it through a Swift Gantt
renderer or export/import the whole model at every render.

Add an optional, default-rejecting model-call capability to the per-operation host
boundary. A suitable narrow signature is:

```rust
fn call_host_function(
    &mut self,
    reference: u32,
    graph: &mut ModelGraph,
    receiver: Atom,
    arguments: &[Atom],
) -> Result<Atom, PublicError>;
```

Place this capability on an additive operation-host adapter/default hook; existing
RenderResources implementations keep compiling. It is a host execution capability,
not a place for source/output policy. Thread it through RenderSession without
altering text/icon/clock behavior. No callback means no new graph copies or lookup
work on ordinary built-in renders.

In Gantt, finish any Rust read borrows before dispatch; pass the actual root receiver
and resolved numeric now; validate the returned atom against the same arena. Read
fields after the call when pinned render reads them after the call. Do not reread
displayMode/topAxis that upstream already captured before calling getTasks. Preserve
the callback's partial graph mutations on throw and native downstream errors.

The C callback receives a borrowed **active model view**, not the public document
handle. Give that view only the existing arena operations needed by the prototype:
read/write own fields, array elements/length, create ordinary record/array/Date,
intern UTF-16 strings, and call a captured built-in method with explicit receiver.
Typed Swift helpers wrap those primitives. Node/Atom IDs refer to the current arena;
neither IDs nor raw view pointers may be used after the call lease expires.
Work ticks and graph limits apply to every operation. No eval, JS property VM,
getter interpreter, or automatic Mirror-based conversion is required.

The first Gantt override calls the captured original getTasks built-in, mutates one
returned task, and returns the same array. A second returns an alternate task array;
a third edits then throws. Reentering public document.invoke would hit its busy guard,
so the active view must dispatch a captured native method directly, under the same
borrow/revision. Reject recursive host dispatch past a small documented call-depth
ceiling; do not accidentally redispatch the override when “original” was requested.

Bind host-reference IDs to retained callables on the document's owner. Raw IDs are
not pointers and are never global. Reject unbound/cross-owner IDs explicitly. The
initial implementation may retain bindings until document.close, even after removal
from the graph, with a bounded registration count; do not silently reuse an ID while
an escaped graph can still reference it. Existing builtinFunction IDs retain their
current schema and identity rules. Callable registration identity and graph references
must survive successful snapshots/applies without inventing executable serialized code.

This closes the named method-override slice only after its acceptance passes. Host
mutation from a later measurer callback, arbitrary replaced nested methods, callable
constructor behavior, and shared mutable Config still require separate source-backed
audits. A single host-call seam is reusable; it is not evidence that all such cases work.

## Integration acceptance and file boundaries

Implement in this order, with one consolidated focused verification per layer:

1. **Provider prototype:** new FFI providers module, minimal shared core helpers,
   private Registry/document dispatch, internal Swift trampolines, public-consumer
   integration executable. Implemented and ARM64 tested: C feature scenario plus
   two direct-C Swift feature tests. x86_64 linked execution remains pending.
2. **Public provider API:** isolated Diagram/context/value types and typed document
   pass five public Swift feature tests. They prove original Error identity, caught
   resource-error recovery, replacement/live properties, reference-model identity,
   exact numeric options, mismatch cleanup, close/reentry and release-callback
   guards. Source exports and old-binary exclusions are integrated. The current
   ARM64 release SDK also passes all 13 provider/resource/registry integrations,
   including icons, clock, date and calendar resources. Final universal packaging
   and release publication remain open.
3. **One callable prototype:** Gantt getTasks via active model view; original-method
   delegation, aliases/Date bits, alternate result, partial mutation/error identity,
   lifetime/recovery. Only then publish generic host-call binding conveniences.

Provider acceptance matrix: preprocessing/title/comments/entity newline; merged
config precedence; exact negative/NaN/infinite source/edge/output policies; source
error before callback; callback before output rejection; responsive section edits;
provider/type/render closure edits; A/B registration retention; direct model edits;
clock/icon/measurer calls and caught resource failure; missing provider; malformed
reply; invalid packet; busy reentry; close in callback; nested other-document call;
parse allocation followed by failure; metadata/output budget failure; registry free
before document; model reference outliving document; no double-release on failure.

Use pinned Node oracle fixtures for JavaScript-observable source/model/error ordering.
Use C/Swift integration assertions for native lifetime/busy/budget contracts. Do not
write tests of private refcount mechanics alone; observable deinit and public reuse
belong in the complete feature scenario. No unit tests or unbounded million-object
stress fixture is necessary to validate this interface.

Likely implementation ownership:

- `crates/pele/src/compat/engine.rs`: share preparation/render orchestration only.
- New `crates/pele-ffi/src/providers.rs`; private dispatch changes in registry.rs and
  public.rs; `include/pele.h`: C ownership and versioned exports.
- New Swift `Diagram.swift`; Registry.swift internal owner access/destruction guard;
  HostResources.swift operation-local error accessor; Engine.swift public link
  initializer; provider feature integration tests. MutableDocument stays unchanged.
- Follow-up callable slice: public host hook, Gantt public_model.rs, active-view C
  helpers and Swift callback bindings. No other family rewrite in this first slice.

The existing Config value model cannot preserve arbitrary shared object identity or
mutations across parse/render callbacks. Config callback views in this first provider
prototype are values, and that limitation must be explicit. Later Config ownership
must reuse a retained value arena rather than bolting JSON copies onto each provider.
Likewise Rust String/Swift String source and SVG paths do not preserve lone UTF-16.
Neither gap is solved by opaque model ownership. This document makes no full-native
or full-public-API parity claim.
