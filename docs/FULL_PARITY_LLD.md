# Full Pele parity: architecture and execution contract

Status: required implementation plan, not a compatibility claim. Target is every behavior of Pele commit `458322a5b8abb1ae57ddf127c32017ab9fb14a92` (package version 1.6.0), not whatever a later Mermaid release adds. Baseline inspected is pele-rs `6803707`, whose released v0.1 interface implements a restricted flowchart subset. The complete obligation inventory is [PELE_PARITY.md](PELE_PARITY.md).

This document supersedes the first-slice scope exclusions in `LLD.md` for the new full interface. It does not retroactively change the released v1 ABI or its documented output. Every previously deferred Pele feature is now required. Implementation remains standalone Rust; no embedded JavaScript engine, invocation of upstream at runtime, or JavaScript implementation hidden behind WASM is a port. Upstream JavaScript is permitted only as a development/test oracle. Thin browser DOM adapters and user-provided extension callbacks are legitimate host code, not diagram implementations.

## 1. What full parity means

There are four independently earned gates per inventory row:

1. **Syntax:** same type detection, accepted/rejected source, lexing precedence, preprocessing, parse errors and source locations. A recognizer that accepts source then ignores a construct fails this gate's accompanying semantic gate.
2. **Semantics:** same ordered typed model, defaults, identifiers, titles, metadata, relationships, styles, links, diagnostics and config effects. Preserve absence versus null versus empty values where observable. Accepting a diagram type and rendering a subset is incomplete.
3. **Rendering:** same layout decisions, text treatment, visual elements, shape geometry, ordering, clipping, links, accessibility, responsive behavior and safety for the same resolved inputs. Primitive and DOM comparisons are primary; screenshots are supplementary. This requires real upstream algorithm translations, not the v0.1 layout reused for every family.
4. **Host:** working native/C/Swift and browser WASM calls; model reuse, resources and callbacks; browser mount/zoom behavior and native equivalents where the browser owns the behavior. Host-independent results must agree across adapters.

No row becomes complete from a small representative fixture set. Completion requires the entire pinned associated corpus, applicable render/security/stress cases and feature inventory, plus generated/mutated coverage. A row may be partially implemented while implementation progresses; all pending rows remain required. An intentional security or platform difference is recorded explicitly and cannot silently count as strict upstream parity. Fix upstream-observable quirks only in a separately named extension mode or with an explicitly documented compatibility exception; target Pele's behavior, including its differences from Mermaid.

## 2. Verified current differences

Current `crates/pele/src/lib.rs` exports `parse -> Flowchart`, `layout`, `render_svg`, and source-only `render`; no general detector, registry, other family, preprocess/config, typed general model or model-render entry exists. `contract.rs` exposes only version-1 render envelopes. C owns immutable result handles; Swift and web wrappers decode that flowchart envelope. Preserve these working consumers.

Current flowchart parser requires direction, restricts IDs, supports five node shapes and five edge operators, and rejects subgraphs, directives/frontmatter, styling, links, metadata and markdown. It does not implement the upstream lexer/database/graph model. Upstream flows include expansion, additional arrow/length/animation forms, rich shape catalogue, nested and collapsed groups, class/style inheritance, labels, links/icons/images and graph normalization. The v0.1 custom layered router is not equivalent to upstream's `layered.ts` + `compound.ts`; lane layout is absent.

Current text is embedded Noto Sans outlined glyphs, with unsupported-glyph/RTL rejection. Pele uses a `TextMeasurer`, Arial fallback metrics, browser canvas fonts, rich label spans and host text rendering. Current concrete five-color themes omit the full token/font/weight/series system and source styles. Native outlined export remains useful but does not establish browser theme/font parity. Current errors and scalar columns differ from upstream `PeleError` categories and UTF-16-based columns. Current hard limits (262144 bytes, 256 nodes, 1024 edges, 8 MiB SVG) differ from Pele's configurable 50000 UTF-16 source units, 5000 expanded edges, and 4000000 UTF-16 output units. The existing 22-fixture corpus and release validation prove only their documented subset.

## 3. Deep module and external seam

Keep one deep Rust engine with two real seams: diagram modules vary by family, and host resources vary by platform. Avoid exposing lexer tokens, rank passes or SVG-writing helpers as user interface. Core orchestration owns limits, preprocessing, registry selection, parsed-model provenance, render context, output validation and errors.

