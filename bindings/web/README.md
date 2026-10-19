# @unlocalhosted/pele-rs

One independent Rust/WASM engine for all 33 Pele-compatible diagram families.
Build from the repository root with `scripts/build-wasm.sh`, then install the
resulting GitHub-distribution tarball. Not published to the npm registry.

```js
import {loadRenderer} from '@unlocalhosted/pele-rs';
const renderer = await loadRenderer();
const model = renderer.parse('pie\n"Swift":2\n"Web":3');
model.sections.set('Rust', 5);
const drawing = renderer.render(model);
document.querySelector('#diagram').innerHTML = drawing.svg;
renderer.dispose(model);
```

Serve over HTTP. `loadRenderer(wasmURL)` supports relocated WASM assets. Parsing
and rendering are synchronous after initialization. The root and `./full` entries
expose the same full interface; `./core`, `./registry`, `./lazy` and all 33
`./diagrams/*` entries expose source preparation, independent registries and
per-family parsers/renderers. Built-ins share one WASM binary.

`renderer.render(source, options)` avoids exporting an editable model when only
SVG is required. `parse` supplies mutable owned models; dispose them when done.
`renderer.mount` and `enableZoom` supply browser lifecycle and interaction.
Host font metrics, icons and time remain outside the Rust diagram engine.
See [registry contracts](REGISTRY.md) and the repository's mutable-model,
caller-authored construction, resource, transport and work-budget documentation.

Independent project, not an official Obsidian/Pele/Mermaid release. MIT engine;
all upstream, dependency and runtime notices are included. No GPU acceleration
or general performance speedup is implied by using Rust or WASM.
