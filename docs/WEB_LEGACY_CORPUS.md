# Historical browser corpus and mutable models

The historical per-family fixtures pin the immutable typed JSON projection and
complete rendering results. The web mutable API also exposes the actual returned
model's private own fields, aliases, and method-driven changes. These are separate
public contracts: a typed JSON projection is not a substitute for the returned
mutable object.

For Gantt, Sequence, C4, Requirement, Block, Railroad, and Venn, the browser corpus
checks the historical model through the existing native `parse_v2().model()` API.
It separately parses the real mutable model and retains every complete mutable
render/source-render comparison. The new mutable feature tests and all-family
numeric tests preserve their complete graph, own-property order, descriptor,
alias, method, and recovery assertions. No mutable fields are stripped to make the
historical projection match.

Measured examples include Gantt scheduling its live task records during render,
Sequence's pending fields and active Map, C4's lookup/current aliases, Requirement's
pending builder fields, Block's private counters/caches, Railroad's `byName` Map,
and Venn's known/current/indent state. A controlled probe confirmed the immutable
projection still exactly matches each historical fixture while the actual mutable
models expose these additional states.

## One independent native output boundary

The existing Class vector `stress/namespace name of 12,000 parts` has source
SHA-256 `046702029bd9c44c84e60976877b377f08f77f0f4a874cde27c3985b9b232bc5`.
Its 24,036 source characters expand to more than 64 MiB of proven emitted IDs.
The exact output lane's independent native byte guard rejects that expansion
before allocating the complete SVG.

The browser integration asserts both envelopes separately:

- Pinned Pele semantic result: `PeleError`, code `limit`, type `class`, message
  `Diagram output is longer than the limit of 4000000 characters.`
- Native byte-budget result: `PeleError`, code `limit`, type `class`, message
  `Native render output exceeds byte budget`.

Both retain line 0, column 0, and an empty snippet. The fixture's original pinned
expectation is preserved. Only this SHA-checked vector receives the explicit byte
boundary assertion. The existing two mutable graph transport boundaries remain
independently checked, as do every other source and render error. No budget is
raised and the oversized SVG is not allocated for this proof.

Oracle: Pele 1.6.0 at upstream
`458322a5b8abb1ae57ddf127c32017ab9fb14a92`. The actual browser probe is retained at
`/tmp/pele-web-legacy-probe.json`; full-suite logs record both boundary counts.