```rust
pub enum DiagramType { /* 33 pinned built-in discriminator names */ }
pub enum DiagramId { Builtin(DiagramType), Extension(String) }
pub trait DiagramModule: Send + Sync + 'static {
    type Model: Serialize + Send + Sync + 'static;
    fn descriptor(&self) -> ModuleDescriptor;
    fn parse(&self, context: ParseContext<'_>) -> Result<Self::Model, PeleError>;
    fn render(&self, model: &Self::Model, config: &Object,
              settings: &RenderSettings, resources: &mut dyn RenderResources)
        -> Result<Rendered, PeleError>;
}
pub struct ParsedDocument { /* private registration, concrete model, effective config */ }
pub struct Engine { /* ordered immutable registration snapshots */ }
```

Concrete implemented interfaces live in `compat/module.rs`, `compat/engine.rs` and
`compat/id.rs`. `Engine::empty` starts with no handlers. `register<D: DiagramModule>`
accepts a module with its own concrete model; type erasure stays private, and
`ParsedDocument::model<T>` exposes a read-only typed view. Built-in family models
remain complete concrete Rust types, not generic JSON graphs. A public closed
33-variant model sum is not required at this seam; it would block new custom kinds.

`DiagramId` separates pinned detector names from open extension IDs. New languages
register under `DiagramId::Extension` and use `parse_as(id, source, options)`; this
explicit seam avoids editing detector ordering or changing the wire discriminator.
Ordinary `parse`/`detect_type` retain pinned built-in precedence. A convenience full
entry will register every completed built-in; it is not present while none qualify.

`ModuleDescriptor` owns its ID, optional config section and `keep_comments` flag.
The engine reads it once at registration and keeps that immutable snapshot with the
handler. Re-registering an ID affects future parses; an existing document retains
its exact registration, model and effective config, even if the provider's mutable
state would now return different metadata. A trusted provider can still mutate its
own behavior; extensions are executable Rust, not sandboxed source.

The public call's `RenderOptions` combines `ParseOptions` and `RenderSettings`.
The module receives only render settings and the document's retained config, so
parse-stage overrides cannot accidentally become a second config authority.
`render_source` parses then renders; `render_document` skips parsing. Arbitrary
JSON cannot fabricate a document. Ordered model projection uses the compatibility
value tree and direct serialization, preserving nested JS enumeration order.

A per-render engine resource session decodes authored icon names, caches positive
and negative lookups, and captures one lazy clock value (or explicit `now`). Those
semantics do not have to be reimplemented in each diagram. The host supplies text
measurement and trusted icon markup; source cannot promote itself to trusted
markup. Registry extensions are trusted code and own their SVG output. Built-in
renderers must still produce the validated scene/result described below; the
retained scene and complete native/browser resource adapters are pending layers.

Capabilities distinguish pinned `known` types, installed `registered` handlers and
earned parity levels. Extension IDs appear as registered but outside the pinned
known catalogue. Replacement must never inherit certification from a prior handler.
The empty core advertises zero complete built-in handlers. At completion the full
entry must register all 33. `supports` checks detected registration and catches
preprocessing errors, rather than claiming source grammar validation.

## 4. Module map and translation discipline

Keep the three-crate workspace. Add `crates/pele/src/compat/` for the new interface while leaving legacy symbols forwarding to their current implementation until a separately tested migration is chosen:

```text
compat/mod.rs, engine.rs, module.rs, id.rs, value.rs, error.rs
compat/source/{mod.rs,preprocess.rs,yaml.rs,entities.rs,detect.rs,locations.rs}
compat/diagrams/<upstream-directory>/{mod.rs,lexer.rs,parser.rs,model.rs,...}
compat/layout/{layered.rs,compound.rs,lanes.rs}
compat/text/{label.rs,metrics.rs,resources.rs}
compat/svg/{builder.rs,root.rs,theme.rs,shapes.rs,edges.rs,cluster.rs,title.rs}
compat/common/{fit.rs,fit_width.rs,fit_text.rs,outline.rs,scan.rs,...}
compat/scene.rs
crates/pele-ffi/src/compat.rs
crates/pele-wasm/src/compat.rs
bindings/web/{core.js,lazy.js,mount.js,zoom.js,resources.js}
bindings/swift/Sources/Pele/{Engine.swift,Document.swift,HostResources.swift}
fixtures/pele/<family>/...; fixtures/pele/manifest.json
scripts/parity/{extract.mjs,oracle.mjs,compare.mjs,report.mjs}
```

