# Web intrinsic exception diagnostics

The native mutable API preserves the pinned Node/V8 intrinsic exception class and
raw UTF-16 message. JavaScript engines generate different prose for the same
invalid operation. Exact cross-engine matching of that prose, stack traces, and
nonstandard Error.line/Error.column is outside this contract. The adapter does
not translate diagnostics, simulate a VM, or fabricate source locations.

This boundary applies only to the seven named invalid mutation vectors below in
public-remaining-er.spec.js and public-remaining-info.spec.js. Each still asserts
an exact native TypeError message against the pinned Node canonical diagnostic,
the live oracle TypeError class, and unchanged complete partial graph, alias,
mutation ordering, and recovery checks. Every other method error comparison stays
strict. Every PeleError retains exact name, message, code, type, line, column, and
snippet comparisons. Valid API values, source parsing, whole SVG output, callbacks,
and model state receive no relaxation.

Each browser test attaches an intrinsic-vm-diagnostics JSON artifact containing
the browser project, family, named vector, method, and both observed raw error
names/messages. Other Error location properties are excluded from comparison;
PeleError locations remain required. The artifact records actual observations,
not a rewritten expected error.

## Measured diagnostic pairs

Oracle: Pele 1.6.0, upstream revision
458322a5b8abb1ae57ddf127c32017ab9fb14a92, bundled pele.js SHA-256
d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
Measurements came from the controlled Chromium/Firefox/WebKit diagnostic run in
/tmp/pele-web-numeric-diagnostic-unx57w85/summary.json, SHA-256
4076afa724cb288f98c3c0cabbe93e91dfd86c1fb9684eff6c338c1a1ff847c0.
These are observed engine messages, not a promise that future engine releases use
identical wording.

| Invalid mutation | Native / pinned Node canonical | Pinned Firefox | Pinned WebKit |
| --- | --- | --- | --- |
| addSection / sections=null | `Cannot read properties of null (reading 'push')` | `can't access property "push", this.sections is null` | `null is not an object (evaluating 'this.sections.push')` |
| addBox / boxes=null | `Cannot read properties of null (reading 'push')` | `can't access property "push", this.boxes is null` | `null is not an object (evaluating 'this.boxes.push')` |
| setCssStyle / cssStyles=null | `Cannot read properties of null (reading 'push')` | `can't access property "push", t is null` | `null is not an object (evaluating 't.push')` |
| setCssClass / readonly array length | `Cannot assign to read only property 'length' of object '[object Array]'` | `can't define array index property past the end of an array with non-writable length` | `Attempted to assign to readonly property.` |
| addRule / byName=null | `Cannot read properties of null (reading 'set')` | `can't access property "set", this.byName is null` | `null is not an object (evaluating 'this.byName.set')` |
| addPersonOrSystem / null label | `Cannot destructure property 'key' of 't' as it is null.` | `t is null` | `Cannot destructure property 'key' from null or undefined value` |
| addSubsetData / subsets=null | `Cannot read properties of null (reading 'push')` | `can't access property "push", this.subsets is null` | `null is not an object (evaluating 'this.subsets.push')` |

The measured packed Sequence codec initially returned TypeError with message
"Sequence value must be an object" for the null boxes case. That was a native
parity defect, corrected before applying this boundary. A separate pinned Node
probe verifies both null and undefined boxes produce the reading-'push' messages,
after currentBox has been published. public_sequence.rs pins those exact native
messages, partial own fields and order, revision publication, and recovery aliases.

The WebKit capture also measured different builtin Error line/column locations:
native errors originated at adapter line 53, column 27, while oracle errors
originated inside its bundled family implementations. These are host source
locations, distinct from PeleError's library-authored diagram positions. The
Firefox Block function own-key ordering defect from that run is also distinct
from diagnostics and is not covered by this boundary.

## JavaScript NaN policy materialization

The browser edge-limit fixture compares graph Number NaN atoms and an expected
NaN maxEdges policy by NaN class. Every finite number, signed zero, infinity,
property descriptor, property/collection order, and alias remains exact. Source
and edge-limit semantics, complete SVG output, dimensions and callbacks are
unchanged. This does not canonicalize any runtime value or alter native packet
transport assertions; supplied WASM numeric packet bits and native C/Swift raw
input bits remain checked exactly.

The controlled Chromium edge-fixture failure measured the same caller Number as
fff8000000000042, retained native policy fff8000000000042, and pinned JavaScript
policy 7ff8000000000000 for flowchart/node/21 (also chain/cartesian and Swimlane).
JavaScript object/JIT materialization can canonicalize that NaN payload. Separate
probe repetitions also observed caller/packet canonicalization before the parser;
therefore a fixed Node payload is not a cross-engine JavaScript object-representation
promise. The full triples are preserved in /tmp/pele-web-edge-diagnostic.log and
/tmp/pele-web-final-diagnostic.json. Other graph numeric values are not normalized.

## Function intrinsic observation order

Constructible native helper functions materialize their intrinsic prototype
before arity is observed. The all-family graph consumer uses the same observation
order for both native and live oracle functions, then compares actual raw own
keys without sorting or dropping entries. This avoids measuring different lazy
property-materialization histories in Firefox. Function arity and complete model
data order, descriptors and identities remain checked. No general data-object
property-order boundary is introduced.
