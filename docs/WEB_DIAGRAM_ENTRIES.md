# Low-level built-in diagram entries

Each of Pele's 33 `./diagrams/<type>` default exports is available as a native
Rust-backed Diagram object. Initialize the WASM host before calling its synchronous
methods:

```js
import pie from '@unlocalhosted/pele-rs/diagrams/pie';
import {loadDiagramEntries, disposeDiagramModel} from '@unlocalhosted/pele-rs/diagrams';
await loadDiagramEntries();
const model = pie.parse('pie\n"A":1\n', {}, undefined, 0, {maxEdges: undefined});
model.sections.set('B', 2);
const drawing = pie.render(model, {}, {fontSize: 13.25});
disposeDiagramModel(model);
```

`parse` receives already-prepared source verbatim. It does not preprocess YAML,
encode entities, append a newline, detect a diagram type or apply the core source
character limit. Configuration, title, line offset and raw maxEdges are passed to
the selected native module. In particular, raw `maxEdges: undefined` is distinct
from core's default 5,000. Existing independent native source/edge budgets remain.

`render` invokes the retained native module with the supplied configuration. It
returns raw `{svg,width,height,links}` without a core `type` field, responsive
default, semantic outputLimit, native theme conversion or link-ID decoding.
Inline icons follow the pinned raw entry behavior; direct renderer icon lookups
still invoke the supplied host. The independent 64 MiB returned-SVG byte budget
remains. Render mutations and partial failures publish back into the original
live graph before an original host exception is rethrown.

Entry objects can be registered in an owned registry. Full and lazy renderers use
the same canonical objects as the per-family imports. Unmodified entries retain
the native source path; later edits to a retained entry's render handler or
section are observed through the existing live-provider boundary. Replacing the
registry does not retarget old models, and retained models survive registry close.
The original effective configuration retains its identity after a live handler
observes it. Nested raw calls inherit only the outer core's host icon context;
standalone raw calls establish no global inline resolver. No source reparsing or
JavaScript diagram renderer is introduced.

A model returned by a low-level entry has separate provenance from
`renderer.parse` and cannot be passed to core `renderer.render`. A low-level
renderer can also draw a native model of the same family returned by core parse;
the model's actual owner supplies the retained Rust renderer.

This verified layer accepts models returned by the same entry, including direct
field edits and native prototype methods. Arbitrary newly constructed plain Model
objects require explicit per-codec constructors and are not implemented here.
Prepared line offsets currently support finite nonnegative integers in the native
source-position range. Malformed UTF-16 strings and fractional/nonfinite offsets
are outside this layer. Configuration transport retains the existing well-formed
JSON object contract.

The pinned oracle is Pele 1.6.0, upstream revision
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`. The fixture records all 33 entry metadata,
99 prepared/context/edited model vectors, whole SVGs, dimension bits, host call
order, complete model graphs and source/artifact hashes. Its generator is a
development oracle only; no upstream JavaScript parser or renderer is distributed.