These are responsibilities, not a demand for empty files. Translate a real upstream module when its consuming feature slice needs it. Maintain source path, upstream SHA and upstream test obligation references in each translated module's header/provenance map. Preserve token order, state transitions, post-parse model-building passes, ordering, and special cases. Do not replace a parser with a permissive line splitter. Translate upstream's bounded algorithms and optimized scans rather than porting potentially catastrophic equivalent regular expressions. No TS-to-Rust interpreter, generalized grammar VM or dynamic JSON graph framework is required.

Families with lexer/parser/db architecture keep that split only where it provides locality. Families with direct AST/model builders keep their concrete model-building step. Railroad has four grammars and an IR; swimlane deliberately shares the complete flowchart parser/model and replaces preparation/layout/drawing. Agentflow has diagnostic and semantic normalization stages that are part of the model, not optional rendering decoration. Config section names (`xyChart`, `wardley-beta`) and `keepComments` (agentflow/eventmodeling) must be explicit descriptors.

## 5. Preprocessing, config and errors

Translate `preprocess.ts`, `util/yaml.ts`, `detect.ts`, entities and locations before generalizing family parsers. Preserve CR/CRLF normalization, quoted HTML-attribute normalization, frontmatter matching/indent rules, title/displayMode extraction, directive scanning, invalid-init JSON being ignored, wrap directives, comment removal, `withComments`, line offsets, entity placeholders and color-semicolon handling. Maintain an original-source mapping separately from the processed text so native diagnostics may offer byte spans without changing compatibility line/column fields.

Config is a custom ordered `ConfigValue` tree (null, bool, JS-like number, string, list, object) matching the subset accepted by upstream's actual YAML parser, not whatever a convenient YAML crate accepts. Translate this custom reader directly rather than substituting a general YAML crate. That includes its handling of scalars, objects, arrays, comments, quoting, multiline content, numbers, nesting and unsupported YAML features. Its `.nan`/`.inf`/`-.inf` and overflowing numeric values require internal non-finite number variants; `serde_json::Value` alone cannot preserve them. Compatibility model transport follows JavaScript JSON projection where requested, while a lossless tagged-number representation stays available to native model inspection. Preserve JavaScript own-property order (integer-index keys first in numeric order, then insertion-ordered string keys), not merely sorted maps or arbitrary insertion order. Deep-merge document configuration/directives exactly where upstream does so; host extra config and preprocessed config use the same shallow top-level precedence as `parseSource`, with document sections winning. Reject/ignore prototype-pollution keys consistently. Do not apply `deny_unknown_fields` to general upstream config; unknown config is retained or ignored as upstream does. Keep strict validation for Rust-specific transport fields.

Expose compatibility errors with upstream `code` (`unsupported-diagram`, `syntax`, `semantic`, `limit`), `type`, `line`, `column`, `snippet`, and `message`. Compatibility columns/source-size/output-size count UTF-16 code units, not Rust bytes or scalar count. Preserve JS numeric/date/string coercion only where upstream behavior observes it; encode that in small named helpers with public fixture coverage. Additional native byte-span and transport/internal diagnostics belong to a namespaced field. No input source/snippet is logged automatically; diagnostic contents may be returned to the caller, as upstream does. Legacy errors remain unchanged.

## 6. Models, scene and schema evolution

Do not force chart series, sequence activations, grammar alternatives, calendar tasks or nested regions into the existing `nodes/edges` geometry. Each family gets its complete typed model, serializable in a canonical tagged representation. Canonical model normalization for comparison preserves ordered maps as arrays of key/value entries and distinguishes omitted optional fields from explicit null. Exclude only executable parser/database methods and private caches, with an allowlisted explanation for each exclusion. Model diagnostics remain included.

Rendering yields a retained `Scene` of drawing primitives and semantic elements: groups, paths, shapes, styled text spans, clip paths, image/icon assets, links, accessibility and animation descriptors. Preserve order and every value used to emit output. Geometry v2 is a tagged family-independent semantic scene, with stable semantic IDs/roles and optional family-specific layout projection, rather than a fake graph:

```json
{"version":2,"type":"sequence","bounds":{"width":420,"height":300},
 "elements":[{"id":"actor:A","role":"actor","bounds":{},"sourceIds":["A"]}],
 "primitives":[],"family":{"type":"sequence","actors":[],"messages":[]}}
```

