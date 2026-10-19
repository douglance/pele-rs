# Public models, mutable documents, and lossless host transport

Status: implementation LLD; no additional parity claim. Target: Pele 1.6.0,
commit `458322a5b8abb1ae57ddf127c32017ab9fb14a92`. Inspected Rust baseline:
`52c928b`, plus the in-progress State/Gantt/Sequence and calendar adapters.

This document supersedes the frozen browser model requirement in
`FULL_PARITY_LLD.md` section 9 and its section 6 permission to omit methods and
private caches from **public-model parity**. Existing normalized JSON fixtures
remain useful syntax/render evidence, but do not establish this gate. The
immutable Rust document API and existing v1/v2/v3 exports remain supported.

## 1. Contract and evidence

The required result is a usable port of the library's public models: callers can
inspect and edit their fields, use their collections and methods, and render the
same parsed model with those edits. Parsing, scheduling, graph construction,
layout, geometry, and SVG generation remain Rust. JavaScript owns JavaScript
objects, callbacks, and DOM integration; it does not reimplement diagram logic.

Distinguish three evidence sets instead of conflating them:

1. **Declared API:** `src/models.ts`, each exported model interface, `Diagram<M>`,
   `ParseOptions`, `RenderOptions`, and public entry functions. `GanttModel.getTasks`
   is explicitly declared. Most parser/database builder methods are not declared
   on the public `DiagramModel` union.
2. **Concrete returned objects:** public `parse()` returns actual database
   instances for many families. Their prototype methods, ordinary writable own
   properties, TS-private enumerable fields, and shared references are observable.
   Preserve these deliberately as runtime compatibility; do not call them declared
   API or silently strip them from the oracle.
3. **JavaScript metaprogramming:** arbitrary proxies, getters that mutate unrelated
   fields during reads, changed built-in intrinsics, and exact function source or
   engine-generated stack traces are not evidence that all 33 Rust renderers need
   a JavaScript interpreter. Track a concrete documented/use-case/upstream-test
   obligation before adding a special read-timing path. This LLD does not claim
   universal ECMAScript observational indistinguishability.

Evidence from pinned source and public Pele 1.6.0 probes:

| Source | Contract that affects implementation |
| --- | --- |
| `core.ts::parse`, `parsedFrom`, `render` | `WeakMap` keys exact root object identity. Clones and fabricated roots fail `render` with `TypeError`. Changing `model.type` does not change the retained diagram type. Parse options passed to `render(model)` are ignored. |
| `core.ts::register`, `parseSource` | Parsed metadata retains the exact `Diagram` object and effective config. Replacing the registry entry does not retarget old models. Editing that retained object's `render` or `section` is visible to old models. This differs from immutable Rust descriptor snapshots. |
| `types.ts`, `models.ts` | Maps, Date-valued Gantt fields, and `getTasks(now?: number \| Date)` are real public types. Host numeric options are `number`, not nonnegative integers. |
| `diagrams/gantt/{db,render}.ts` | Render calls `db.getTasks(options.now)`. Scheduling mutates tasks and caches, returns the same tasks array, and may fail after partial mutation. Tasks retain identity; new scheduling creates new Dates. A successor's `startTime` can be the predecessor's exact `endTime` object. |
| `diagrams/state/db.ts` | `direction` is a prototype getter over `rootDoc`, not an enumerable own field. `setRootDoc` replaces/extracts derived state explicitly; direct source-document edits do not implicitly run that method. |
| `diagrams/sankey/db.ts` | `links[i].source`, `nodes[j]`, and `lookup.get(id)` can reference the same node. |
| `diagrams/mindmap/db.ts` | Flat `nodes`, nested `children`, and private `open` share nodes. `getType` is an enumerable own function shared with Kanban's imported helper. |
| `diagrams/class/db.ts` | `withCssClass` is `Map<string, Set<ClassNode>>`; namespace maps reuse class/note objects. Arrays of copies are not equivalent. |
| `diagrams/timeline/db.ts` | `getCommonDb()` returns `this`. A copied result is wrong even when all fields match. |
| `diagrams/swimlane/index.ts` | The returned model has `type: 'flowchart'` and FlowDb's prototype; retained render result type is `swimlane`. |
| `tests/stress.test.ts`, `tests/gantt/stress.test.ts` | Public calls use `limit: Infinity`, `maxEdges: Infinity`, and Date-valued `now`. JSON-only options already exclude exercised behavior. |

