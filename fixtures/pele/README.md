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

No oracle JavaScript ships in Rust, Swift, or WASM packages.

Pele: Copyright (c) 2026 kepano, MIT; see `third_party/PELE-LICENSE`.
Mermaid documentation examples: Copyright (c) 2014 - 2022 Knut Sveidqvist,
MIT; see `third_party/MERMAID-LICENSE`. Pele extracted those examples at
Mermaid `mermaid@12.1.0`, commit `21f72f07ea`.
