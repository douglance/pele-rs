# Live Gantt getTasks callbacks

A caller may replace a retained Gantt model's declared `getTasks` method with an
ordinary JavaScript function. The renderer calls that exact function with the
original receiver and captured numeric render time. Capturing the original
method and invoking `original.call(this, now)` continues to run the Rust
scheduler against the same arena. Returned arrays, Date aliases, partial
mutations, thrown values and subsequent recovery survive the operation. The raw
Gantt entry uses the same callback seam while retaining its raw render policies.

The callback and model stay owned by their document. Closing the registry does
not invalidate them; disposing the document does. Rendering or disposing the
same document during its active native operation rejects with TypeError.
Captured native calls use a scoped wasm-bindgen closure, which cannot outlive the
active arena. Recursive entry into that borrowed native call is rejected.

JavaScript objects remain ordinary caller objects. At an opted-in callback
boundary the bridge scans their own data records, compares flat records and
sends only changed records plus appended UTF-16 strings/object identities. The
Rust arena validates the delta, updates its indexes, and journals native changes.
Captured native calls return only those changed records. There is no full graph
serialization on every callback and no JavaScript scheduler or renderer.
Ordinary operations retain their existing full snapshot/exchange contract.

The synchronization journal is absent for native callers that do not request it.
All synchronization uses the existing node/string/reference/work ceilings;
individual callback packets are capped at 64 MiB and calls at one million per
active override. These bound transport and work, not universal peak allocation.
The flat transport preserves cycles, lone UTF-16 units and raw numeric atoms.
This does not make active lone-surrogate drawing/calendar inputs supported.

A captured `getTasks` call with an omitted or nonfinite argument reads the direct
host clock. It must not reuse the render session's explicitly supplied or cached
clock. The additive `direct_now_millis` capability provides this distinction;
ordinary rendering and foreign provider context clocks keep their existing
session policy. Host clock exceptions can be caught inside the override, with
exact JavaScript exception identity, before retrying the native scheduler.

This layer covers the declared getTasks override, ordinary mutations before and
after captured native calls, and partial failure publication. It does not promise
arbitrary other method overrides, accessor/prototype edits, or live mutation
from a nested resource callback while the native arena is borrowed. The separate
fresh-model constructor gate and missing `isInvalidDate` check remain tracked in
their own slice.

`tests/web/host-functions.spec.js` compares complete drawings, callback traces,
state and aliases against pinned Pele 1.6, including alternate task arrays,
captured render flags, repeated delegation, partial throws, output-limit failure,
clock-error recovery, registry/document ownership, and raw entry exceptions.
