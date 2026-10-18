# Upstream relationship

Reference: [obsidianmd/pele](https://github.com/obsidianmd/pele), commit
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`, inspected 2026-10-18.
This is an independent Rust adaptation, not an official Pele release. No upstream
TypeScript, generated parser, Mermaid specs, or Lucide artwork is copied into
the engine. Upstream MIT attribution is retained conservatively in
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

First-slice omissions: subgraphs/compound layout, full Mermaid identifier grammar,
rich shapes, CSS classes/styles, callbacks/URLs/images, frontmatter/configuration,
Markdown/HTML labels, font fallback/bidi, and non-flowchart diagram families.
See `docs/LLD.md` and README for the exact contract and ceilings.

`THIRD_PARTY_NOTICES.md` inventories bundled notices. The unmodified font's pinned
origin and SHA-256 are in `crates/pele/assets/fonts/README.md`.
