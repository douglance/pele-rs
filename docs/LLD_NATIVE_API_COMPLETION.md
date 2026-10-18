# Native and Swift API completion

Target: Pele 1.6.0, upstream `458322a5b8abb1ae57ddf127c32017ab9fb14a92`.
Inventory inspected 2026-10-19. Registry v8 is committed; render-number v9 is committed with ARM64 acceptance; universal and browser
verification continues. This document defines remaining work, not a
claim that the current SDK has complete native or Swift parity.

## Authoritative surface

Upstream `package.json` exposes the full entry, `core`, `lazy`, and per-diagram
entries. `src/core.ts` exports register, registered, detectType, supports, parse,
render, mount, forgetTextWidths, enableZoom and PeleError, plus their public types.
`src/lazy.ts` adds load, renderAsync and mountAsync. `src/index.ts` registers every
built-in and reexports core and model types. Per-diagram entries export Diagram
objects, not model constructors. `src/models.ts` reexports 33 model types.

Native host equivalents remain required by FULL_PARITY_LLD sections 9/12 and
PELE_PARITY HOST-04. Swift does not need to impersonate HTMLElement/Node or a JS
Promise, but a string-returning SDK cannot satisfy mount/zoom's lifecycle contract.
Newer mutable-model and native-registry LLDs supersede the older frozen-projection
description in FULL_PARITY_LLD; immutable APIs remain supported alongside them.

## Inventory and remaining evidence

| Surface | Current native counterpart | Remaining boundary or work |
| --- | --- | --- |
| Full/core registration | Rust Engine/PublicEngine; Swift Registry.empty/allBuiltins, registerBuiltin, registered, capabilities | Built-in selection exists; Swift-authored custom Diagram providers do not. |
| detectType / supports | Registry v8 preprocesses independently of membership and parse limits; supports catches preprocessing failure | Existing native byte/UTF-8 guards remain explicit differences. |
| parse | Immutable Document; mutable Registry/Engine parseMutable; exact ParseLimits | Immutable projection is a convenience subset. Exact foreign source/config value domains remain incomplete. |
| render(source or parsed model) | Engine source/immutable overloads and MutableDocument.render/renderOperation | Exact thresholds compose through parsed documents. v9 adds exact output overloads to immutable/source calls. Legacy integer/Class ordering remains intentionally available. |
| Mutable declared model fields/methods | PublicGraph snapshot/apply plus schema-described invoke; owned native models preserve identities and partial state | Generic native representation is valid, but ordinary foreign method replacements and callback-visible mutation/reentry are not fully implemented. Gantt.getTasks is a declared example, not merely exotic metaprogramming. |
| Result/error types | type/svg/width/height/links and structured Pele/JS/host errors | v9 supplies bounded outer errors and IEEE dimensions. Earlier error-envelope budgets are a separate compatibility backlog. |
| TextMeasurer / IconResolver | HostResources throwing width/icon callbacks; TrustedSVG boundary | Strings remain UTF-8. Host exception identity is supported; arbitrary model function dispatch is separate. |
| fontFamily/fontFamilyMono/idPrefix | Existing Options strings | Ill-formed UTF-16 cannot traverse String/C UTF-8 lanes. |
| responsive/autoDirection/links/images | Existing Options booleans | Declared ordinary values are represented; runtime coercion outside declared types is separate. |
| fontSize/maxWidth/directionBreakpoint/padding | v9 raw-bit numeric packet, existing Double? properties | v9 integration must pass; presence/transport proof is not universal numeric-domain renderer proof. |
| now: number or Date | v9 raw numeric clock input, finite fallback; setNow(date:) TimeClip convenience | Fractional number and clipped Date differ intentionally. Arbitrary nonfinite host clock replacement is not covered by the finite-host clock proof. |
| limit/maxEdges/outputLimit | SourceLimit/EdgeLimit/OutputLimit and ParseLimits | Keep legacy integers and exact lanes distinguishable; no silent f64 conversion of legacy policy. |
| linkSchemes/imageSchemes/linkRel | Existing Options and Rust filtering | Native display still needs approved resource loading and actual link interaction. |
| config | ConfigValue plus Rust Object; YAML/init preprocessing | Host JSON encoding rejects nonfinite numbers; shared-reference/config mutation semantics are not fully represented. Source YAML support does not close host-config transport. |
| load / renderAsync | New native lazy/async slice below | Native built-ins are already linked; selection is lazy, binary download/code splitting is not. |
| mount / mountAsync | Source-built AppKit DiagramView/Mounted; ARM window lifecycle/pixels verified | mountAsync, zoom, links and resource matching remain mandatory. |
| enableZoom / Zoom | No shipped native counterpart | Mandatory native fit/pan/zoom/actions/focus/controls/localized labels and cleanup. |
| forgetTextWidths | Rust makes width calls through resources; no shared cross-render width cache | Define invalidation in the native presentation/font-resource owner; do not invent a cache or claim a no-op invalidates host caches. |
| Distribution | Local universal SDK build and AppKit smoke entry exist | Current root package can fall back to older v0.1 binary and exclude new APIs. Final release must publish matching artifact/checksum/header/Swift source and prove fresh external consumption. |

