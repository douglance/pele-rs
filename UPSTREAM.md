# Upstream relationship

Reference: [obsidianmd/pele](https://github.com/obsidianmd/pele), commit
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`, inspected 2026-10-18.
This is an independent Rust adaptation, not an official Pele release. The legacy renderer is independently implemented. The new `compat/source` layer
translates upstream detection, preprocessing, YAML and entity encoding into Rust.
Pinned documentation examples and expected preparation output are retained as
development fixtures. No JavaScript engine or upstream renderer runs in distributed
Rust/native/WASM code. Upstream MIT attribution is retained in
`third_party/PELE-LICENSE` and `NOTICE`.

| Rust module | Upstream reference | Relationship |
| --- | --- | --- |
| `parser.rs`, `model.rs` | `diagrams/flowchart/{lexer,parser,db,types,graph}.ts` | Independently implemented bounded grammar/model |
| `layout.rs` | `layout/layered.ts`, `diagrams/flowchart/graph.ts` | Independently implemented layered layout; eight crossing sweeps, virtual routes and explicit return lanes |
| `text.rs` | `text/{label,measurer,metrics}.ts` | Different implementation: pinned Noto Sans shaping and glyph outlines instead of host canvas/Arial approximation |
| `svg.rs` | `svg/{builder,edges,shapes,theme}.ts` | Independently implemented resolved colors and simple paths, no CSS variables |
| C/Swift/WASM adapters | Public `core.ts` interface; Kamui's general adapter pattern | New implementation; no Kamui dependency |

The npm development dependency pins Pele 1.6.0 as a public-interface semantic
comparison oracle. Tests compare node IDs, endpoints, counts and plain labels for
accepted fixtures. Pele preserves literal `\\n` in its model and interprets it at
render time; Rust normalizes this quoted escape into a newline during parsing.
Injection/combining-label representation is not asserted against Pele because
its model has internal entity encoding. This is a limited compatibility check,
not a full differential grammar corpus or visual-equivalence claim.

The following omissions apply to the released legacy v0.1 renderer; all are
required work under `docs/FULL_PARITY_LLD.md` for the full interface.

First-slice omissions: subgraphs/compound layout, full Mermaid identifier grammar,
rich shapes, CSS classes/styles, callbacks/URLs/images, frontmatter/configuration,
Markdown/HTML labels, font fallback/bidi, and non-flowchart diagram families.
See `docs/LLD.md` and README for the exact contract and ceilings.

`THIRD_PARTY_NOTICES.md` inventories bundled notices. The unmodified font's pinned
origin and SHA-256 are in `crates/pele/assets/fonts/README.md`.

## Development compatibility source map

| Rust module | Pinned Pele source | Relationship |
| --- | --- | --- |
| `compat/source/detect.rs` | `src/detect.ts` | Ordered detector translation, including prefix/case/anchor quirks |
| `compat/source/preprocess.rs` | `src/preprocess.ts` | Frontmatter, directives, comments and HTML quote normalization |
| `compat/source/yaml.rs` | `src/util/yaml.ts` | Custom JSON-schema YAML reader translation |
| `compat/source/{entities,decode}.rs` | `src/preprocess.ts`, `src/text/entities.ts` | Entity encoding/decoding and bounded color-semicolon scan |
| `compat/{value,error}.rs` | `src/{preprocess,errors}.ts`, `src/util/chars.ts` | Ordered JS-compatible config values, scalar formatting and UTF-16 diagnostics |
| `compat/{engine,module}.rs` | `src/core.ts`, `src/types.ts` | Typed Rust module/resource interfaces and parse provenance; full built-ins pending |
| `compat/diagrams/pie/` | `src/diagrams/pie/{parser,model,render}.ts`, `src/diagrams/common/{tokens,fit-width}.ts` | Pie parser, ordered model, sectors/donut layout, legends, width fitting and exact SVG translation |
| `compat/diagrams/packet/` | `src/diagrams/packet/{parser,model,render}.ts`, common literal-label/fit-text/fit-width | Packet rows, splitting, narrow reflow, labels and bit numbering; shared Rust label strings avoid repeated allocation |
| `compat/diagrams/quadrant_chart/` | `src/diagrams/quadrant/{index,lexer,parser,db,render,tokens}.ts` | Typed ordered styles/points, axes, quadrants, labels, point layout and responsive sizing |
| `compat/diagrams/radar/` | `src/diagrams/radar/{parser,model,render,index}.ts` | Axes/curves/options, reference entries, radial geometry, legends and width fitting |
| `compat/diagrams/timeline/` | `src/diagrams/timeline/{index,db,lexer,parser,render}.ts` | Periods/events/sections, colors, horizontal/vertical layout and narrow adaptation |
| `compat/diagrams/journey/` | `src/diagrams/journey/{index,db,lexer,parser,render}.ts` | Tasks/actors/scores, ordered sections, faces and horizontal/vertical layout |
| `compat/diagrams/class_diagram/` | `src/diagrams/class/{index,db,types,members,lexer,parser,tokens,graph,render,markers}.ts` | Generic members, namespaces, notes, interfaces, relation markers and actual compound rendering |
| `compat/diagrams/er/` | `src/diagrams/er/{index,db,types,lexer,parser,tokens,graph,render,markers}.ts` | Entities/attributes/groups/classes and full cardinality/table rendering |
| `compat/diagrams/xychart/` | `src/diagrams/xychart/{index,db,lexer,parser,tokens,ticks,render}.ts` | Axes, data series, ticks, clipping, legends and responsive layout |
| `compat/diagrams/common/generics.rs` | `src/diagrams/common/generics.ts` | Shared generic display formatting |
| `compat/diagrams/common.rs` | `src/diagrams/common/tokens.ts` | Hidden YAML greedy-optional-body behavior shared by pie, packet and radar |
| `compat/diagrams/flowchart/` | `src/diagrams/flowchart/{lexer,parser,db,types,shapes,graph,render}.ts` | Full declared models, graph normalization and actual layered/compound SVG rendering |
| `compat/layout/` | `src/layout/{layered,compound}.ts` | Actual rank/cycle/crossing/coordinates/routes, nested groups and bounded expansion translation |
| `compat/svg/{shapes,edges}/` | `src/svg/{shapes,edges,theme,cluster}.ts` | Full shape paths/intersections, markers/curves/loops/style resolution and cluster titles |
| `compat/diagrams/kanban/` | `src/diagrams/kanban/{index,db,render}.ts` | Columns/cards, metadata/icons/tickets, wrapped/folded board rendering |
| `compat/diagrams/mindmap/` | `src/diagrams/mindmap/{index,db,parser,render}.ts` | Typed branch tree, shapes/icons, balanced branches and narrow outline rendering |
| `compat/diagrams/common/{outline,icon}.rs` | `src/diagrams/common/{outline,icon}.ts` | Shared indentation/parser and icon layout behavior |
| `compat/source/url.rs` | `src/util/url.ts` | Sanitization and scheme gates; WHATWG HTTP normalization uses Rust url crate |
| `compat/text/` | `src/text/{label,measurer,metrics}.ts` | Rich labels, wrapping, style spans and deterministic Arial fallback metrics |
| `compat/svg/` | `src/svg/{builder,root,theme}.ts` | Escaping, labels, SVG root and theme tokens; native palette resolution is a Rust extension |
| `bindings/web/resources.js` | `src/text/measurer.ts` | Browser-only Canvas measurement/cache adapter; fallback advances execute in Rust |

`fixtures/pele/source.json` records exact upstream input-file hashes and revision.
`scripts/parity/extract-source.mjs` regenerates expected data from a clean pinned
checkout using Node's development-only TypeScript transformation. Its 34 source
corpora include 761 examples across every family and agentflow reference cases.
Mermaid fixture licensing is retained in `third_party/MERMAID-LICENSE`.

The twelve family fixtures and extractors pin source-file hashes and 151,426 public
parse/render scenarios. Large SVGs use exact SHA256 comparisons; complete models
remain present. A separate packet stress case checks shared labels and a 10,000-row
typed model without materializing 400 MB of JSON. The oracle JavaScript is a
development dependency only. Models/errors/SVG are checked through Rust, Swift/C
and three real browser engines. This does
not certify all public API behavior: six Class stress models explicitly exceed
projection budgets while exact typed/source rendering remains covered; JavaScript model mutations are not yet applied
to retained Rust documents, and lazy registration/mount/zoom remain pending.
Shared label model projections have an explicit 16 MiB JSON transport cap, while
typed Rust models and direct source rendering retain shared data. Native export
also repeats initial tspan coordinates on parent text and resolves generated
opaque sRGB color-mix halos for AppKit's SVG decoder. Mindmap model projection
preflights repeated tree expansion and rejects depth 48 before recursive encoding;
its typed model and source-render path support the tested depth 3,000.

The full-parity inventory remains the acceptance gate. Passing preparation cases
is not complete grammar/model/render compatibility. In particular, Rust/Swift UTF-8
transport cannot represent lone UTF-16 surrogates; YAML unpaired Unicode escapes
currently use replacement characters. This remains an explicit unresolved
compatibility case, not a passing parity exception. Init-JSON configs over the explicit 64-level work cap and
extreme-radix rounding also require broader conformance before that gate closes.