Published bundles minify constructor names (`FlowDb` was `tc` in the inspected
installed main entry). Do not invent a public constructor-name promise from a
TypeScript identifier. Record the package artifact/hash and entry used by the
oracle separately from the source SHA. Prototype identity, descriptors, callable
behavior, and shared-function identity are useful obligations; minifier names
and function source text are not a reason to duplicate upstream JavaScript.

## 2. Smallest architectural addition

Keep `DiagramModule`, `ParsedDocument::model<T>()`, and
`Engine::render_document(&ParsedDocument, ...)` unchanged. They remain the owned,
immutable, strongly typed Rust convenience API. In particular, immutable Gantt
rendering may schedule a private working copy as today. Name that behavior in its
documentation; it does not implement JavaScript mutable-model semantics.

Add an opt-in **public document** interface alongside it. A public document owns:

- exact module registration and effective parse config;
- a family-specific mutable state, including observable bookkeeping;
- a flat object-identity arena and the correspondence between typed records and
  exported objects;
- revision/change information used by foreign adapters.

The identity arena is a data exchange facility, not a universal diagram model.
Family parsing/rendering continues to use concrete Rust types. A simple Pie
codec can update its existing typed fields directly. Sankey and Gantt need typed
record IDs for shared nodes/tasks/Dates. Avoid forcing simple value-only structs
into `serde_json::Value` or an interpreter to accommodate those families.

An implementable opt-in seam, with concrete names finalized in the first slice:

```rust
pub trait PublicDiagramModule: DiagramModule {
    type PublicState: Send + 'static;

    fn model_schema(&self) -> &'static ModelSchema;
    fn parse_public(&self, context: JsParseContext<'_>)
        -> Result<Self::PublicState, PublicError>;
    fn export_public(&self, state: &Self::PublicState, out: &mut GraphWriter)
        -> Result<(), PublicError>;
    fn apply_public(&self, state: &mut Self::PublicState, edits: &ValidatedEdits)
        -> Result<(), PublicError>;
    fn invoke_public(&self, state: &mut Self::PublicState, call: MethodCall,
        host: &mut dyn PublicHost) -> OperationOutcome;
    fn render_public(&self, state: &mut Self::PublicState, config: &PublicConfig,
        options: &PublicRenderOptions, host: &mut dyn PublicHost)
        -> OperationOutcome;
}
```

`OperationOutcome` contains either a return value/render result or an error,
**and** the changes made before completion/failure. A host exception is an opaque
exception token, not a stringified `PeleError`. A synchronous host bridge returns
that same exception object to JavaScript. The upstream `guarded` conversion of a
`RangeError` to the nesting-limit `PeleError` remains at the public call boundary.

`Engine::register_public` stores this opt-in implementation beside the existing
erased module. Ordinary `register` remains valid; a module without a public codec
can still parse/render natively and must advertise that its public-model bridge
is unavailable. Do not silently fall back to a misleading mutable JSON object.
Custom Rust modules may provide their own schemas and method IDs; no closed enum
of all 33 model types is required at the registration boundary.

Schema describes only real exchange needs: stable schema ID/version, root and
record kinds, ordered fields, optional/undefined rules, collection kinds,
prototype/member descriptors, and typed method IDs. Family `public_model.rs`
owns the codec. It is not an inferred schema from whatever `Serialize` happens to
emit, nor a second renderer. Small generated descriptor tables are reasonable;
runtime source-code generation or a generic grammar/JavaScript VM is not.

## 3. Lossless flat graph

Introduce graph schema 1 independently of ABI and render-result version:

