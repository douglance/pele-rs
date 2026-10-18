# Full flowchart port: development implementation

The public Rust development interface `compat::diagrams::flowchart::parse` now
performs the pinned full flowchart source/model layer. `Engine::with_builtins()`
registers the actual compound/layout/SVG renderer. This is separate from the
released legacy `pele::render` subset.

Pinned upstream: `458322a5b8abb1ae57ddf127c32017ab9fb14a92`. The Rust lexer,
parser, typed ordered model, shape-name catalogue and metadata/URL handling translate
`src/diagrams/flowchart/{lexer,parser,db,types,shapes,graph,render}.ts`, the
shared layered/compound algorithms, shape/edge geometry and YAML/URL behavior.
The extractor records the exact source map and hashes. HTTP URL normalization uses
the Rust `url` crate; source sanitation and scheme gates remain explicit Rust logic.

`cargo test --locked -p pele-rs --test flowchart_compat` passes four public
integration tests. The corpus retains 18,175 scenarios: 8,953 declared models,
8,952 complete SVG renders, 9,222 parse errors and one render-limit error. No
exclusions. It includes pinned docs/specs, official ordered grammar/security
campaigns, shapes/assets/styles/configuration matrices, 1,000 generated graphs
and layout-budget fallbacks. A separate typed stress document contains 47,917
edges; foreign hosts check its model projection through their actual ABI.
`scripts/parity/extract-flowchart.mjs` regenerates expectations from the clean
pinned checkout. Models preserve all fields declared by `FlowchartModel`.

The oracle currently excludes six TypeScript-private enumerable database members:
`subgraphLookup`, `members`, `edgeById`, `pairCount`, `subCount`, `options`.
JavaScript prototype methods and observable extra database members require a
deliberate host-API compatibility decision and verification before the full-library
gate can close. Parser-model evidence is not a full public JavaScript API claim.

Graph normalization uses a reusable `FlowVariant` seam. Actual layered and
compound layout, cluster title placement/halos, shape intersections, edge curves,
markers, icons/images and policy handling stay in Rust. Native palette export also
resolves generated sRGB `color-mix` halos for AppKit. Public API/projection gaps
above remain required; this implementation does not certify full host parity,
performance improvement, GPU acceleration or a new release.
