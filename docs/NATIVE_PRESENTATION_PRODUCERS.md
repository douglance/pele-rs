# Native presentation producers: AgentFlow, ER, Kanban

This batch extends the version 1 Rust artifact from
`NATIVE_PRESENTATION_ARTIFACT.md`; it adds no ABI, Swift geometry, parser or
render policy. The shared engine still commits only after output/theme policy,
and mutable operations additionally wait for successful graph export.

- AgentFlow retains the artifact with each across/down SVG candidate and submits
  only the narrower selected frame. Visible group/node labels, ordinary edge
  labels and border-held edge labels use their rendered span text. External
  node anchors use their actual logical node frame. Internal anchors use the
  positioned label frame, including collapsed-node label offsets. Pinned
  AgentFlow caches its resolved graph at parse time (`render.ts` graphs WeakMap).
  Later semantic node label/link edits do not replace that private graph; the
  artifact follows the actual cached drawing, while live accessibility metadata
  edits still publish. The feature test explicitly preserves this distinction.
- ER similarly retains across/down candidates. Accessible text includes group
  and entity headings, attribute type/name/key/comment cells and relationship
  labels. Internal links cover the entity heading alone: table attributes are
  not falsely made actionable. Header offsets use the actual table/header
  geometry; action IDs match the emitted entity data ID.
- Kanban records section/card/ticket/assignee labels and ticket anchors, after
  the real wrapping and row-folding layout. Ticket rectangles use the actual
  ticket text dimensions, baseline block position and nested column/card
  translations. Whole cards are not falsely made clickable. `title` provides
  the accessible title fallback; Kanban's renderer does not have source
  `accTitle`/`accDescr`, so this batch does not invent them from graph extras.

All action hrefs come from the same approved href emission sites as SVG.
Rejected internal `data-href` entries do not create actionable regions, even
when the legacy `Rendered.links` list retains them. Existing scheme policies,
`links: false`, SVG bytes, IDs, host call order and error behavior stay intact.
No metadata strings/vectors are allocated when capture is disabled. Candidate
budgets and unavailable outcomes remain those of the first artifact; unsupported
families remain explicit unavailable. Regions remain logical layout rectangles,
not painted-path/stroke hit testing, DOM interaction, or a VoiceOver UI proof.