Relevant current sources: compat/engine.rs, compat/public/engine.rs,
crates/pele-ffi/{src/public.rs,src/registry.rs,src/render_numbers.rs},
bindings/swift/Sources/Pele/{Engine,Registry,MutableDocument,HostResources,
RenderNumbers,PublicGraph}.swift. The AppKit smoke entry decodes all families but
is not an interactive view/controller proof. Existing README family counts are
historical and need reconciliation at release, not optimistic replacement now.

## Next coherent source slice: native lazy selection and async owner

Add `Registry.load(_ type: String?) throws -> Bool`. An open registry returns false
for nil or an unknown exact ID. If already registered, return true without replacing
the handler. Otherwise select a known built-in using native capabilities and call
registerBuiltin once. Do not duplicate the 33-ID table in Swift. Closed-owner and
native transport guards still throw. No unregister or loader-provider abstraction.

Add `Pele.AsyncEngine`, an actor which constructs and exclusively owns an empty
Registry. Expose load, registered, capabilities, renderAsync and close. Registry
and MutableDocument never leave the actor; no unchecked Sendable conformance.
Rendering takes Sendable source/options/resources and returns FullRenderedDiagram.

```swift
public actor AsyncEngine {
    public init() throws
    public func load(_ type: String?) async throws -> Bool
    public func registered(_ type: String) throws -> Bool
    public func capabilities() throws -> [DiagramCapability]
    public func renderAsync(
        _ source: String,
        limits: ParseLimits = .init(),
        outputLimit: OutputLimit? = nil,
        options: Engine.Options = .init(),
        resources: HostResources = .init()
    ) async throws -> FullRenderedDiagram
    public func close() throws
}
```

Order follows lazy.ts: attempt detection, catch its failure, load the detected type
or nil, then parse/render to produce the authoritative error. A recognized source
over the semantic source limit therefore registers its family before parse fails.
Malformed front matter does not select a family. Native transport ceilings remain.

Construct ParseOptions from the supplied render options' parse fields, then call
the owned registry's parseMutable with exact ParseLimits. Render that retained
document and close it on every path. Its parsed config/handler remains authoritative.
The new facade always uses exact Pele policies. Resolve each field independently:
`limits.source ?? options.limit.map { .number(Double($0)) } ?? .undefined`,
`limits.maxEdges ?? options.maxEdges.map { .number(Double($0)) } ?? .undefined`,
and `outputLimit ?? options.outputLimit.map { .number(Double($0)) } ?? .undefined`.
An explicit policy therefore overrides only its corresponding integer option.
Swift integer fallbacks become JavaScript numbers with normal Int-to-Double
rounding; callers needing exact IEEE values use the explicit policy enums.
Pass both resolved parse policies to parseMutable and always use the exact
post-render OutputLimit overload. Negative integer options reach Pele semantic
comparisons, and Class callbacks run before output rejection. Preserve config.
Existing synchronous Engine and C legacy policies remain unchanged.

Actor serialization makes concurrent requests share registration state. Built-in
installation is synchronous; there is no suspended module fetch to deduplicate,
no fake Promise cache, and no native download-retry claim. Failed parse/render
does not remove successful registration. A later valid call can recover.

Cancellation is a native extension: check cancellation before beginning load or
render. Once synchronous native dispatch starts, let it finish and return its
result/error; do not free a live document or claim mid-render cancellation. Close
is idempotent and must remain usable from a cancelled cleanup task. Resources run
synchronously on the actor's executor; UI callbacks must arrange their own MainActor
work without synchronously waiting for this same engine.

Acceptance: all canonical IDs selectable, unknown/nil/case variants; concurrent
repeated selection; independent engines; real source rendering versus synchronous
Engine; parse-before-render errors; detect/load-before-limit behavior; malformed
front matter; exact limits/numeric/resource composition; original callback error
and recovery; cancellation before dispatch; results survive actor closure.
Integration tests exercise public APIs, not actor implementation details. New
AsyncEngine.swift must join the old-binary exclusion list until a matching SDK is
published. No C ABI change is needed for this slice.

## Required native presentation layers

Build these in order, one coherent feature at a time. They remain mandatory gates,
not optional polish after reporting native completion.

1. **Presentation ownership and lifecycle.** Introduce an AppKit `Pele.DiagramView`
   plus MainActor `Pele.Mounted` controller with source, result, error, zoom,
   update/refresh/destroy. The first slice may use the already exercised NSImage
   SVG decoder; actual all-family drawing must be tested in an NSWindow. Keep
   drawing/parsing in Rust. A WebView or JavaScript renderer is not a substitute.