Concrete fields of each family projection must be specified with the family implementation. Do not publish an unbounded `any` geometry blob as the permanent interface. Scene primitives remain a focused tagged enum; arbitrary SVG strings occur only at the explicit trusted icon/extension seam. Full render result includes `type`, `svg`, `width`, `height`, `links`, and versioned geometry. `LinkInfo` matches upstream `{id, href, internal}` with decoded authored IDs.

Use a new full-contract envelope (`version:2`) and new Rust types. Keep v1 `pele_render`, `Rendered`, `Geometry`, Swift `Pele.render` and web `loadPele().render` compatible, including their old default behavior, unless shipped as an explicitly breaking major release. New Swift `Pele.Engine` and web full/core entry points consume v2. Do not quietly return geometry v2 to old Swift, which rejects non-v1 geometry. Legacy output may later be derived from full scenes only for losslessly representable flowcharts; unsupported projection must error, never discard groups or shapes. ABI version and JSON/model/geometry schema versions are separate concepts.

## 7. Layout and rendering parity

Port upstream shared `layered`, `compound`, and `lanes` with their actual constants, ordering, caps, cycle reversal, virtual label nodes, crossing reduction, Brandes–Köpf positioning, compaction, edge tracks/ports, nested clusters and fitting logic. The current router is retained solely for legacy v1. Specialized architecture grid, block grid, C4 layout/routing, Sankey flow layout, treemap packing, Venn geometry and family-local render layouts are separate implementations matching their upstream files. Keep malformed/degenerate/large-value behavior and source order stable. Port `common/fit*` and per-family narrow-layout alternatives; scaling an SVG down is not equivalent to upstream's reflow.

Port the entire shape catalogue, aliases, syntax-only/internal shapes, dimensions, label placement, side spans and intersections. Additions need a complete enlarged outline/shape gallery, not just switch-arm existence. All arrowheads, relation ends, loops, parallel edges, group ports, line patterns, animation settings, text styles and source classes must survive the model-to-scene path. Rendering parity includes zero/empty diagrams where upstream renders them; the legacy `empty_graph` rejection is not universal.

Number formatting must follow upstream `svg/builder.ts::num` and field-specific rounding in compatibility mode; do not apply v1's blanket 0.001 quantization before algorithms. Differences in Rust versus JS sorting, truncation, modulo, Infinity/NaN propagation and floating-point formatting need explicit tests. Use deterministic iteration order. Freeze `now` in oracle runs; host-supplied time is captured once for default Gantt calls. Time/calendar semantics follow pinned `gantt/dates.ts`, including local-date behavior, exclusions, durations and formatting; do not assume UTC changes are harmless.

## 8. Text, theme and host-resource seam

Full parity requires two output adapters over the same model/layout/scene:

- **Browser-compatible SVG:** preserve upstream CSS token names/fallbacks, classes, data attributes, text/tspan semantics, font families/weights, CSS-driven radius, accessibility and responsive root behavior. Theme changes that upstream handles without rerender must behave likewise.
- **Resolved native SVG/raster:** resolve those tokens and styles to supplied concrete theme values; shape text using matching supplied fonts, convert text to outlines where needed; resolve images/icons under host policy. This is a supported native representation, not evidence of byte-identical upstream SVG. Return semantic model, links, accessible text and hit geometry alongside pixels.

The resource interface is justified by real browser and native adapters. It supplies text widths for `{text,size,style,font,monoFont}`, icon resolution, resolved theme/font inputs, image resources where requested, and current time. Core never performs network/file fetch or reads clocks. Rust takes a per-render context; do not port upstream's temporary global icon resolver as shared mutable global state. Scope icon caches to one render and preserve upstream's lookup/missing-icon behavior.

Implement upstream fallback Arial metrics as a compatibility measurer; use that exact measurer in both upstream/Rust deterministic oracle runs. Browser Canvas measurer must match family/font loading/cache behavior; JS may supply synchronous callbacks via wasm-bindgen. Native CoreText/resource callbacks supply host widths, with caller-provided font bytes for outline export if exact shared glyph output is desired. A separate bundled deterministic Noto mode remains available. Unsupported Noto glyphs/RTL are not reasons for the full text-SVG interface to reject source that Pele accepts. Do not promise bidi/shaping improvements Pele does not itself guarantee; preserve host text rendering in compatible SVG mode.

