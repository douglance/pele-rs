# Source-built native presentation

Requires a current local XCFramework and Swift 6/macOS 14+. The published v0.1.0
binary does not expose this development API. Parsing, layout and SVG stay in Rust;
AppKit owns presentation, input and view lifecycle.

```swift
import AppKit
import Pele

@MainActor
func show(in view: Pele.DiagramView) throws -> Pele.Mounted {
    var options = Pele.MountOptions()
    options.zoom = .automatic
    options.onRender = { drawing in print(drawing.type, drawing.width) }
    options.onError = { error in print(error) }
    return try Pele.mount(view, source: "flowchart LR; A --> B", options: options)
}
```

Keep the returned controller for `update`, `refresh`, `invalidateResources` and
`destroy`. The view also retains its current controller. A replacement mount
destroys the previous controller's automatic observation. Destroy stops automatic
refresh and input; explicit updates remain usable.

`Mounted.source` records the attempted source. `result` and the painted image keep
the last successful drawing when parsing, rendering or image preparation fails.
`error` records the failure, and the view dims the last image with a diagnostic.
The next successful update clears it. Size changes reuse the retained model;
hidden attached views defer drawing until visible. Font changes and explicit
resource invalidation render again without parsing.

`onRender` runs after publication. `onError` runs after recording a failure. Their
exceptions propagate from explicit calls; automatic refresh retains callback
exceptions in `unhandledCallbackError`. Reentry before a native operation commits
is rejected with a busy error. Post-publication callbacks may update the view.

An owned registry uses the same lifecycle:

```swift
@MainActor
func showSelected(in view: Pele.DiagramView) async throws -> Pele.Mounted {
    let registry = try Pele.Registry.empty()
    let mounted = try await registry.mountAsync(view, source: "pie\n\"Rust\":3")
    _ = try registry.load("info")
    try mounted.update("info")
    return mounted
}
```

Async mounting selects the initial family before parsing. Later synchronous
updates require explicit selection of another family. The mounted controller
retains the registry; already parsed models remain renderable after registry
close. Cancellation before dispatch leaves selection and the existing view
untouched. AppKit work stays on MainActor. `AsyncEngine` is the separate actor-owned
background rendering API; native selection does not download code.

Zoom exposes `scale`, `fit`, `zoomBy`, `zoomTo`, `reset`, `refresh` and `destroy`.
Mount policies are `.automatic`, `.disabled`, `.always` or `.custom(options)`;
standalone callers can use `Pele.enableZoom(view)`. Mouse pan/double click,
modified wheel, magnification, keyboard and accessible native buttons use one
camera over the image. Zoom never reruns diagram layout. Ordinary scrolling
continues to the enclosing scroll view. Zoom keeps the host view's frame stable.

Native presentation requires finite positive image dimensions and a finite zoom
maximum. Raw engine render APIs retain their IEEE number behavior. A throwing
`preparePresentation` may supply an NSImage before publication; a failure retains
the prior image. The default path resolves a native palette and decodes SVG.

## Verification and remaining gates

The native window suite covers ten mount lifecycle features and nine zoom/input
features, including actual painted pixels, mouse and keyboard focus, controls,
parent scrolling, resize, failure recovery and teardown. It must run in an active
application, since command-line XCTest cannot reliably activate a key window.
Magnification uses a synthetic public event fixture; physical trackpad input has
not been verified. Four separate async/selective mount tests run through ordinary
Swift XCTest.

This closes native mount/zoom lifecycle, not every native presentation requirement.
The current NSImage does not provide interactive diagram link regions, full
diagram VoiceOver content or an approved image-resource loader. These remain
release requirements in [the native completion inventory](LLD_NATIVE_API_COMPLETION.md).