2. **Dependencies and callbacks.** Track viewport width, explicit theme, font
   identity/generation, window attachment and hidden state. Retain parsed and
   last-drawn documents independently. Source/parse-policy/config changes reparse;
   width/font changes rerender retained models. Updates to zero-width attached views defer;
   detached/reattached behavior follows an explicitly tested native mapping.
   Refresh must not chase its own intrinsic width or emit onRender when nothing
   changed. Moving windows rebinds observations. Destroy removes every observer
   and pending refresh but leaves the last drawing visible. A new mount replaces
   the old controller for that view.
3. **Viewport/zoom.** Add native ZoomOptions (always, maxScale, controls, labels)
   and Zoom scale/fit/zoomBy/zoomTo/reset/refresh/destroy. Port pinned fit/clamp and
   anchor rules; use AppKit events for magnify, modified wheel, drag, double click,
   keyboard and focus. Bounds use the drawing's natural size and visible box;
   zoom never changes neighboring layout. Default maxScale is 3. Viewport math may
   be shared Rust, while platform event ownership remains Swift. Native drawing
   rejects unusable nonfinite presentation dimensions explicitly; raw engine
   rendering still returns its exact numeric result.
4. **Interaction and accessible content.** Current LinkInfo has approved URLs and
   IDs but no hit geometry, and NSImage is not an interactive SVG document. Do not
   claim clickable node/link parity from an NSImage or source-text fallback. Add a
   versioned native presentation artifact from Rust with accessible title/text,
   approved link IDs/targets and regions in SVG coordinates. Collect regions at
   existing link emission sites/shared layout seams, beginning with one real
   Flowchart link fixture; reuse the engine's positions rather than reparsing
   source or estimating node layout in Swift. Extend every link-producing family
   before closing this gate. Native host receives activation actions and decides
   whether to open an external URL or navigate internally. It never auto-opens.
5. **Resources and cache invalidation.** Native presentation supplies fonts and
   measurement consistent with the actual SVG decoder, explicit image/resource
   resolution with no implicit network fetch, and appearance changes. If it owns
   text-width caches, expose forgetTextWidths/invalidateFonts there and advance a
   generation that retained controllers observe. Verify changed fonts really
   change geometry and pixels, not merely callback counts.

Mounted callback mapping is concrete: onRender runs after the new drawing and
result commit; if it throws the drawing stays and the exception propagates.
Parse/render/preparation failure retains and dims the last good drawing, records
the attempted source/options and calls onError. A native `preparePresentation`
callback replaces DOM sanitize: it produces a typed native drawing artifact before
commit, and failure leaves the prior artifact. It does not reimplement parsing or
geometry. Use source's accTitle/accDescr/accessibility information where provided;
native VoiceOver and actionable link controls need real UI proof.

Keep the synchronous native mount/update contract explicit; an optional async
mount selects the required family before invoking the same mount path. Do not
quietly make a synchronous method return before its result/error state is ready.
If background presentation is added later, use an owned render session and
generation checks; never send a non-Sendable document between executors.

Acceptance is derived from pinned tests/mount.test.ts and tests/zoom.test.ts:
narrow/wide transitions; hidden/show and detach/reattach; parse-once refresh;
invalid update followed by resize and recovery; font change; callback throws
before/after commit; multiple views and ID isolation; window move; teardown;
zoom/focus/controls and link activation. Run them through a standalone native
window fixture, not only a command-line renderer. `mountAsync` composes loading
with that exact lifecycle; changing family on later update does not silently
register an unrequested family unless its documented API explicitly loads it.

## First native presentation slice: lifecycle contract

This section fixes the state machine before AppKit implementation. Source evidence
is pinned `src/mount.ts` lines 129–425, `src/window.ts`, and
`tests/mount.test.ts` lines 81–158, 241–258, 446–556 and 580–738. It is a native
mapping of the declared mount contract, not a claim that NSImage implements DOM,
SVG interaction, CSS inheritance or font loading.

The minimal public shape is MainActor-isolated. A synchronous method returns only
after its state and drawing have settled; ordinary parse/render/preparation errors
are recorded and reported, while a host notification callback may throw.

```swift
@MainActor public final class DiagramView: NSView { /* native drawing and error overlay */ }
@MainActor public struct MountOptions {
    public var render: Engine.Options
    public var limits: ParseLimits
    public var outputLimit: OutputLimit?
    public var resources: HostResources
    public var preparePresentation: ((FullRenderedDiagram) throws -> NSImage)?
    public var onRender: ((FullRenderedDiagram) throws -> Void)?
    public var onError: ((any Error) throws -> Void)?
}
@MainActor public static func mount(
    _ view: DiagramView, source: String, options: MountOptions = .init()
) throws -> Mounted
@MainActor public final class Mounted {
    public var source: String { get }
    public var result: FullRenderedDiagram? { get }
    public var error: (any Error)? { get }
    public func update(_ source: String, options: MountOptions? = nil) throws
    public func refresh() throws
    public func destroy()
}
```