Port rich-label handling exactly: entity decoding, `<br>`/escaped/real newlines, accepted inline HTML style markers, tag removal, markdown emphasis and dedent, mono spans where used, line heights, whitespace trimming, word wrapping, long-word fallbacks in family modules, font-awesome/icon placeholders, and missing icon whitespace rules. Unknown HTML is handled as upstream handles it, not executed. Host-measured width changes must trigger appropriate mount invalidation.

Host icon markup is an explicitly trusted seam upstream inserts into SVG. Make trust visible in Rust/Swift/browser types and document it; diagram source cannot promote itself to a trusted icon. A hardened sanitized-icons option can exist, but cannot silently alter compatibility expectations. User source styles use upstream allowlists and unsafe-value filters. Links/images apply upstream entity decoding, sanitizer rounds, scheme allowlists, `links`/`images` flags and `linkRel`; blocked assets must disappear exactly where upstream omits them. Native rendering returns approved resource requests and accepts host-resolved bytes or uses a host loader; it must never fetch automatically. Test approved remote-image rendering with local controlled fixtures, and rejected schemes with zero host fetches.

## 9. Browser and native host contracts

Browser adapters expose equivalent `register`, `registered`, `detectType`, `supports`, `parse`, `render`, `forgetTextWidths`, `load`, `renderAsync`, `mount`, `mountAsync`, and `enableZoom`, plus matching option/result/error shapes. Initialization of WASM is explicit; after initialization sync APIs remain synchronous. `load` deduplicates concurrent requests, returns false for unknown/null types, retries after failed loads, and changes registry state only after successful module availability. Actual WASM code splitting is a packaging optimization; a monolithic Rust module with lazily registered built-ins may meet behavioral loading parity if documented honestly, but does not claim equivalent bundle-size behavior.

Browser parsed models use a frozen read-only JS projection associated with an owned WASM document handle in a WeakMap; fabricated objects are rejected. Explicit disposal and FinalizationRegistry fallback release handles. Existing parsed documents keep their renderer/config; changing render-only width/font options does not reparse. Module registration adapters must preserve existing parsed documents after replacement. No built-in parser, graph builder or layout moves into JavaScript.

Port `mount.ts`, `window.ts`, and `zoom.ts` host behavior into thin adapters: width observation without feedback loops, hidden/disconnected deferral, moved-window ownership, font loading invalidation, one mount per element, unique prefixing, last-good drawing retention/dimming plus errors, sanitizer callback, `onRender`/`onError` semantics, source/result/error state, updates, refresh and destruction. Cleanup removes observers, font subscriptions, pointer/keyboard/wheel listeners and pending work without deleting the final drawing. Host callback exceptions follow upstream ordering (a completed inserted drawing stays when `onRender` throws).

Zoom includes fitting/clamping, maxScale, optional controls/localized labels, wheel modifiers, pinch, drag, double-click, keyboard, focus indication and reset. CSS and Lucide reset icon license must accompany the port. Browser DOM work may be ordinary JS, but viewport math can be shared Rust where that lowers duplication. Do not rewrite interaction behavior in the name of simplifying the engine.

Native Swift provides a document/controller layer with render/update/refresh/dispose, viewport bounds/theme/font/time inputs, last-good render state, errors, approved link hit targets and accessible descriptions. AppKit owns resizing, font/theme observation, gestures/keyboard, focus and actual URL opening; C/Rust returns actions instead of opening links. Browser-only Node/HTMLElement callbacks are mapped to native typed callbacks, not meaningless JSON fields. Prove native zoom/interaction in a small standalone view example; a string-returning Swift CLI only proves the engine bridge. This is an adapter contract equivalence, not a claim that Swift exposes a DOM.

## 10. C/WASM ownership and versioned transport

Keep all existing immutable `PeleResult` access/free symbols. Add versioned full entry points instead of changing existing meanings. A practical C surface:

