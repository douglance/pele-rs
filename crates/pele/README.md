# pele-rs

Independent Rust diagram engine for native applications and browser WebAssembly.
The 0.2 source candidate implements all 33 diagram families from Pele 1.6.0.
Final API and distribution acceptance is tracked in the repository; source
coverage alone is not a full-parity release claim.

```rust
use pele::compat::{Engine, RenderOptions, text::MetricsResources};

let engine = Engine::with_builtins();
let drawing = engine.render_source(
    "pie\n\"Swift\":2\n\"Web\":3",
    &RenderOptions::default(),
    &mut MetricsResources,
)?;
assert!(drawing.rendered.svg.contains("<svg"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`compat::Engine` exposes typed retained documents and extensible diagram modules.
`compat::public::PublicEngine` adds owned mutable model graphs, schema-described
methods and caller-graph construction. Hosts implement `RenderResources` for
fonts, icons and time. Parsing, geometry and SVG generation stay in Rust.

The original `pele::render` flowchart API remains available for v0.1 consumers.
Its documented subset and outlined font output are separate from the newer
compatibility interface.

Compatibility is pinned to Pele revision
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`. Native transport/work ceilings,
bounded source/config domains and documented JavaScript VM differences remain
explicit. No speedup or GPU acceleration is claimed. See the
[repository](https://github.com/vijayksingh/pele-rs) for Swift/WASM adapters,
source builds, the parity inventory and revision-specific verification.

Independent project; not an official Obsidian, Pele or Mermaid release.
Engine is MIT; bundled Noto Sans is OFL. See LICENSE, NOTICE and the licenses
under `assets/` for upstream and bundled-code attribution.