This is an interface sketch, not a second implementation. Zoom joins this same
handle in the next slice; do not publish a dummy zoom object. Nil update options
reuse the previous options; supplied options replace them. Use the same exact
policy resolution as the new AsyncEngine, leaving existing Engine APIs alone.
The first preparation hook returns an NSImage for the actual result before any
visible replacement. Default preparation uses the existing SVG-to-NSImage path.
Failure to decode or to obtain usable finite presentation dimensions is a named
native presentation error; engine output remains available through raw APIs.

The view owns the installed controller, and the controller holds the view weakly.
An externally retained controller can outlive a view without retaining its window.
Every native document remains confined to MainActor. Keep these independent:
latest attempted source/options; latest successfully parsed document (`model`);
last document whose drawing committed (`drawnModel`); committed result and image;
current/error; natural-width drawing and adaptation/width/font cache; stale;
observation ownership. Documents may alias. Do not clone or reparse them on resize,
and do not discard successful model mutations when render or preparation fails.

Parse failure leaves `model` alone. Draw/preparation failure restores display
caches and selects `drawnModel` for later refresh; the failing model's partial
state is not a transaction rollback. A fresh success commits result/image and
`drawnModel` before notification. Publish errors before onError. Close each native
document only when no retained role or active operation needs it.

The fit sequence matters to callbacks and mutable models: render natural width
with maxWidth absent; if positive available width is smaller, probe maxWidth=1
once to learn whether SVG changes, then render at available width if it adapts.
Cache natural output and adaptation between width-only refreshes. Use explicit
maxWidth when present; otherwise the view's content width. Do not redraw because
the view merely followed the drawing's own width. A quiet refresh with equal SVG
keeps the same image and skips preparation/onRender. Fresh explicit update still
prepares and notifies. Per-mount idPrefix stays stable; explicit prefixes win.

| Operation and trigger | State and drawing after operation | Callback and propagation contract |
| --- | --- | --- |
| Initial visible mount succeeds | Parse once; fit; prepare; commit; result and source agree | onRender sees the committed result/image; then observation begins |
| Initial attached width is zero | **Still render natural width immediately.** This is asserted at mount.test.ts152; there is no initial waits check | Initial onRender runs; a later positive-width refresh fits the retained model |
| Initial detached mount or detached update | Draw a best-effort natural result with current resources; mark stale for attachment | onRender runs; refresh while detached is quiet; reattachment redraws with current dependencies |
| Initial parse/render/preparation fails | No drawing/result; record source/options/error; show complete diagnostic | onError receives the original error; return handle unless onError itself throws |
| Visible update succeeds | Store attempted source/options first; reparse only when required; commit new model/result/image; clear old error | onRender runs after publication; nil options reuse prior callbacks |
| Update parse fails | Attempted source/options stand; preserve latest successfully parsed model and last drawing; current=false; show error and dim drawing | New options' onError receives error; update returns normally unless callback throws |
| Update render/preparation fails before replacement | Last drawing/result stand; restore fit caches; model becomes drawnModel; current=false; attempted source/options/error stand | onError receives the original failure; the engine's completed mutations are not reversed |
| Attached zero-width update parses successfully | Keep prior image/result; retain newly parsed model; mark stale; do not clear an existing displayed error yet | No render/preparation/onRender until shown; parse errors still report immediately |
| Visible A, hide, valid B, syntax-invalid C, show | Show B, not A: B is latest parsed model. source remains C and C's error stays over B | Refresh may notify onRender for B while error still refers to C; pinned test617 exercises this distinction |
| Deferred parsed B fails when shown | Keep A's image/result; model falls back to drawnModel A; current=false; stale=false | onError once for B; unchanged refresh must not retry B repeatedly |
| Resize/font refresh while latest source is invalid | Rerender retained model with latest options/resources; preserve the failed source's existing error | A successful refresh may call onRender; failure calls onError with the refresh failure but does not replace the existing source error |
| Explicit update's onRender throws | Newly committed source/model/result/image stay; error remains cleared when current | Propagate that exact callback error; do not call onError or restore old drawing |
| Initial mount's onRender throws | New drawing stays; no handle is returned and observation is not installed | Propagate original callback error; do not classify it as a render failure |
| Refresh's onRender throws | New drawing/result already stand; if current, record callback failure and dim it; otherwise preserve prior source error | Refresh catches it and calls onError; explicit refresh only throws if onError throws |
| onError throws | Already published render/source error and last drawing remain | Explicit mount/update/refresh propagates the callback error itself; error property keeps the underlying failure |
| Unchanged, hidden or detached refresh | No parse/render/preparation/replacement | No callbacks; an observer may still rebind to a new window |
| Destroy / replacement mount | Stop resize/font/window observations and pending automatic refresh; leave image, source, result and error | Idempotent; replacement owns future events; old callbacks must not receive them |