```rust
enum Atom {
    Undefined, Null, Bool(bool), Number(f64),
    String(StringId), Ref(ObjectId), BuiltinFunction(FunctionId),
    BuiltinReference(SharedObjectId),
    HostReference(HostReferenceId),
}
enum Node {
    Object { prototype: PrototypeId, properties: Vec<Property> },
    Array { length: u32, properties: Vec<Property> },
    Map { entries: Vec<(Atom, Atom)> },
    Set { values: Vec<Atom> },
    Date { milliseconds: f64 },
}
struct Graph {
    root: Atom,
    strings: Vec<JsString>,
    nodes: Vec<Node>,
}
```

The actual node records also allow own properties on collections/Dates where
the selected runtime-compatibility scope needs them. `Property` has a string
key, a data value, and its enumerable/writable/configurable flags. Absent is no
property; present `undefined` is `Atom::Undefined`; array holes are absent indexed
properties, separate from explicit undefined and length. Default prototype
definitions live once in the schema. State's getter and builtin method members
are schema descriptors, not values fabricated as own fields.

Rules:

- Object IDs describe identity, not a user-visible node's `id` field. Equal IDs
  in a chart do not imply equal object identity. IDs are document/session scoped,
  checked, and never recycled while a foreign reference can still name them.
  Module-level shared constants such as ClassDb's `lineType`/`relationType`
  objects use runtime-scoped shared IDs, like shared builtin functions. Do not
  clone them per parse or freeze them when upstream does not. Ordinary objects
  assigned across two model roots keep their JS identity; each document imports
  their latest contents before its own next operation.
- Use iterative worklists for export/import/comparison/free. A node is emitted
  once; references may point forward, backward, or to itself. Cycles do not force
  recursive serialization or reference-counting leaks. A cyclic transport graph
  does not imply every renderer accepts a cyclic logical hierarchy.
- Map/Set order follows JavaScript insertion order and SameValueZero behavior.
  Object property enumeration follows integer-index keys then string insertion
  order. Delete/reinsert is an observable order change. Do not sort Map keys.
- Dates are objects with identity and IEEE-754 milliseconds, including invalid
  Date's NaN. Never serialize them as ISO strings or collapse two equal Dates.
- Intern strings by code-unit value within exchange sessions. Packet can repeat
  one very long label in thousands of rows without expanding the label bytes
  thousands of times. String identity itself is not observable in JavaScript.
- NaN, both infinities, and negative zero remain binary numbers. Equality for
  change detection distinguishes `-0` from `+0`; Map keys still use SameValueZero.
- Host functions/objects are retained opaque references within one host session,
  never executable code in a payload. Exporting a host callback to another process
  requires an explicit callback binding or fails as a transport capability error.
- Decode structural transport rules before applying changes: checked lengths,
  reference validity, tag/version checks, allocation arithmetic, and duplicate-ID
  rules. Treat `__proto__` as a data key using property definition, not assignment
  to an inherited setter. Preserve existing source/config pollution defenses.

Use a length-delimited binary encoding: fixed tags, explicit integer widths,
little-endian IEEE numbers and UTF-16 units, string/node tables, then root and
references. C uses borrowed byte slices for requests and owned buffers for
responses. WASM uses `Uint8Array`/`Uint16Array`. An escaped JSON diagnostic dump is
optional and explicitly lossy unless it uses all tags; JSON is no longer the
authoritative model/options transport.

Graph size is bounded by actual unique nodes, references, and string units, not
JSON nesting depth or repeatedly expanded paths. Replace the fixed 16 MiB/128
depth dependency on this new lane with separate configurable **transport**
budgets. Chunking bounds each crossing; it does not remove total-memory limits.
Never allocate from an unchecked declared count. Report budget exhaustion as a
transport limit, with its configured value, separately from Pele's source/output
limits. Select tested defaults from the pinned stress corpus and measurements;
do not assert that a new arbitrary ceiling proves parity.

## 4. Mutation and synchronization

