# Pele Rust: first flowchart slice

Status: implementation contract for the initial standalone release. This is a focused Rust port inspired by and attributed to Pele, not full Pele or Mermaid compatibility. The first delivery is a working library, C/Swift adapter, and browser WASM example using one engine and one fixture corpus.

## 1. Scope and provenance

Repository: `https://github.com/vijayksingh/pele-rs`. No Kamui, MetalUI, Apple, browser, filesystem, network, clock, or thread dependency in the engine. Native and browser consumers supply source and concrete render options and receive SVG plus geometry.

Inspected upstream: [obsidianmd/pele at 458322a5b8abb1ae57ddf127c32017ab9fb14a92](https://github.com/obsidianmd/pele/tree/458322a5b8abb1ae57ddf127c32017ab9fb14a92), checked 2026-10-18. Relevant implementation:

- `src/diagrams/flowchart/{lexer,parser,db,types,graph,render,shapes}.ts`: lexer/parser, mutable parsed model, resolved drawing graph, rendering.
- `src/layout/layered.ts`: flat Sugiyama layout; cycle handling, rank assignment, virtual nodes, bounded crossing sweeps, coordinate assignment, routing. `src/layout/compound.ts`: nested group layout and direction transforms.
- `src/text/{measurer,metrics,label}.ts`: browser canvas or approximate Arial metrics and label layout.
- `src/svg/{builder,edges,shapes,theme}.ts`: SVG escaping, paths, shape intersections, and CSS-variable themes.

Upstream uses more capable grammar, compound layout, and routing than this slice. Preserve its architecture's separation of parsing, graph normalization, geometry, and output, while making every accepted construct work across native and WASM. The specified deterministic layered implementation is not a claim of an exact translation or visual equivalence to upstream's complete layout.

`UPSTREAM.md` must record commit, inspection date, source-to-port mapping, intentionally deferred features, and whether each module is translated or independently implemented. Keep upstream's MIT notice, including `Copyright (c) 2026 kepano`, in `THIRD_PARTY_NOTICES.md` with the full license. Any copied Mermaid fixtures/specs require their own original MIT notices. No need to copy upstream's Lucide reset icon; if copied later retain its ISC notice too. Bundle a pinned Noto Sans Regular font with its OFL-1.1 license and exact origin/hash; do not redistribute a system-installed font. Root license is MIT with appropriate port author attribution, without deleting upstream notices from derived work.

Kamui conventions consulted: `Native/kamui-medium/src/{api,ffi,wasm}.rs`, its `Cargo.toml`, `Sources/Medium/MediumCore.swift`, and `scripts/build-medium.sh`. Reuse only the general pattern: pure Rust, owned UTF-8 results, C ABI, Swift copy then free, wasm-bindgen adapter, native unwind containment. No Kamui source or dependency is required. Its single-character WASM status prefix is not adopted; use an explicit versioned JSON envelope.

## 2. Public interface and module shape

The deep module is the flowchart engine. Its external seam consists of a typed Rust interface plus one versioned JSON render operation for foreign adapters. Parsing, text shaping, layout, and SVG construction are private implementation modules; callers do not orchestrate rank or glyph internals.

```rust
pub fn parse(source: &str) -> Result<Flowchart, Error>;
pub fn layout(graph: &Flowchart, options: &RenderOptions)
    -> Result<Layout, Error>;
pub fn render_svg(layout: &Layout) -> Result<String, Error>;
pub fn render(source: &str, options: &RenderOptions)
    -> Result<Rendered, Error>;
pub fn render_json(source: &str, options_json: &str) -> String;
```

`render` is convenience composition, not a second engine. `Layout` owns its resolved theme, positioned nodes/edges, shaped text and final bounds; `render_svg` cannot accidentally use different metrics or colors. Keep construction fields private where invariants matter; public accessors or serializable geometry expose useful data. A typed graph built by consumers must undergo the same validation and resource checks as parsed source. The only public JSON input in v1 is source plus render options; arbitrary deserialization must not bypass graph/layout validation.

Proposed files:

```text
Cargo.toml, Cargo.lock, rust-toolchain.toml
crates/pele/src/lib.rs     public composition and re-exports
crates/pele/src/model.rs               graph and geometry value types
crates/pele/src/error.rs               stable errors and source locations
crates/pele/src/parser.rs              bounded scanner + flowchart grammar
crates/pele/src/text.rs                embedded font, shaping, glyph outlines
crates/pele/src/layout.rs              layered layout + routes
crates/pele/src/svg.rs                 safe bounded SVG writer
crates/pele/src/contract.rs            v1 JSON options/envelope
crates/pele-ffi/src/lib.rs C ownership, pointer checks, panic containment
crates/pele-wasm/src/lib.rs wasm-bindgen string adapter
include/pele.h
bindings/swift/Package.swift
bindings/swift/Sources/CPele/        header/module map or thin C target
bindings/swift/Sources/Pele/         Swift value types + throwing wrapper
bindings/swift/Sources/PeleSmoke/    real command-line integration example
examples/web/index.html, examples/web/main.js browser editor + rendered SVG + errors
scripts/build-native.sh, scripts/build-wasm.sh, scripts/verify.sh
assets/NotoSans-Regular.ttf, assets/OFL.txt
fixtures/manifest.json, fixtures/*.mmd
fixtures/expected/*.json    semantic assertions, not private snapshots
crates/pele/tests/conformance.rs public Rust interface integration tests
tests/ffi.c               real C-link ownership/error integration test
```

Paths may be consolidated if it reduces shallow wrappers; responsibilities and contract must remain recognizable. Use a Cargo workspace with `pele` (pure engine, `rlib`), `pele-ffi` (native `staticlib` and optionally `cdylib`), and `pele-wasm` (WASM `cdylib`). Only adapters depend on platform tooling. Native and WASM adapters each depend on `pele`; neither duplicates rendering. Keep testable Rust libraries available where needed, and verify Apple static-archive linkage instead of assuming simultaneous crate-type output works. Pin wasm-bindgen dependency and CLI together (installed baseline 0.2.128); pin Rust toolchain (available baseline 1.98.1). Native release uses `panic = "unwind"`; WASM traps cannot be caught by Rust `catch_unwind`.

## 3. Supported syntax: exact first contract

A document starts with `flowchart DIR` or `graph DIR`; DIR is `TB`, `TD`, `BT`, `LR`, or `RL`. `TD` normalizes to `TB`. Direction is mandatory in v1. Leading blank lines and `%%` line comments are accepted. Statements are separated by newlines or semicolons outside labels. CRLF is supported. Comments begin with `%%` outside labels and continue to the next newline. The complete source must be consumed.

Identifiers are ASCII `[A-Za-z_][A-Za-z0-9_]*`, case-sensitive. Endpoints may declare a node or refer to an existing node. First mention fixes stable node order; last explicit shape/label declaration wins; a bare reference never resets a declared label or shape. An implicit node uses its identifier as label and rectangle as shape. Duplicate edges remain separate edges.

Supported node forms:

| Form | Meaning |
| --- | --- |
| `A` | implicit rectangle labeled A |
| `A[Text]` | rectangle |
| `A(Text)` | rounded rectangle |
| `A{Text}` | diamond |
| `A((Text))` | circle |
| `A([Text])` | stadium |

Supported edge operators: `-->` solid arrow, `---` solid line, `-.->` dashed arrow, `-.-` dashed line, `==>` thick arrow. Optional edge label immediately follows the operator as `|Text|`, including a quoted string inside the pipes. Chains such as `A --> B{Ready?} -->|yes| C` expand to separate edges in textual order. No implicit all-to-all expansion. Self-loops, reverse links, cycles, disconnected components and joins are supported graph behavior, not syntax exceptions.

A label is plain text. Double-quoted labels support `\"`, `\\`, and `\n`; unrecognized escape sequences fail. Quoting lets literal shape delimiters and separators occur inside labels. Bare labels stop at their matching delimiter; unsupported nested delimiter syntax fails. Actual newline inside a quoted label may be accepted as an explicit line break, provided location accounting remains correct. Empty explicit labels are allowed. HTML is never interpreted: strings such as `<script>` or `<br>` remain literal visible text, escaped in metadata. Backtick markdown syntax fails with `unsupported_syntax`; it must not silently become formatted output. Unicode is permitted syntactically and then validated against the text contract below.

Explicitly reject (with location) `subgraph`/`end`, local `direction`, frontmatter, init/config directives (`%%{...}%%` must not be swallowed as comments), `style`, `class`, `classDef`, `linkStyle`, `click`, `:::`, `&`, edge IDs, shape metadata `@{...}`, links with extra length markers, bidirectional/circle/cross arrows, invisible links, unsupported shapes, and every non-flowchart diagram type. Detect reserved statement keywords before creating an implicit node. Do not flatten or discard subgraphs. A graph with only a header returns an `empty_graph` error.

Compatibility documentation must publish these accepted forms and rejection examples. This slice does not claim full Mermaid identifier grammar, subgraphs, rich text, YAML configuration, interaction, or upstream pixel parity.

## 4. Graph and geometry contracts

`Flowchart` contains canonical `direction`, ordered nodes, and ordered edges. `Node` has stable source ID, plain label, shape and source span. `Edge` has stable numeric index, source/target node indexes, label, line style, arrow-end flag and source span. Use vectors for iteration and an ordered or lookup-only map for ID resolution; hash-map iteration must never choose geometry or SVG order. No cycles are removed from this model.

`Layout` contains graph direction, resolved theme, `width`, `height`, nodes, edges, and shaped text. Each node exposes `id`, `label`, `shape`, and bounding rect `{x,y,width,height}`. Each edge exposes `index`, `source`, `target`, `label`, `points: [{x,y}]`, optional label rect, style and arrow flag. Coordinates use top-left origin, x right, y down, SVG user units equivalent to CSS pixels at 1x. Node boxes exclude stroke; output bounds include strokes, arrowheads, labels, return lanes and outer padding. A geometry schema version is returned with every render.

Geometry must remain finite and within declared canvas limits. Preserve source direction on edge points: the first point belongs to the source shape and the final point to the target shape even when ranking temporarily reverses an edge. Do not return internal virtual nodes as user nodes. Quantize final numbers to 0.001 units before serialization; use one formatter for JSON-observable geometry and SVG, normalize negative zero, forbid NaN and infinity. Same source/options must return byte-identical SVG and geometry on native and WASM for the fixture corpus.

## 5. Deterministic bounded layout

Implement one layered flat-graph engine, not a vertical list of nodes or force simulation. Canonical flow coordinates run down; horizontal directions swap the sizing axes before layout. Transform final node centers, dimensions, edge paths and label boxes back to requested orientation; text itself remains upright.

1. Shape every node/edge label and compute dimensions. Node separation must use actual dimensions; diamond and circle sizes must contain the complete label rect, not merely its width. Keep explicit multiline labels and padding. Long single lines may grow a node until the canvas budget; automatic text wrapping is deferred.
2. Discover weakly connected components in first-mention order. Lay each independently; pack components across the perpendicular axis with component separation. Disconnected nodes remain visible. This avoids artificial connections between unrelated subgraphs.
3. Run iterative DFS in stable node/edge order to identify feedback edges and self-loops. Remove only those edges from the temporary ranking DAG; retain them for final routing. A cycle is never an error solely because it is cyclic. Use longest-path ranks in topological order; joins sit after every forward predecessor, not at first-discovery depth.
4. Insert virtual nodes for forward edges spanning multiple ranks; reserve real label-width space on the route's selected middle rank/gap. Check total virtual nodes before allocating. Node order initially follows source order. Run at most eight alternating barycenter/median sweeps with stable index tie-breaking. Keep the candidate with fewest crossings, then least departure from declaration order; do not search indefinitely.
5. Assign perpendicular coordinates using size-aware separation and bounded neighbor alignment/compaction. Nodes and virtual route corridors in a rank cannot overlap. Forward and backward alignment passes may improve joins, but preserving separation takes priority. Rank distances include maximum node extent, labels and dedicated inter-rank track space.
6. Route forward edges through their virtual corridors and inter-rank tracks. Distribute distinct ports for fan-in/fan-out in target order. Parallel edges get distinct lanes. Source and target attachment points are clipped against the actual rectangle/rounded/circle/diamond/stadium outline. The final arrow segment must point toward the target, with the line shortened by arrowhead length.
7. Route feedback edges through dedicated exterior return lanes, assigned by stable edge index, with departure/arrival corridors that do not cross unrelated node interiors. Self-loops get an exterior loop around their own node with sufficient reserved space and distinct lanes for multiple loops. Reserve lanes in overall bounds. Edge labels occupy reserved route space and must not overlap nodes or another label. If the finite route/label budget cannot produce valid placement, return `layout_limit` rather than silently clip or omit content.
8. Validate node non-overlap, label containment, finite bounds and nonincident edge/node intersections through the public layout result. Orthogonal polylines with modest rounded corners are sufficient. Crossings between different edges may remain; crossing minimization and globally optimal routing are not promised. Shared arrowheads or coincident parallel edge paths are not acceptable.

The initial implementation may use an iterative order-based acyclic orientation instead of DFS feedback detection if it preserves all original edges and follows the same deterministic ranking/routing contract. Keep this decision local to layout. Complexity is bounded by graph caps, virtual-node cap, eight sweeps and bounded route construction; no recursive traversal over untrusted graph depth.

## 6. Text and concrete themes

Bundle pinned Noto Sans Regular under OFL. Use `rustybuzz` to shape supported runs and `ttf-parser` to obtain the matching glyph outlines. The same embedded bytes drive advances, ascent/descent, labels' bounds and SVG paths on every target. Cache only within a render call, keyed by text and font size; no global unbounded cache. Reset glyph pen per line, apply glyph offsets and advances, and flip font y coordinates into SVG space. Account for glyph ink overhang as well as advance width when sizing.

V1 supports left-to-right runs covered by the bundled font (including its Latin/Greek/Cyrillic coverage and combining marks). It does not implement Unicode bidi paragraph layout or font fallback. Reject unsupported RTL/bidirectional runs or bidi controls with `unsupported_text`, and reject missing glyph ID 0 with a precise `unsupported_glyph` diagnostic rather than drawing tofu or deleting text. A covered combining-mark sequence must retain rustybuzz shaping. CJK/emoji and other scripts absent from the font are explicit limitations. Rendering is not limited to ASCII, but must not claim universal Unicode support.

SVG visible text is glyph paths, not `<text>` dependent on host fonts. This gives native SVG rasterizers and browser SVG the same glyph geometry without font installation, CSS font loading, or `foreignObject`. Each node/edge keeps escaped textual `<title>` metadata, and the SVG root has `role="img"` plus an accessible title/description. The web example also exposes source/text for accessible inspection; outlined glyphs alone are not selectable text.

`RenderOptions` uses a concrete `Theme` with colors serialized as restricted hex (`#RRGGBB`, optionally `#RRGGBBAA` only if supported consistently), finite numeric font size/padding/gaps/radius/stroke width, and optional plain accessible title. Defaults approximate Pele's current neutral palette: background `#ffffff`, foreground `#1f1f1f`, surface `#f3f3f3`, border `#8a8a8a`, line `#1f1f1f`; neutral accent is optional until used. Sensible baseline: node text 16, edge text 14, node gap 40, rank gap 48, outer padding 16, radius 4. Values belong to this standalone engine; consumers can map their own tokens into them.

Never emit `var(...)`, `currentColor`, stylesheet selectors, externally resolved CSS, font URLs or arbitrary caller-provided style strings. Native compatibility depends on resolved presentation attributes and paths. Validate options before shaping; unknown JSON fields fail instead of masking misspelled configuration. Default options are applied only when fields are absent, never when invalid. No host-measurement callback in v1: two adapters already share one stable font implementation, so an additional measurement seam is unnecessary.

## 7. Safety, errors and budgets

Errors are values, not strings scraped by consumers:

```json
{"code":"unsupported_syntax","message":"Subgraphs are not supported in v1",
 "span":{"start":13,"end":21,"line":2,"column":1}}
```

`start`/`end` are UTF-8 byte offsets; line/column are one-based, column counts Unicode scalar values. A source span is optional for option/ABI errors. Stable codes include `invalid_utf8`, `syntax`, `unsupported_syntax`, `unsupported_diagram`, `empty_graph`, `unsupported_text`, `unsupported_glyph`, `invalid_options`, `input_limit`, `graph_limit`, `layout_limit`, `output_limit`, `internal`. Messages may improve without changing codes. Error strings must not dump the entire document.

Hard v1 ceilings (callers may lower, never increase them): source 262144 UTF-8 bytes; options JSON 16384 bytes; 256 nodes; 1024 edges; 128-byte identifiers; 4096 bytes per label; 64 explicit lines per label; 8192 virtual nodes; 65536 shaped glyphs per diagram; canvas width/height at most 100000 each; SVG at most 8 MiB; complete JSON result at most 16 MiB. Font size 8–72; spacing/padding 0–512 with positive required separations; stroke width 0.5–8. Validate finite numbers and range before allocating dependent arrays. Bound format/output growth incrementally, not after building an arbitrarily large string. XML 1.0-invalid controls are rejected at parse/options validation. Arithmetic uses checked counts and finite-coordinate validation.

SVG allows only engine-generated root/group/path/rect/circle/polygon/title/desc elements and fixed presentation attributes. Escape all plain text and attribute strings; source IDs are metadata or mapped to safe internal numeric IDs, never copied into markup structure. Do not emit scripts, event handlers, links, images, foreignObject, user markup, user CSS or external references. Arrowheads can be direct triangles, avoiding marker-ID collisions when multiple diagrams are mounted. No side effects occur during parsing/rendering.

## 8. C ABI, JSON and ownership

Use a small versioned ABI, explicitly independent of Rust object layout:

```c
#define PELE_ABI_VERSION 1
uint32_t pele_abi_version(void);
typedef struct PeleResult PeleResult;
PeleResult *pele_render(const uint8_t *source, size_t source_len,
                        const uint8_t *options, size_t options_len);
uint32_t pele_result_status(const PeleResult *result);
const uint8_t *pele_result_data(const PeleResult *result);
size_t pele_result_len(const PeleResult *result);
void pele_result_free(PeleResult *result);
```

`PeleResult` owns one immutable UTF-8 envelope allocation. Every non-null result, success or error, must be freed exactly once. Data has explicit length, is not promised NUL-terminated, and remains valid only until result free. Swift copies bytes before freeing. `free(NULL)` is a no-op. Null result is reserved for an unrecoverable adapter/allocation failure; callers synthesize an `internal` error. Accessors on NULL return safe sentinel values (`status = internal`, data NULL, len 0). Double-free, accessing freed handles, invalid non-null pointers or lying about readable buffer length are caller violations; the library cannot validate arbitrary memory addresses.

Null source/options pointers are accepted only with zero length. Empty options bytes select defaults; empty source produces a normal structured error. Validate declared lengths against ceilings before making slices. C input buffers remain caller-owned and are borrowed only for the synchronous call. Returned results are independent; no thread-local last error, global mutable state, or caller-supplied output buffers. Calls may run concurrently on separate inputs; caller must synchronize result destruction against reading.

Native extern entry points contain unexpected unwind with `catch_unwind`; no panic may cross C. Ordinary parse/layout failures use `Result`, not panics. OOM/abort and invalid caller pointers are outside unwind recovery. Keep all unavoidable `unsafe` in `crates/pele-ffi` and the minimum release/memory access helpers.

`status` is 0 for success, 1 for input/syntax/options/text error, 2 for budget/layout/output limit, 3 for internal failure. Detailed codes live in JSON. Envelope shape is identical through C and WASM:

```json
{"version":1,"ok":true,"result":{"svg":"<svg ...>","width":420.0,
 "height":240.0,"geometry":{"version":1,"direction":"LR","nodes":[],"edges":[]}}}
```

```json
{"version":1,"ok":false,"error":{"code":"syntax","message":"Expected node","span":{"start":19,"end":20,"line":2,"column":7}}}
```

One result does not contain both `result` and `error`. Adapter status agrees with the envelope. JSON schema is separate from ABI number so additive result evolution can be managed explicitly; v1 wrappers reject unsupported envelope versions.

## 9. Swift and browser delivery

SwiftPM wrapper exposes `Pele.render(_ source: String, options: RenderOptions = .init()) throws -> RenderedDiagram`. It encodes options, calls the C interface with `withUnsafeBytes`, uses `defer` to free any non-null result, copies exact-length bytes to `Data`, checks envelope version, decodes typed success/error values, and returns SVG plus dimensions/geometry. No private pointers escape. Verify `pele_abi_version()` before rendering. Build native static archive from this repository, stage headers/module map and link it through the package; build instructions must work outside Kamui. An XCFramework release package is a later distribution layer unless produced and tested now.

Returning an SVG string proves only the bridge. Native proof also renders the produced SVG into a PNG using a real native SVG renderer. First exercise `NSImage(data:)` with current engine output and export through `NSBitmapImageRep`; successful decoding and painted pixels must be measured. If this SVG subset is unsupported, use an optional `resvg` dependency in `pele-ffi`, exposing `pele_rasterize_svg(svg_ptr, svg_len, scale) -> PeleResult*`; its successful output bytes are PNG (status 0), while failures retain the JSON error envelope. This operation must be documented as binary-result rather than render-envelope data. Scale must be finite, positive, capped (e.g. 0.25–4), with a 16-million-pixel raster ceiling before allocation. Reject external resources; engine-generated path SVG needs no fonts. Swift copies PNG bytes before freeing, constructs an `NSImage` or writes the image, verifies nonzero dimensions and painted pixels, and opens the artifact for inspection. A separate optional verification binary is acceptable if it provides the same actual native raster proof. Do not generalize observed `NSImage` support for this constrained subset to arbitrary SVG. A SwiftUI integration example can display a rasterized `NSImage`, while Rust/C Swift library itself remains renderer-independent. Installing or changing Kamui is not part of this slice.

WASM exports `render(source: &str, options_json: &str) -> String` and `version() -> u32` through wasm-bindgen. They invoke the same `render_json` composition. Browser JavaScript parses the versioned envelope, surfaces typed errors, and mounts only successful engine-generated SVG. Catch WASM traps and show a stable internal error; reinitialize before a subsequent attempt if module state might be compromised. Keep user error messages in `textContent`, never `innerHTML`.

`examples/web/index.html` is a small standalone playground served over HTTP, importing generated web-target wasm-bindgen output. It has source editor, direction/fixture examples, render action, SVG preview and visible diagnostics. Load fonts zero times: glyph outlines are embedded in output. Initial sample demonstrates fork, join, labeled decision, feedback edge and more than one node shape. Keep generated WASM glue/artifacts out of source control if the documented build produces them; do not claim published npm/crates/Swift releases from local builds.

## 10. Verification through public interfaces

No unit tests. Use feature-slice integration/e2e checks through exported Rust functions, real C ABI, actual Swift wrapper, and real browser WASM. Store a common fixture manifest with source path, expected accepted/rejected status, node/edge counts, direction and error code where applicable. Include:

- chain and standalone declarations; each supported shape/operator; quoted delimiters, semicolons and escaped newlines;
- fork/join (join ranked after both parents), long skip edge, disconnected components;
- cycle of at least three nodes, self-loop, duplicate/parallel links;
- all four canonical directions, multiline and wide labels, combining marks;
- labels containing markup delimiters and injection-shaped text;
- subgraph, styles/click, directive/frontmatter, unsupported shapes/operators, malformed quotes/UTF-8, unsupported glyph and RTL text;
- source/node/edge/label/virtual/output limits, zero/NaN/negative/huge options, repeated success/error/free cycles.

Rust integration assertions inspect public graph/layout and SVG: counts/endpoints are preserved; joins have appropriate ordering; node interiors do not overlap; labels fit own shapes and avoid other nodes; nonincident route segments avoid node interiors; edge directions and arrow endpoints are correct; geometry finite/in bounds; repeated runs equal; no disallowed SVG constructs. Parse SVG with an independent XML parser and rasterize representative cases to ensure output is meaningful. Do not use brittle full-output snapshots as the only oracle.

The C executable links the actual archive and exercises success, structured failure, null/zero-length inputs, invalid UTF-8 and ownership across repeated calls. Swift executes the same success/rejection corpus through its wrapper and writes representative SVG/PNG artifacts. Browser loads the actual `.wasm` through generated glue, renders representative fixtures, checks live SVG bounds/visible paths, drives malformed source then recovery to valid source, and renders the security fixture without active elements. Compare canonical geometry and SVG from Rust, Swift/C and browser for the common corpus; image pixel equality across different rasterizers is not required.

Run focused Rust integration checks during implementation, then one consolidated Rust + native + Swift + browser pass. `cargo fmt --check`, `cargo test --locked --test conformance`, relevant C/Swift scripts, WASM build, browser proof and `git diff --check` are release evidence. Record actual target architectures and any unavailable checks. Compilation is separate from execution and visible rendering. Coherent architecture and implementation changes are separate commits; preserve unrelated Kamui work and do not push unless user authorization covers it.

## 11. Following layers

First ship this contract end-to-end. Next, independently reviewed layers can add compound subgraphs (real containment and group ports, never flattening), a larger grammar fixture matrix, configurable font collections/bidi, additional shapes, restricted declarative styling, and further diagram families. A full upstream-compatible port requires a feature-by-feature compatibility inventory and upstream differential fixtures. Keep each new semantic capability behind the same engine interface, with adapters gaining only fields that consumers genuinely need.