Initial native preparation does not discover a new CSS font from the SVG child.
That platform feature remains in the resource/font layer. When added, preserve
pinned second-pass semantics: if a post-insertion font correction fails during
render or preparation, keep the first drawing and its fit caches, suppress that
second failure, and report the first successful result once. Do not discard a
visible good first result.

Reentry is phase-specific. MainActor does not prevent synchronous callback reentry.
After commit, onRender may call update; after error publication, onError may recover
with update. The nested operation runs synchronously and its final state wins;
outer code must not reassign result/error/model afterward. A notification callback
may destroy the handle: stop observation immediately, retain any active document,
and allow the in-progress synchronous operation to finish without restarting
observation. Tests must cover both cases with actual callback identity.

Pinned destroy stops watching; it does **not** guard explicit update. Preserve that
bounded behavior: refresh after destroy is a no-op, while explicit update can still
change the retained drawing and notify without reviving observers. Do not close
all documents in destroy. Close them when their roles are replaced or the handle
is released. Replacement mount must also prevent the old controller from
reattaching its automatic observation.

Pre-commit reentry from a native measurement/preparation callback is different:
recursive rendering of the same live native document is rejected by its existing
ownership guard. The first slice should reject update/refresh during that phase
with a stable native busy error before changing source/options, rather than
silently queueing it or releasing a borrowed C handle. Clear that guard before
onRender/onError. Arbitrary sanitizer-induced DOM reentry is not in pinned public
tests; record this native restriction instead of claiming universal JS behavior.
Destroy is always allowed and never frees an actively borrowed document.

AppKit automatic refresh has no throwing caller. Preserve callback exceptions at
that boundary: explicit refresh remains throwing; the observer wrapper must retain
its thrown callback error for host inspection (for example a read-only
`unhandledCallbackError`) and must not route an onError failure recursively back
through onError. Final naming belongs to the implementation review. Silent `try?`
that loses the original callback error does not close callback parity.

For existing Swift value options, compare source, resolved source/edge policy and
config contents to decide whether to parse. Nil options reuse identity naturally;
refresh never parses. JavaScript config-reference identity is not represented by
the current value dictionary; it remains in the explicitly tracked shared-config
layer. Do not claim a structural comparison implements arbitrary JS object
identity, or add an unrelated whole-model copy to approximate it.

Acceptance is one native-window feature suite: initial natural/narrow/zero-width;
width-only fitting and no-op refresh; parse failure and recovery; render failure;
preparation failure; callback throws at each publication boundary; hidden B then
invalid C; failed deferred render; detached/reattached; two views with stable unique
IDs; replacement/teardown; post-commit reentrant recovery; destroy during callback.
Compare actual NSImage-backed pixels before/after failure and recovery, plus public
source/result/error and callback traces. Include one Gantt retained-state path so
resize cannot silently reparse away render mutations. Run all-family image decode
once against the same SDK, not on every lifecycle edit. Zoom, link geometry,
resource matching and VoiceOver remain subsequent mandatory gates.

## Next native slice: viewport zoom and input ownership

Grounding: pinned `src/zoom.ts` exposes ZoomOptions, Zoom, enableZoom and the
internal mount integration `zoomWith(..., observe: false)`. Its public tests cover
frame/clamp math, passive wheel ownership and focus. The mouse/pinch/keyboard and
control rules below come directly from the implementation and require native
integration proof; passing the smaller upstream test file is insufficient.

Add `Zoom.swift` and one actual-window integration suite. Keep camera math and
platform input in Swift; diagram parsing, labels, graph layout and geometry stay
in Rust. Zoom never asks the engine to redraw a diagram or reparses SVG. Draw the
committed NSImage through a view transform and clip it to the presentation area.
Do not resize the image bitmap, rerender Rust or change sibling layout on each
input event. The default SVG-backed NSImage can draw at the requested scale.

```swift
extension Pele {
    public struct ZoomLabels: Sendable, Equatable {
        public var zoomIn: String?; public var zoomOut: String?; public var reset: String?
    }
    public struct ZoomOptions: Sendable, Equatable {
        public var always = false
        public var maxScale = 3.0
        public var controls = true
        public var labels = ZoomLabels()
    }
    public enum ZoomPolicy: Sendable, Equatable {
        case automatic, disabled, always, custom(ZoomOptions)
    }
    @MainActor public static func enableZoom(
        _ view: DiagramView, options: ZoomOptions = .init()
    ) throws -> Zoom
    @MainActor public final class Zoom {
        public var scale: Double { get }
        public var fit: Double { get }
        public func zoomBy(_ factor: Double)
        public func zoomTo(_ scale: Double)
        public func reset()
        public func refresh()
        public func destroy()
    }
}
```