JavaScript returns ordinary writable objects, genuine Maps/Sets/Dates/arrays,
and stable per-runtime prototypes. No frozen projection and no proxy collection
that breaks `Map.prototype.get.call(model.sections, key)`.

On initial parse, allocate object shells once, then populate references and
properties iteratively. Keep `WeakMap<object, ObjectId>` plus the current
ID-to-object table needed by the live document. Root provenance belongs to a
separate WeakMap; no enumerable marker, user-supplied schema ID, serialized graph,
or matching `type` string grants parse provenance.

Before `render(model)` or a bridged model method:

1. Walk the schema's relevant fields and reachable records iteratively. Compare
   against the last exchanged values. Read Map/Set/Date contents, not just their
   root identity. New ordinary records receive IDs; changed/deleted fields and
   reordered collections produce edits. Preserve aliases across all traversals.
2. Send only changes and new string/object table entries. Unchanged render must
   not serialize and deserialize the full model. Retain the concrete Rust model
   and apply edits to it; regenerate only invalidated typed projections where
   the family's current representation requires one.
3. Execute the Rust family operation. Return/apply changes to existing JS objects
   in place. Replace a property/array/Date only when upstream replaces it. Return
   preexisting JS objects when Rust returns their IDs.

Unrestricted ordinary JS mutation has no reliable dirty bit: a linear scan of
reachable API state at a call boundary is the honest default cost. Proxy-only
tracking cannot see every alias or borrowed native Map/Date operation. Do not
promise constant-time mutation discovery. Measure scan time, imported bytes,
typed rebuilding, and render time separately. String identity/value caching and
per-node comparisons avoid unnecessary byte copies. Native typed edits and an
optional explicit managed-edit extension can mark changes directly; neither
changes compatibility `parse()` semantics.

Retain unknown application-owned properties in the host object. Do not erase
them during reconciliation or pull their unrelated large subgraphs into Rust.
The runtime schema additionally retains upstream-created enumerable bookkeeping
fields. A family codec must not rebuild a cache simply because its name sounds
private: upstream leaves caches stale after some direct edits. Where a method
uses that cache, its observed state and aliases matter.

Valid mutation means the values allowed by the public model contract, including
ordinary edits to nested collections. Do not re-run the parser or reapply parse
validation to these values. For example, model edits need not be expressible in
source syntax. Wrong runtime kinds should fail at the relevant family operation
with the appropriate error category, not become default values or disappear.
Wider arbitrary-object coercion behavior is recorded separately when exercised;
schema validation must not be advertised as full JavaScript coercion emulation.

Mutation publication is not transactional with rendering. After a renderer or
method throws, expose every change made before that throw. Transport corruption
can reject an entire edit request before execution; that is a different failure.

### Gantt: first stateful proof

Gantt public state owns task IDs and Date IDs. Its `tasks` array and `byId` map
reference the same task records. Port scheduling as `schedule(&mut state, now,
resources)`; both public `getTasks` and public rendering use it. The existing
immutable render convenience invokes the same scheduler on its working copy.

Preserve the exact reset sequence for `startTime`, `endTime`, `renderEndTime`,
`processed`, and `manualEndTime`; preserve updates to `now`, `work`, `includeSet`,
`excludeSet`, `excludedDays`, and `datedTokens`. An unresolved predecessor can
leave a task unscheduled. Do not normalize it to an empty or invalid Date.
Dependency assignments share Date IDs; a fresh scheduling-created Date gets a
new ID even if its milliseconds match an earlier value. Retained old JS Dates
remain unchanged after replacement.

`getTasks(now)` is an actual JS prototype function forwarding to Rust and returning
the existing tasks array. Render's call to it is an explicit dispatch point:
if still the installed builtin, call Rust directly; if caller replaced it, call
the supplied function with the model as receiver, preserve exceptions, import
the returned task graph, and continue with that returned value. Do not bake in
the assumption that it is always `model.tasks`.

For a supplied `getTasks` result, retain its relation to the model graph: an
existing tasks array imports as existing IDs; a new array imports as new IDs.
Rendering that array must not overwrite `model.tasks` unless the callback itself
did so. Method dispatch stays separate from parse provenance.

