# pele-rs

Independent, deterministic Mermaid flowchart engine for Rust, Swift and browser
WebAssembly. Version 0.1.0 covers a documented flowchart subset, not full Mermaid parity.

```rust
let diagram = pele::render("flowchart LR; A[Hello] --> B{Ready?}", &Default::default())?;
assert_eq!(diagram.geometry.nodes.len(), 2);
// diagram.svg contains concrete colors and outlined glyphs; no host fonts needed.
# Ok::<(), pele::Error>(())
```

Supports TB/TD/BT/LR/RL, rectangle/rounded/diamond/circle/stadium, solid/dashed/thick
links, pipe labels, chains, cycles, parallel edges and disconnected nodes.
Subgraphs, styles, callbacks and unsupported text return structured errors.
The repository README and `docs/LLD.md` specify scope, budgets and foreign adapters.

Inspired by Pele at `458322a5b8abb1ae57ddf127c32017ab9fb14a92`.
Engine is MIT; bundled unmodified Noto Sans is OFL. See LICENSE, NOTICE,
`assets/PELE-LICENSE` and `assets/fonts/OFL.txt`.