Provide ordinary public initializers for these option types. MountOptions gains
`zoom: ZoomPolicy = .automatic`; Mounted gains read-only `zoom: Zoom?`. Automatic
maps to upstream omitted/'auto', disabled to false, always to true, custom to the
object form. Native option values use value equality for replacement. The
standalone enableZoom uses the same controller and camera as mounted zoom.
Exactly one current zoom owns a DiagramView; replacing it destroys the previous
owner, restoring rest presentation before installing the next. Neither Zoom nor
DiagramView becomes Sendable. The view retains its current zoom; the controller
holds the view weakly. No global event monitor and no window-wide hotkey capture.

The committed image's logical size is `natural`. With default preparation it
matches result width/height; an explicit preparePresentation may supply a different
native presentation size, just as sanitize can replace the displayed SVG. `rest`
is the image rectangle actually drawn when unzoomed: today that is top-left,
width=min(view width,image width), proportional height. `fit=rest.width/natural.width`
when both widths are positive, otherwise 1. Do not change this into a second
height-based fit algorithm. A host's explicit bounds may clip a resting drawing;
zoom does not rewrite host constraints. Rest layout stays separate from camera.

At rest, set `box=(max(rest.width,view.width), max(rest.height,view.height))`.
If neither box dimension exceeds rest by more than 1 point, use rest as box.
A fixed panel larger than the image therefore gives enlarged content that extra
room. Save the rest rectangle; while enlarged, it does not follow the zoomed
image's apparent dimensions. Native insets, if introduced, affect rest only;
zoomed presentation reaches the full box like pinned negative padding margins.
For the first slice DiagramView has no added content padding.

Keep an explicit resting flag, x/y drawing-coordinate origin, active scale and
natural/rest/box sizes. Public scale returns fit when resting. This avoids treating
an ordinary zero-valued camera request as a valid active division denominator.
Zoomable requires an image, positive natural width and `(always || fit < 0.999)`.
Maximum is `max(fit,maxScale)`. A fitted image with always=false ignores zoom input.

The primitive is the pinned frame function. For each axis,
`viewExtent=boxExtent/scale`. If viewExtent>=naturalExtent, clamp origin to
`[naturalExtent-viewExtent,0]`; otherwise clamp to `[0,naturalExtent-viewExtent]`.
Negative origins in a roomy panel are intentional. They let a smaller image remain
whole anywhere inside the larger viewport. Do not always clamp x/y to nonnegative.
No image coordinate is inferred from diagram node geometry.

For anchored zoom, from=activeScale or fit, to=clamp(request,fit,maximum). If
to<=fit*1.001, reset. Otherwise convert the event location into the view's flipped
coordinates. Preserve `point=(oldOrigin or 0)+(anchor-oldRect.origin)/from`, then
set `newOrigin=point-(anchor-newRect.origin)/to` and apply the frame clamp. Without
an anchor use the previous drawn rectangle's center. Opening a roomy viewport may
change the drawn rectangle's size/origin; account for both rectangles. Panning by
screen delta `(dx,dy)` sets origin to `(x-dx/scale,y-dy/scale)` then clamps.
A graphics transform implements `screen=(drawing-origin)*scale` within the box;
use the same transform for future link hit regions, not a second approximation.

| Input/action | Native mapping and exact controller rule |
| --- | --- |
| zoomBy/zoomTo | Multiply public scale or use requested scale; anchored show at prior rectangle center; clamp to fit/maximum |
| Reset | Restore rest drawing/fit, zero active origin, release drag/pinch state, refresh controls; no engine render |
| Plus/minus controls | Multiply/divide scale by 1.25 squared (1.5625), not keyboard's 1.25 |
| Modified wheel | Handle only Ctrl or Command while zoomable; use native scrollingDeltaY converted to the positive-down wheel convention, `turn=clamp(-deltaY,-24,24)`, factor=`2^(turn*.02)`, anchor at event location |
| Unmodified wheel | Forward to super/parent scrolling even while enlarged; never pan or swallow ordinary scrolling |
| Native magnify | Start from current scale; apply magnification deltas at event location, or cumulative recognizer magnification against captured start scale; choose one API, not both; end/cancel clears gesture state |
| Left mouse drag | Record pointer when zoomable; pan only while enlarged; capture through AppKit's normal mouseDragged/mouseUp sequence; track sum(abs(dx)+abs(dy)); over 4 points counts as drag |
| Right/other mouse | Forward unchanged; never start zoom pan |
| Double click | While resting, show max(1,fit*2) at click; while enlarged, reset; controls do not trigger this |
| Keyboard + or = / - | Only when diagram owns first responder and Ctrl/Command/Option absent; multiply/divide by 1.25; Shift allowed for + |
| Keyboard arrows | Only enlarged: ArrowLeft moves screen +48 x, Right -48 x, Up +48 y, Down -48 y before origin conversion/clamp |
| Keyboard 0 / Escape | Only enlarged: reset; at rest forward rather than consuming |
| Unknown/modified keys | Forward unchanged; never swallow app shortcuts |
| Drag release | Return cursor to grab; suppress the following diagram/link activation when a drag actually occurred; release outside view must clear capture |
| Resize | Reset to fit; preserve result/source/model and sibling layout; mounted controller already owns resize observation |
| New committed drawing | refresh reads new image/natural size, resets camera and recomputes zoomability; same-SVG quiet mount refresh does not reset camera |
| Destroy | Reset to rest, remove controls and focus/input ownership, cancel pointer/pinch state, stop owned observation; leave diagram/image visible |