Flush scheduling mutations before invoking a host measurer/icon/calendar callback
that can inspect the model, and flush on every exit. A callback throwing after
scheduling does not undo the schedule. For changes made *inside* callbacks or
reentrant model operations, first add a concrete public integration reproducer
and define the narrow synchronization points it requires. Snapshotting all
fields before the operation is not evidence of identical arbitrary getter or
callback-mutation read order. Escalate only the affected family path to live
access if such an obligation cannot be met with its actual staged algorithm.

## 5. Methods, prototypes, and per-family codecs

Prototype thunks only perform argument/result conversion and call Rust method
IDs. Do not port `addVertex`, `getActors`, `getTasks`, or state graph rebuilding
into JavaScript. Preserve descriptor placement: methods nonenumerable on the
family prototype, own `getType`/Block helper functions enumerable where upstream
assigns them. TypeScript `readonly` does not freeze a JS property.

Each descriptor manifest records members from the actual returned instance and
its prototype, classifies declared versus incidental methods, and gives a Rust
implementation/proof reference. Preserve shared helper-function identity across
instances when upstream shares it. Borrowed methods and exposed constructors can
be added against concrete runtime-compatibility cases; constructing a database
must never accidentally grant `render` provenance.

Per-family conversion requirements below are a work map, not proof that existing
Rust fields already satisfy them. Read the named model interface in full; do not
use this table as a field allowlist. Record every field/descriptor in the schema.

| Family | Returned shape and required conversion seam |
| --- | --- |
| flowchart | FlowDb; ordered nodes/classes/tooltips; subgraph/edge lookups alias exported records; members Set and pair counts; builder methods and options bookkeeping. |
| swimlane | Same FlowDb schema/prototype as flowchart; retained diagram ID remains swimlane. |
| pie | Plain object; sections Map; own undefined title/accessibility fields; no fabricated database methods. |
| er | ErDb; entities/classes Maps; subgraph lookup aliases; claimed membership; entity/subgraph methods. |
| mindmap | MindmapDb; flat nodes and children/open aliases; own shared getType; iterative deep traversal. |
| kanban | KanbanDb; sections/items and last/misplaced references; own shared getType. |
| timeline | TimelineDb; periods/sections/currentSection; getCommonDb returns root identity. |
| journey | JourneyDb; tasks/sections/currentSection; getActors result and ordering. |
| class | ClassDb; classes/notes/namespaces/styleClasses Maps; namespace shared records; withCssClass Map of Sets; line/relation constants and namespaceStack. |
| quadrantChart | QuadrantDb; classes Map; point records and quadrant/axis fields; setters. |
| xychart | XyChartDb; tagged axes/plots; hasSetXAxis and hasSetYAxis remain distinct; public result includes fields omitted by narrower interface declarations. |
| state | StateDb; rootDoc, states/classes/links Maps, graph and statement aliases; prototype direction getter; translate/extract only at upstream call points. |
| gantt | GanttDb; tasks/byId identity, mutable Date objects, scheduling sets/counters, method outcomes on success/failure. |
| architecture | ArchitectureDb; nodes/groups Maps and registered bookkeeping; edges/layoutHints; builder methods. |
| requirement | RequirementDb; requirement/element/class collections, pending builder fields and styleCopies; copy behavior must match actual sharing. |
| sankey | SankeyDb; nodes/links/lookup share nodes; findOrCreateNode returns existing identity. |
| packet | Plain object; nested rows/segments; intern repeated label strings; no tree-path expansion. |
| radar | Plain object; axes/curves/options; distinguish undefined optional values from omission. |
| treemap | Plain object; recursive roots, classes Map; iterative topology exchange. |
| info | Plain object; exact own title/accessibility field presence. |
| sequence | SeqDb; actors/createdActors/destroyedActors, actor links/properties Maps; actor.box aliases boxes; notes/messages and active/currentBox bookkeeping. |
| gitGraph | Plain object, not a GitDb instance; commits/branches Maps, lanes/head/currentBranch/warnings. |
| c4 | C4Db; parent object references, shape/boundary/index aliases, nested relation index Maps, current/stack; extra Maps on elements. |
| block | BlockDb; root/blocks/byId aliases, classes Map; pending Set and counters; own helper functions as well as prototype methods. |
| railroad | RailroadDb; rules/byName aliases, notation and grammar IR; include reachable recursive/aliased rule data without recursive serialization. |
| agentflow | AgentflowDb; FlowDb-like records plus connectors/mappings/diagnostics and their spans; private maps/sets; semanticModel and emit/builder methods. |
| treeView | Plain result from buildTreeView, not TreeViewDb; root/count and deep children. |
| cynefin | Plain result from buildCynefin, not CynefinDb; domains Map and transitions. |
| wardley | Plain result; all nodes/links/pipelines/annotations/axes/size structures; do not expose its internal builder as the root prototype. |
| eventmodeling | Plain result; boxes/lanes/relations/notes/specifications/entities and their actual shared references. |
| usecase | Plain result; actor/usecase/boundary/class collections, graph statements/source spans and JSON nodes; include accDescription separately from accDescr. |
| venn | VennDb; subsets/textNodes/styles, known Set/current/indent bookkeeping and methods. |
| ishikawa | IshikawaDb; root/stack aliases and baseLevel; addNode. |