```c
typedef struct PeleEngine PeleEngine;
typedef struct PeleDocument PeleDocument;
PeleEngine *pele_v2_engine_new(uint32_t mode); /* empty or full */
void pele_v2_engine_free(PeleEngine *);
PeleResult *pele_v2_call(PeleEngine *, const uint8_t *request, size_t len);
/* call: detect, supports, registered, capabilities, register_builtin, render_source */
PeleDocument *pele_v2_parse(PeleEngine *, const uint8_t *source, size_t len,
                          const uint8_t *options, size_t options_len,
                          PeleResult **error_out);
PeleResult *pele_v2_document_model(const PeleDocument *);
PeleResult *pele_v2_render_document(const PeleDocument *,
                                  const uint8_t *options, size_t len);
void pele_v2_document_free(PeleDocument *);
```

This surface is the first full-engine transport; resource callback configuration is added as a size/version-tagged C vtable with caller context, scoped to synchronous rendering. Every callback has explicit byte lengths, owned/borrowed duration, failure result and reentrancy rules; Swift callbacks cannot throw across C. `error_out` is initialized to NULL; parse success returns one document, failure returns NULL and one owned diagnostic. A document retains renderer/config independently of engine lifetime; free it explicitly. Engine mutation is caller-serialized; immutable document rendering may run concurrently only when its configured resource callbacks permit it. No shared last-error slot. Existing native panic containment remains mandatory.

V2 request is a tagged operation enum, not stringly named arbitrary Rust functions. V2 results remain `{version:2,ok,result|error}`. `capabilities` reports the upstream SHA, 33 known types, installed handlers and earned gates. WASM wraps Engine/Document classes around the same logic and exposes compatibility-shaped JS results. JS adapters translate native error fields to `PeleError`; no error category is inferred by message matching.

Do not publish C extension-handler vtables until an actual external custom handler is implemented and exercised. Rust and browser public registration must support replacement/custom modules at full completion; native C users can select built-in registrations now. If full arbitrary C extension registration is added, it must preserve parse-document handler lifetimes and owned model state rather than passing untyped borrowed pointers around.

## 11. Limits and safety without compatibility masking

Use upstream defaults in the full interface: source 50000 UTF-16 units, output 4000000 UTF-16 units, expanded flow/agent edges 5000. Host options may change them as upstream allows; document config cannot override host limits. Family-specific limits and behavior are inventoried from source and stress tests (for example packet row ceiling, layered dummy/sweep caps, URL decode rounds, cache bounds). Do not inherit v1's 256-node/1024-edge ceilings in full mode.

Add checked allocation arithmetic, bounded scans, explicit stack/work accounting and platform resource budgets. A platform ceiling that rejects an upstream-accepted case is a recorded compatibility difference, not a passing parity case. CI stress comparisons must separate intentional host budget results from parser mismatches and implementation failures. Avoid claiming equality for unlimited user-provided budgets; safety still requires finite host memory/work budgets. Before full sign-off, all pinned upstream cases must fit the tested compatibility budget.

Security acceptance covers escaped text/attributes, source-derived IDs, namespace collisions, style filters, URL obfuscation/schemes, encoded entities, malicious frontmatter keys, malformed UTF-8 transport, giant numbers, deep nesting, token explosions, image/icon trust and host callback behavior. Every valid output remains finite and within bounds. A parser failure never becomes executable HTML in browser error UI. Preserve compatibility message/snippet values without logging private diagrams. Full SVG may legitimately contain anchors/images/style attributes/animation; the v0.1 blanket element denylist must evolve into the actual pinned safe-output policy, not be disabled.

## 12. Complete parity evidence, not sample evidence

Pin an oracle checkout and build its package once for development. Every extraction run records SHA, source/test file hashes, extraction tool version, fixture count, exclusion ledger and license origins. Extract from **all** `tests/corpus/*-docs.json`, `tests/compat/**` specs/grammar fixtures, family oracle/model/parser/reference tests, render/security tests, generic render/font/max-width/mobile tests, stress tests and documented website examples. Use AST extraction or the existing upstream corpus loader where appropriate; count dynamic/generated cases separately and reproduce their fixed seeds. Do not execute arbitrary fixture strings as source code. Any unextractable executable case is manually ported as a public-interface integration case and accounted for by file/test name.

The reference is public Pele `parse/render/detectType/supports` and host wrappers, not Mermaid directly. Upstream already records intentional deviations from Mermaid; blindly adopting its Mermaid oracle exclusions as Rust exemptions is invalid. Normalize public models with explicit, reviewed family adapters. Preserve meaning/order/defaults; do not delete unsupported fields to get equality. The same fixture may have separate expectations for detection, parsing, normalized model, error, render and host operation.

