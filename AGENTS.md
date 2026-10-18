# Contributor rules

- Keep the engine independent of Kamui and other host applications.
- Read `docs/LLD.md` before changing public contracts.
- Preserve deterministic native/WASM output and explicit unsupported-syntax errors.
- Tests must exercise feature slices through public interfaces: integration or end-to-end only. Do not write unit tests.
- Bound parsing, graph expansion, layout, shaping and SVG output work.
- Preserve upstream provenance and all required license notices.
- Commit coherent chunks; do not include generated build artifacts.
- Keep Swift and JavaScript adapters thin. Parsing, geometry and glyph paths belong to Rust.