New Rust parsers should preserve identity during construction when the upstream
builder creates shared objects. Existing structs that contain equal copies need
a family migration or explicit identity sidecar. Never infer aliasing merely
from equal serialized values. A schema exporter can reference typed arena IDs;
it does not need a second fully materialized generic graph before writing.

## 6. UTF-16 and numeric inputs

Lossless transport alone does not fix UTF-8 parser semantics. Add `JsString` with
a compact well-formed UTF-8 representation plus an exact UTF-16 representation
when needed. Expose code-unit length/index/slice, Unicode scalar iteration where
upstream uses it, and an explicit fallible `as_str`. This choice needs benchmarks;
always allocating two bytes per ASCII character is not a prerequisite.

Add `parse_public`/source-preparation methods accepting this representation.
Preserve the existing `&str` API as its well-formed convenience. Migrate actual
string operations according to upstream semantics: many scanners use
`charCodeAt`/UTF-16 indices; some code uses scalar iteration. Byte indices cannot
be substituted for either. Do not encode lone surrogates as private-use
characters, U+FFFD, or textual entity escapes: each changes recognition or values.

The migration includes model keys/labels, error snippets, source preparation,
text-measurer requests, icon names, URL helpers, SVG strings, and result fields.
The Sequence menu-link split already demonstrates a lone output surrogate from
an otherwise well-formed source. A Unicode-only parser plus a lossless input
buffer does not close that gap. JS output is reconstructed by code units in
bounded chunks; no `TextEncoder`/`TextDecoder` round trip for ill-formed strings.

Keep `PublicParseOptions` and `PublicRenderOptions` separate from today's
`usize`/JSON structs. Number-valued compatibility limits are f64, with presence
and defaulting represented explicitly. Apply upstream comparisons directly:
`source.length > limit` behaves differently for negative, fractional, NaN, and
infinite limits. `now` accepts number or Date; Gantt's `clock` uses Date.now when
numeric conversion is non-finite. Nullish versus undefined defaults differ by
option; specify these from `core.ts` and each consumer, not a global sanitizer.

Independent native work/allocation caps remain checked integers. A user setting
`Infinity` disables the corresponding Pele comparison; it cannot authorize
unchecked allocation. Report separate platform-budget behavior honestly.

Swift `String` cannot preserve isolated surrogates. Supply an exact
`Pele.JSString` (`[UInt16]` or owned packed storage) with `stringIfWellFormed` and
explicit replacement conversion, plus convenience String overloads. A
`Pele.DateValue` preserves invalid Date and identity; Foundation `Date` alone
does not represent every JS Date state. Existing String/Decodable APIs stay
convenient projections, never the only path to full data.

