# Pinned upstream preparation corpus

`source.json` is a development/test oracle for source preparation and detection,
not evidence of diagram parser/model/render parity. It includes every source in
Pele's 34 corpus files (33 diagram families plus agentflow reference models),
metadata examples, detector boundaries and fixed-seed malformed inputs.

Regenerate with Node 25+ and a clean checkout at the exact pinned commit:

```sh
node scripts/parity/extract-source.mjs /path/to/pele
```

The script verifies the checkout revision, records source SHA-256 digests, and
uses the pinned TypeScript preprocessing code solely to produce expected data.
`entity-links.json` adds public extension-link scenarios for entity decoding.
`pie.json` adds 3,678 complete public pie model/render/error scenarios, including
upstream examples, layout/configuration, host resources, rich labels, inaccessible
inputs and fixed-seed malformed sources. Regenerate with:

```sh
node scripts/parity/extract-pie.mjs /path/to/pele
```

| Family fixture | Scenarios | Render/model results | Errors |
| --- | ---: | ---: | ---: |
| pie | 3,678 | 1,179 | 2,499 |
| quadrant | 10,614 | 1,586 | 9,028 |
| packet | 12,774 | 2,924 | 9,850 |
| radar | 8,431 | 1,791 | 6,640 |
| timeline | 11,932 | 8,017 | 3,915 |
| journey | 10,725 | 3,244 | 7,481 |

Each has an `extract-FAMILY.mjs` generator. Complete expected models are retained;
large SVG strings may be replaced by SHA256 hashes of their exact UTF-8 bytes.
Packet has one additional top-level stress case: Rust checks every shared-label
row, and Swift/browser hosts compare source-render output without projecting a
400 MB model. JSON model projection now fails explicitly above 16 MiB.

Radar records two and quadrant three unrepresentable lone-surrogate mutation
sources explicitly in `exclusions`, with original UTF-16 code units. They are not
counted as passing UTF-8 cases. The shared UTF-16 transport gap remains unresolved.

`flowchart.json` is a separate parser checkpoint: 4,796 declared-model/error
scenarios, with explicit TypeScript-private cache exclusions. It does not certify
the full renderer or JavaScript model methods; see `docs/FLOWCHART_PORT.md`.

No oracle JavaScript ships in Rust, Swift, or WASM packages.

Pele: Copyright (c) 2026 kepano, MIT; see `third_party/PELE-LICENSE`.
Mermaid documentation examples: Copyright (c) 2014 - 2022 Knut Sveidqvist,
MIT; see `third_party/MERMAID-LICENSE`. Pele extracted those examples at
Mermaid `mermaid@12.1.0`, commit `21f72f07ea`.
