# Full flowchart port: parser checkpoint

The public Rust development interface `compat::diagrams::flowchart::parse` now
performs the pinned full flowchart source/model layer. This is separate from the
released legacy `pele::render` subset. No incomplete flowchart renderer is installed
in `Engine::with_builtins()`.

Pinned upstream: `458322a5b8abb1ae57ddf127c32017ab9fb14a92`. The Rust lexer,
parser, typed ordered model, shape-name catalogue and metadata/URL handling translate
`src/diagrams/flowchart/{lexer,parser,db,types,shapes}.ts` and shared YAML/URL behavior.
The extractor records the exact source map and hashes. HTTP URL normalization uses
the Rust `url` crate; source sanitation and scheme gates remain explicit Rust logic.

`cargo test --locked -p pele-rs --test flowchart_compat` passes 4,796 public source
scenarios: 2,521 models and 2,275 errors. `fixtures/pele/flowchart.json` retains
the expected data; `scripts/parity/extract-flowchart.mjs` regenerates it from the
clean pinned checkout. Models preserve all fields declared by `FlowchartModel`.

The oracle currently excludes six TypeScript-private enumerable database members:
`subgraphLookup`, `members`, `edgeById`, `pairCount`, `subCount`, `options`.
JavaScript prototype methods and observable extra database members require a
deliberate host-API compatibility decision and verification before the full-library
gate can close. Parser-model evidence is not a full public JavaScript API claim.

Next required layer: the actual layered/compound algorithms, shared shape and edge
geometry, graph transformation, full SVG rendering and Swift/browser delivery.
Three background agents own layout/rendering, shapes and edges respectively. No
layout, host parity, acceleration or release claim follows from this checkpoint.