## 7. Foreign ABI, lifetimes, registry, and concurrency

Use new `_v4` entry points for graph/source/options operations; current `_v3`
calendar exports are already in use. Reserve separate versioned schemas rather
than changing `PeleResult` payload meanings. Minimum surface:

```c
PelePublicDocument *pele_public_parse_v4(/* UTF-16 + options graph + error out */);
PeleBuffer *pele_public_snapshot_v4(const PelePublicDocument *, /* cursor/budget */);
PeleBuffer *pele_public_apply_v4(PelePublicDocument *, /* graph edits */);
PeleBuffer *pele_public_invoke_v4(PelePublicDocument *, /* method + args + host */);
PeleBuffer *pele_public_render_v4(PelePublicDocument *, /* options + host */);
void pele_public_document_free_v4(PelePublicDocument *);
```

The implementation header must define every pointer/length, buffer owner,
callback scope, null rule, status, version/size field, and exception token.
`snapshot` is optional inspection; parse success must not depend on constructing
one giant projection. Keep allocation/free paired within the library. Native
panic containment and borrowed callback lifetimes remain mandatory. WASM can
wrap the same owned document with binary methods; avoid raw pointers as JS IDs.

One public document has one serialized mutable operation stream. Immutable Rust
documents remain separately shareable. Swift's new `MutableDocument` is not
`@unchecked Sendable`; confine it to one owner/actor or explicitly synchronize
its edit/render operations. Existing immutable `Document` behavior is unchanged.
Foreign revision checks detect stale edit batches; a conflict is an adapter
error, not silent last-writer replacement.

Release a browser handle with explicit `dispose` plus `FinalizationRegistry`
fallback. Registry values must not strongly retain roots through a global
ID-to-object table: keep tables per WeakMap value and make finalizer holdings
contain only the native handle. Detached child objects/old Dates remain normal
JS objects after replacement or disposal. No operation may reuse a disposed
document handle. Constructor-created or copied roots remain unregistered.

The JavaScript registry owns live `Diagram` references. Parsed-model metadata
retains `{ typeAtParse, diagram, config, builtinDocument? }`. A replacement entry
affects later parses; edits of the same diagram object affect existing models.
Builtin descriptor functions invoke Rust. A user-supplied Diagram's parse/render
callbacks run in JS as explicitly trusted extensions and may own arbitrary models;
they need not be serialized into a builtin schema. Primitive parse results are
returned as upstream does, but only non-null object results acquire WeakMap
provenance. Model rendering cannot recover provenance from a primitive.

Retain nested references from host `config` according to upstream's shallow
top-level merge. Changes to a retained host config section before render must be
imported; copying options by JSON at parse time would break this behavior.
Registered custom handler lifetime and config identity tests belong with this
slice. The Rust registry's immutable descriptor rule remains its own documented
API, not a claim about the JavaScript adapter.

Lazy loading, mount, and zoom consume this same browser registry/model surface.
Their full behavioral LLD remains `FULL_PARITY_LLD.md` section 9; this change does
not implement them or license a second JavaScript renderer. Native extensions
use Rust registration first. Add arbitrary C callback-provider vtables only with
an actual consumer and lifecycle integration proof.

## 8. Normalization and proof

Replace `JSON.stringify` normalization for this gate with an iterative graph
normalizer. Canonical comparison IDs are assigned by deterministic traversal,
independently in oracle and port; production IDs need not match. Compare:

- field presence and enumeration order, collection/Date kind, value and order;
- reference equality/alias topology, including cycles;
- declared versus runtime bookkeeping fields without blanket `private` removal;
- root/prototype member placement and descriptor flags;
- methods through operation scripts and observable return identity/state changes;
- exact UTF-16 strings and non-finite/negative-zero numbers;
- returned/error values and state after failed operations.

Functions normalize to a schema/member identity and callable behavior, not
`Function.prototype.toString`. Existing value-only model fixtures remain a
separate historical normalization revision. Do not rewrite their expected
values to imply broader coverage.