AppKit's scroll and magnification units are platform mappings, not a claim that
NSEvent is a DOM WheelEvent. Record actual event deltas in native integration and
verify zoom-in direction, stepped mouse wheel, precise trackpad updates and parent
scroll passthrough. Do not introduce an unmeasured line-to-pixel multiplier. A
magnification recognizer must handle cancellation and view/window changes; a
left-button drag must end cleanly after focus loss or teardown. Drawing from these
events must not invoke onRender, because the render result did not change.

Controls are real NSButtons, placed inside a small overlay at top trailing. Default
labels are exactly 'Zoom in', 'Zoom out', 'Reset zoom'; independently overridden
strings become accessibility labels and tooltips. Empty supplied labels remain
empty, matching nullish fallback. Controls=false creates no buttons. Hide the bar
when not zoomable. Disable zoom-in at scale>=maximum*.999; disable zoom-out at
rest; show reset only when enlarged. Buttons stay in the normal keyboard focus
loop and do not also trigger diagram mouse/key handlers. SF Symbols can implement
the native icons without copying the upstream Lucide path. Preserve native high
contrast/keyboard-focus affordances; color/theme is presentation ownership.

The diagram accepts first responder only while zoomable and active. Tab focus
shows a native focus ring around its viewport; a pointer click may establish
first responder for keyboard zoom without an extra pointer-origin ring. Use
NSWindow/NSView responder APIs, not global key monitors. Explicitly draw the focus
ring when AppKit's automatic ring cannot surround the custom NSView. Clear owned
first responder safely when zoom becomes disabled/destroyed; do not disturb focus
already on another control. VoiceOver discovers the named controls, their disabled
state and the zoomable viewport. Full diagram accessible nodes/links remain the
separate presentation-artifact gate.

Native graphics must never receive a nonfinite camera transform. Reject nonfinite
maxScale when enabling a controller with a native semantic error; a finite
maxScale<=fit simply prevents enlargement as pinned max() does. Requests whose
clamped result is finite are supported, including positive/negative infinity with
a finite maximum. A NaN request leaves the viewport unchanged. Record this narrow
native presentation-domain restriction; it is not equivalence to a browser that
can store 'NaN' in a viewBox. Actual engine rendering keeps its existing raw-number
contract. No arbitrary small zoom cap below the user's finite maxScale is added.

Destroy must be idempotent and remove automatic event routing even when called
inside a callback or while dragging. Keep readable scale/fit and owned image data
valid. Pinned closures still allow explicit methods after destroy; preserve
programmatic reset/zoom/refresh on a retained native handle without reviving
automatic observation. Detached/replaced controllers must not regain the view's
event ownership. If such an explicit method changes pixels, it uses the same
camera; it never frees another controller's state. A later enableZoom explicitly
installs a new owner. Test cleanup separately from retained programmatic use.

Mounted integration order follows pinned mount.ts: an existing zoom refreshes
after image/result commit and before onRender. Reconcile changed zoom policy after
onRender/update error handling; an initial successful mount creates zoom after its
first onRender. This means onRender for a policy-changing update can still see the
previous zoom handle. Use current settings after a reentrant callback, not the
outer operation's captured options. Error overlays/dimming remain independent of
camera, and controls remain operable over a last-good drawing. A mount destroy
also destroys its zoom. Explicit update after mount destroy may replace the image
and refresh its retained camera but must not restart automatic input/observation.
Standalone zoom observes its view; mounted zoom receives resize notifications
from Mounted to avoid duplicate reset paths.

Acceptance is one native-window feature suite, not camera unit tests: large image
fit and anchored enlargement; roomy fixed panel with negative-origin clamp;
programmatic minimum/maximum/reset; real button actions and localized labels;
focus traversal and key input; modified wheel versus parent NSScrollView; mouse
drag and release outside; double click; native magnify cancel; resize reset;
changed drawing resets while equal-SVG refresh preserves camera; disabled/always
policy and policy replacement; teardown during drag; two windows/controllers;
last-good error overlay while zoomed. Assert public scale/fit, stable view/sibling
frames, result identity/value and zero render callbacks during camera operations;
capture pixels proving pan and enlargement. Include actual event-path proof rather
than calling only private camera helpers. Link drag suppression is carried into
the later approved-link interaction suite; NSImage alone cannot close link parity.

