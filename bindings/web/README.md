# @unlocalhosted/pele-rs

Source-built v0.1.0 WASM package for the documented flowchart subset. Build from repository root with
`scripts/build-wasm.sh`, then `npm pack ./bindings/web`. Available as a GitHub release artifact; not published to npm registry.

```js
import { loadPele, PeleError } from '@unlocalhosted/pele-rs';
const pele = await loadPele();
const diagram = pele.render('flowchart LR; A[Hello] --> B[World]');
document.querySelector('#diagram').innerHTML = diagram.svg;
```

Serve over HTTP. Bundlers that relocate WASM can pass its URL or bytes to
`loadPele(wasmURL)`. Rendering is synchronous after initialization. Malformed
input throws `PeleError` with `code` and optional UTF-8 `span`; a WASM trap requires
reloading the page/worker. See the repository README for supported syntax and limits.

Geometry and outlined SVG use the same Rust engine as Swift. No browser fonts,
external URLs or runtime npm dependencies. The engine is MIT; bundled Noto Sans
is SIL OFL 1.1. See included notices. Independent project, not official Pele.

## Development full-library interface

Source builds also provide `@unlocalhosted/pele-rs/full`. All 33 renderers have
pinned Rust/Swift/browser corpus verification. Nine families currently expose
mutable models; Journey, Quadrant and XYChart also expose their actual native
methods. Remaining model/API compatibility is still under development. This does
not change the earlier published v0.1.0 release.

```js
import {loadRenderer} from '@unlocalhosted/pele-rs/full';
const renderer = await loadRenderer();
const model = renderer.parse('pie\n"Swift":2\n"Web":3');
model.sections.set('Rust', 5);
const diagram = renderer.render(model);
renderer.dispose(model);
```

The full interface uses browser font/resource adapters with Rust processing.
See the repository's `docs/MUTABLE_MODELS.md` for ownership, method/error behavior
and the explicit remaining compatibility boundaries.