Integration/E2E tests only. Every test starts from public parse/entry/registration
or a public native document operation. Required operation sequences:

1. **Pie edit:** mutate a Map value, add/delete/reinsert a slice, replace the Map,
   edit title, render twice; verify resulting drawing/order and root/Map identity.
   Clone/root fabrication fails; editing `type` does not retarget provenance.
2. **Aliased graph:** Sankey edit through link.source changes the same node seen
   in nodes/lookup; Class namespace/Set references and Mindmap flat/tree references
   agree. Exercise deep and cyclic *transport* via a permitted mutable field;
   compare renderer rejection independently where cycles are invalid.
3. **Methods:** Timeline `getCommonDb() === model`; a builder method changes
   rendering; own getType/helper functions keep expected placement; State getter
   and explicit extract/translate show no invented implicit cache refresh.
4. **Gantt:** getTasks with number/Date/fallback clock, same tasks identity,
   dependency Date sharing, Date replacement on reschedule, edited raw task data,
   direct Date mutation, overridden getTasks, excluded days/DST, and partial state
   after scheduling/measurement/output-limit errors. Compare state before and
   after each operation, not just pre-render model plus SVG.
5. **Limits/strings:** Infinity/NaN/negative/fractional options, -0 transport,
   absent/undefined/null, invalid Date; lone input surrogates and Sequence's
   surrogate-producing split; depth above 128 and Packet repeated-label cases
   without 16 MiB JSON expansion.
6. **Registry/resources:** replacement versus editing retained Diagram,
   retained nested config changes, old model after engine disposal, exact callback
   thrown identity, cleanup/recovery, disposed/stale handles, and source rendering
   agreement with unedited model rendering.
7. **Swift/C/WASM:** same graph script and numbers/UTF-16 values; independent
   document ownership, repeated creation/free, bounded invalid payload handling,
   Swift mutable model edit/render, and unchanged v1 fixture responses.

Inventory public-model cases separately from parser/render corpus counts. Record
declared-API pass/fail, runtime-object compatibility, and unimplemented exotic
behavior honestly. No family's public-model gate is complete while declared
mutations or methods are silently ignored.

## 9. Atomic implementation order

1. **Pie end to end:** add graph schema/codec and new document ownership only as
   needed for Pie; implement ordinary Map/scalar edits, absence/undefined,
   number/UTF-16 transport, provenance, and changed-state import. Compare existing
   render corpus plus the Pie operation script in Rust/C/WASM. Measure unchanged
   and single-edit cost. Commit this bounded slice before generalizing.
2. **Sankey identity:** add typed node-reference support and prove alias-preserving
   import/export. This exercises real identity before adding all complex families.
3. **Gantt mutable methods:** extract shared mutable scheduler, Date identity,
   returned-object identity and failure changes, dynamic getTasks, calendar and
   callback publication. Add Swift mutable-document proof here. Keep immutable
   rendering's existing behavior explicit.
4. **Lossless string semantics:** complete UTF-16 preparation and one vertical
   parser/model/render path, using Sequence's concrete split case. Migrate each
   remaining family's actual string operations as its model codec is adopted;
   transport-only success cannot close its string gate.
5. **Runtime model families:** adopt one family at a time using the table above.
   Reuse proven descriptors, identity tables, and collection primitives. Migrate
   private caches only where that family's returned object actually exposes them;
   do not expose internal builders of plain-object families.
6. **Live JS registry and host entries:** preserve Diagram/config object retention,
   per-family exports, lazy registration, then mount/zoom lifecycle integration.
   Keep their progress independent from model codec completion.

Do not implement a universal live-property bridge in advance. If a supported
callback/method scenario proves incremental import's timing wrong, minimize the
case and add a narrow family access seam. Conversely, do not advertise the
snapshot/delta design as universal exotic-object parity. This boundary preserves
the pure Rust engine and provides concrete next commits without hiding ordinary
public mutations behind immutable cached renders.
