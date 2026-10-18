# Native presentation artifact: first Rust slice

Flowchart, Swimlane, AgentFlow, ER and Kanban can produce a version 1
presentation artifact alongside normal Rust rendering. This is the first
interaction/accessibility gate from `LLD_NATIVE_API_COMPLETION.md`; producer
specifics are recorded in `NATIVE_PRESENTATION_PRODUCERS.md`. Other families
report `UnsupportedDiagram`.
There is no C/Swift/WASM artifact transport or native link activation in this slice.

```rust
use pele::compat::{Engine, PresentationBudget, PresentationCapture, RenderOptions};
use pele::compat::text::MetricsResources;
let engine = Engine::with_builtins();
let mut host = MetricsResources;
let mut capture = PresentationCapture::new(&mut host, PresentationBudget::default());
let result = engine.render_source(
    "flowchart LR\nA[Read] --> B[Finish]\nclick A \"https://example.test\"",
    &RenderOptions::default(), &mut capture,
)?;
let presentation = capture.take();
# Ok::<(), pele::compat::PeleError>(())
```

The same wrapper works with retained `Engine::render_document`, exact/bounded
output-policy methods, `PublicDocument::render` and mutable `render_operation`
methods. It forwards text, icons, direct icons, clock and calendar callbacks to
the original host. Ordinary hosts use default no-op hooks and create no metadata
vectors or strings. This is an opt-in Rust extension, not an upstream Pele API.

The artifact owns finite SVG user-space width/height, accessible title (explicit
`accTitle`, falling back to visible diagram title), optional `accDescr`, plain
rendered node/group/edge label text and approved link actions. Label text comes
from laid-out spans, so Markdown markers are not exposed as accessible content.
Icons without text do not acquire invented descriptions. IDs and source
accessibility fields use the renderer's entity decoding.

A link action has its decoded ID, href, internal/external flag and logical
rectangle. External node anchors use the node layout frame; internal text
anchors use the positioned label frame. These are the actual selected layout's
coordinates after title/padding/centering shifts, before scalar SVG serialization
to hundredths. They are **interaction frames**, not exact painted shape/stroke
contours, DOM hit testing, or screen coordinates. A native presenter must apply
its view/camera transform and choose overlap handling. Geometry for supported
shapes comes directly from the renderer; no consumer reparses source or SVG.
Groups currently contribute accessible text but no invented links.

Link capture uses the same allowed-scheme/links gate as actual href emission.
An internal `data-href` without an approved href is deliberately absent even when
legacy `Rendered.links` retains it. `links: false` suppresses all actions.
This slice never opens URLs; a host chooses its own activation policy. Native
link target/window handling and VoiceOver action controls remain later UI gates.

The renderer keeps provisional loose/tight/across/down candidates together with
their SVG. Only the winning candidate reaches the engine render session. The
engine publishes after module success, optional native theme resolution, byte
and UTF-16 output limits and link-ID decoding. Mutable render operations defer
publication again until the final model snapshot succeeds; existing mutation,
revision and typed error behavior remains unchanged. Thus host errors, limit
errors, theme errors and graph export errors leave the caller's last accepted
presentation intact. `take()` explicitly clears that retained outcome; a fresh
capture starts with none. This transaction requires Engine/PublicDocument;
direct `DiagramModule::render` calls are provisional and bypass engine policy.

`PresentationOutcome` distinguishes an available versioned artifact from
`UnsupportedDiagram`, `BudgetExceeded`, `InvalidGeometry`, and `InvalidText`.
Unavailable metadata does not fail an otherwise successful compatible SVG
render and never returns a truncated partial action list. A successful
unsupported render replaces the previous outcome with explicit unavailable.

Default candidate budgets are 4096 combined text/link elements and 1 MiB copied
UTF-8 text/IDs/targets. Requested limits clamp to hard ceilings of 16,384 elements
and 4 MiB per candidate. Strings are checked before metadata copies/decoding;
collection stops and discards the whole frame on exhaustion. Adaptive selection
retains at most three candidate frames simultaneously, each independently
bounded. These are metadata storage/work bounds, not universal peak-allocation
limits for the renderer, user callbacks or trusted extension implementations.
Custom modules opting into the capture hook must respect the requested budget;
the current built-in producers are Flowchart/Swimlane, AgentFlow, ER and Kanban.

`presentation_flowchart` exercises typed and mutable public calls, whole SVG and
host-call equality against ordinary rendering, actual emitted rectangle/anchor
geometry, wide/narrow/turned/lane layouts, a discarded final direction candidate,
accessible data and URL gates, explicit metadata budgets/nonfinite dimensions,
output/theme/measurement failures and recovery, and a downstream codec whose
post-render snapshot fails. Existing Flowchart and Swimlane pinned source/SVG
corpora remain the compatibility check for unchanged ordinary output.