Canonical rendering runs use injected fixed time, fixed text measurer, fixed icon resolver/assets, fixed ID prefix, theme, width and options. Compare output DOM/scene topology, attrs, resolved styles, accessible strings, path commands and numeric geometry with an initial 1e-6 tolerance for equivalent arithmetic. Any larger difference requires a per-field explained tolerance; no broad rounding-away. Exact SVG output is desirable where the emitter is directly translated, but irrelevant attribute serialization order is normalized. Compare text SVG separately from native outlined export; both must preserve visible label content, line breaks, style, dimensions and semantic structure. Native and WASM engine modes using identical resources should yield identical canonical JSON/SVG.

Every family must cover all valid seeds and upstream rejection seeds; mutation/generation campaigns have fixed recorded seeds and operation counts. Use differential minimization for mismatches, preserving the minimized reproducer as a permanent public integration fixture. Public render integration checks verify document parse/reuse, model correctness, SVG XML/safety, actual raster output, limits and recovery. No new unit tests of private lexer/layout helpers. Upstream private tests contribute cases, but the Rust assertions call exported interfaces.

Run actual C lifecycle tests, Swift model/render/native-view integration and browser WASM against the same manifest. Browser checks run Chrome, Firefox and WebKit serially with DOM/font/assets/resize/theme/zoom/error/recovery paths. Native proof covers real PNG/view rendering, links/hit geometry, accessibility and gestures; build success is not feature proof. Record each platform/architecture actually run. CI reports attempted/passed/failed/skipped counts per gate and family; skipped/unimplemented counts prevent full parity sign-off. Performance compares pinned bounded stress cases and captures work/time/memory, without inferring general speed superiority.

## 13. First atomic implementation layer

Implement **source preparation and truthful type capability discovery** first. It is one feature slice; do not simultaneously rewrite layout or add several diagrams.

Files: `compat/source/{preprocess,yaml,entities,detect,locations}.rs`, `compat/{options,error,registry}.rs` and minimum public `compat` entry; only necessary adapter calls for `detect/supports/capabilities`. Registry lists all 33 known types with no fake handlers; the existing restricted flowchart engine remains available only through legacy v1 and does not qualify as a fully compatible registered handler. Define the full typed `DiagramType`, compatibility errors/config values and source-size accounting now; introduce family model variants only as their real implementations arrive.

Acceptance: every detector example, precedence/case/prefix case, upstream preprocessing/YAML/entity fixture and malformed case runs through the exported preparation/discovery interface; UTF-16 limits and locations match; `supports` is false for an unregistered detected family and false on preprocessing failure; legacy v1 fixture outputs remain identical. Rust/C/WASM call the same Rust preparation path. Report zero rendering parity progress from this layer. Commit this coherent slice after targeted public integration checks.

Next atomic layer is one **complete** diagram, preferably pie: direct parser/model + actual pie/donut/legend/layout options, all relevant labels/theme/safety/accessibility and public render/model interfaces. This establishes scene/render/resource/transport seams with a small but complete family. Then expand complete shared flowchart + compound support, followed by families that consume it. A family may require multiple internal commits, but is never marked complete before its whole row's gates pass.

Recommended dependency order (not concurrent work authorization): preparation/discovery; common scene/text/theme and complete pie; flowchart lexer/model; flowchart full rendering/shared layered+compound; swimlane; class/ER/state/requirement/usecase; sequence; chart/timeline/outline families; independent specialized architecture/block/C4/git/Sankey/railroad/agentflow/eventmodeling/Wardley/Cynefin/Venn. The matrix, not this ordering, defines completion. Browser mount/zoom and native controller layers consume real parsed documents once a complete family exists; finish them before a full release claim.

## 14. Release gate and licensing

Retain pinned upstream MIT author notice in derived modules/distributions, Mermaid fixture/spec notices, bundled-font OFL, and Lucide ISC if its reset path is ported. Generate a provenance map listing every source module represented or deliberately replaced and its acceptance evidence. Native text/font/resource extensions are documented separately from upstream behavior.

A full-parity release requires every row and all four gates complete, zero unexplained mismatch/skip, complete public adapter tests, all packages built and exercised as external consumers, current CI evidence, and exact documentation of platform representation differences. Do not relabel v0.1 as full, infer parity from 33 detector strings, or hide missing rows behind “supported.” Any necessary exception remains visible in the parity report and qualifies the public claim.