## Data and extension layers still needed

These are independently tracked gaps, not a demand to simulate all JavaScript.

- **Host Config data:** add a lossless data-value transport using the existing flat
  graph/IEEE/string primitives. The first vertical slice covers nonfinite numeric
  values actually consumed by built-ins, defaults/precedence, and parsed config
  retention. Do not turn every unknown runtime object into a new universal VM.
  Shared mutable host configuration requires an explicit owned reference contract,
  not repeated whole-graph copying on every render.
- **Lossless strings:** source preparation, semantic indexing, model strings,
  callback requests and returned output need a UTF-16-capable execution path.
  Swift JSString and binary snapshots alone are insufficient. Port a small
  end-to-end source-to-render case first; preserve existing String APIs and
  propagate the same primitive through actual string operations, not a lossy
  replacement-character adapter. Lone-surrogate strings are a value-domain gap,
  not Function.toString-style metaprogramming.
- **Callable model hooks:** Gantt.getTasks is declared and may be replaced; foreign
  callback dispatch and model-visible mutation timing need a bound callable
  reference/lifetime/error contract. Start from a real Swift getTasks override
  fixture, with partial-state publication and original exception identity. Add
  targeted read-time synchronization where that fixture proves snapshots wrong;
  do not rewrite every renderer into a generic live-object interpreter.
- **Swift-authored Diagram providers:** Rust DiagramModule/PublicDiagramModule
  already provide native extensions and retained-provider lifetimes. Built-in
  Registry selection alone does not expose arbitrary Swift Diagram registration.
  Before a C callback vtable is published, implement one actual Swift provider
  prototype and prove parse/model/render/free/replace lifetimes and failure paths.
  Use an owned opaque context and retained provider, not borrowed Any pointers.
  This is a separate reviewed interface, not part of lazy built-in selection.

Runtime `model.constructor`, enumerable private caches/prototype reflection,
arbitrary Object coercion and live mutation of a JavaScript Diagram object remain
separate compatibility gates from the declared native equivalents above. Some
are exercised by the mutable-model corpus; none may be relabeled complete because
a read-only projection or a cloned model renders. Conversely, constructor names,
Function.toString and arbitrary exotic getter timing are not invented mandatory
Swift APIs. The existing model LLD retains those boundaries and evidence.

## Release gates and truthful completion

After the layers above, run the consolidated Rust/C/Swift public integration
matrix, actual arm64 and x86_64 SDK builds, native window lifecycle/interaction
proof, and fresh external Swift-package consumers using the produced artifact.
Verify matching exported symbols, header, binary checksum, minimum macOS/Swift
versions and old binary/legacy API compatibility. Update package documentation
from measured capability/evidence, not diagram count. Do not claim iOS support:
the current package declares macOS 14+.

Report separately: diagram corpus/render coverage; declared API coverage; mutable
model/foreign callback coverage; lossless input domains; native presentation and
resources; release/distribution proof. Browser work is independently verified by
its owners. Neither all 33 renderers nor a local universal library closes this
native/Swift completion checklist by itself.

The v9 bounded-error review found the old shared compatibility_failure writer
could emit oversized syntax diagnostics. Current v9 routes through bounded_call,
which contains serialization/unwind and uses constant bounded fallback envelopes;
the C feature test supplies an explicit-length 6 MiB NUL line. That addresses the
new v9 result-byte contract structurally; runtime proof belongs to the parent
verification batch. Earlier APIs' error-envelope budgets remain separate work.


## Native view lifecycle acceptance checkpoint (2026-10-19)

The rebuilt ARM64 SDK passes ten native-window feature tests. Ordered-front
NSWindow fixtures exercise real DiagramView bitmap pixels, retained NSImage
pixels, dimmed diagnostics, failed/deferred updates and recovery, callback
publication/reentry, replacement/teardown, detached/zero-width initialization,
adaptive host-follow-width quietness, Gantt retained cache/clock behavior and
unique versus explicit drawing IDs. Native measurement/preparation reentry is
rejected before changing ownership. Original class-based callback errors retain
identity; structured value errors compare fields, not AnyObject bridge boxes.

DiagramView owns its controller; native documents retain parsed and drawn roles
separately. Font-set and window changes invalidate presentation without reparsing;
`invalidateResources()` explicitly refreshes host resources. Native presentation
resolves the default palette and rejects unusable image dimensions without
changing raw engine numeric acceptance. Errors over an existing drawing appear as
a bottom one-line summary with complete tooltip/accessibility help. Error-only
views display the full diagnostic.

This proves the first presentation layer, not interactive SVG parity. NSImage
still supplies no link regions, zoom, image-resource policy or complete diagram
accessibility tree. The next layers above remain release gates. x86_64/Rosetta
verification and independent consumers are tracked separately.
