# Native link and accessible presentation

The source-built macOS SDK mounts Rust-produced version 1 metadata alongside the
actual native image. It does not parse SVG/source or derive node positions in
Swift. Flowchart, Swimlane, AgentFlow, ER and Kanban cover every current native
link-emitting renderer. Other metadata producers remain a later gate.

```swift
var options = Pele.MountOptions()
options.onLink = { action in
    // The host decides whether/how to navigate. Pele never opens a URL.
    print(action.id, action.href, action.internal)
}
let mounted = try Pele.mount(view, source: source, options: options)
```

The accepted artifact is exposed as `DiagramView.nativePresentation`.
`presentationGeometryAvailable` indicates whether it can map to the displayed
image. `MountOptions.presentationBudget` controls the existing opt-in Rust
metadata budget independently of SVG output limits. Successful unsupported or
budget-unavailable metadata keeps the actual image and exposes truthful generic
`<type> diagram` accessibility. Legacy link lists from custom providers never
become fabricated hit regions. Custom providers have explicit unsupported
metadata until a native artifact contract is added for them.

The view uses the same image rectangle as native drawing and Zoom. Rust's SVG
user coordinates map linearly onto that rectangle, including camera pan/scale,
image fit, clipping and nested host/window transforms. Only approved Rust action
frames participate. They remain **logical interaction rectangles**, not painted
path/stroke hit contours. The host callback receives the original Rust action
and SVG coordinates, not a re-created URL or estimated layout.

Default SVG decoding preserves this geometry. An arbitrary
`preparePresentation` image does not: its metadata is retained, while geometry,
link actions and diagram-specific accessible content are disabled by default.
A host can explicitly set `preparedImagePreservesSVGGeometry = true` only when
its custom image depicts the complete SVG over its entire image bounds. That
assertion permits full-image scaling, including an explicitly chosen aspect
change; it does not support guessed cropping, translation or re-arranged nodes.
The generic fallback is preferable when that contract cannot be met.

Image, artifact, accessible content and action callback commit together after
successful drawing/preparation. Failed parse/render/output/theme/preparation
and deferred hidden/zero-width updates retain the accepted image and metadata.
The diagnostic augments accessibility help. Retained accessibility children
remain tied to their accepted drawing; after replacement, stale children cannot
activate a new drawing's link at the same index. Callbacks run after commit and
may update/replace the mount. An original callback exception is retained in
`Mounted.unhandledCallbackError` without dimming an otherwise valid image.

The group exposes the artifact's accessible title, rendered plain label text and
description. Approved actions expose native accessibility link elements with
labels, screen-space frames and press actions. Tab/Shift-Tab while the diagram
owns first responder select actions; Return/Space activates the selected action.
At the sequence boundary, keyboard input forwards to the host. Unrecognized and
app-modified keys retain existing Zoom/host handling. Native focus drawing uses
AppKit's focus ring. The accessibility tree also retains native Zoom controls;
actual NSButtonCell labels match the host's Zoom labels rather than SF-symbol
fallback names. Control insertion/removal and visibility refresh that tree.

Ordinary single clicks call the host after a matching press/release over the
same current region. Pointer travel cancels the link candidate, and Zoom's drag
suppression is consumed before activation. A double-click zoom event does not
produce a second link action. As with browser click/double-click ordering, an
already completed first single click is not retroactively undone. Camera-only
input never calls Rust rendering. Window resign, detach and hiding cancel
pending pointer selection. Key-window observation belongs to either live link
actions or Zoom, including links that remain after standalone Zoom destruction.
Controls keep their own native hit targets.

Destroy preserves the last drawing/static accessible content but removes link
and Zoom input ownership. Explicit post-destroy updates do not silently restore
input. A replacement mount owns new actions, and an old controller cannot tear
them down.

`PresentationUITests` exercises real titled, active AppKit windows, actual view
pixels, window-routed mouse/keyboard events and public accessibility press/frame
queries, alongside the existing Mount and Zoom GUI suites. This is native API
and event-path proof, not a claim that physical trackpad gestures, VoiceOver's
spoken output or every assistive-technology traversal was exercised.
