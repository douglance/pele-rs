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

`fixtures/pele/source.json` records exact upstream input-file hashes and revision.
`scripts/parity/extract-source.mjs` regenerates expected data from a clean pinned
checkout using Node's development-only TypeScript transformation. Its 34 source
corpora include 761 examples across every family and agentflow reference cases.
Mermaid fixture licensing is retained in `third_party/MERMAID-LICENSE`.

The full-parity inventory remains the acceptance gate. Passing preparation cases
is not complete grammar/model/render compatibility. In particular, Rust/Swift UTF-8
transport cannot represent lone UTF-16 surrogates; YAML unpaired Unicode escapes
currently use replacement characters. This remains an explicit unresolved
compatibility case, not a passing parity exception. Init-JSON configs over the explicit 64-level work cap and
extreme-radix rounding also require broader conformance before that gate closes.
