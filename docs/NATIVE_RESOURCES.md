# Native font and approved raster resources

This additive native lane keeps parsing, URI policy, layout and SVG generation in Rust. `Pele.NativeResources` owns native measurements and approved image bytes. `Engine.renderNative`, retained immutable and mutable `renderNative`, and built-in native mounts use the same lane. Existing render/presentation/C exports preserve their URI and measurement contract.

## Fonts

Native mounts create an owner unless `MountOptions.nativeResources` supplies one. Missing host text measurement uses CoreText with the concrete installed font family written into render options. Explicit `HostResources.textWidth` stays authoritative. Family fallback lists are resolved against installed fonts; generic sans-serif/system-ui, serif and monospace map to Helvetica, Times and Menlo. Unknown families fail explicitly. Bold/italic/mono style bits use matching CoreText symbolic traits. This is platform font measurement, not a promise of pixel identity with a browser font backend or arbitrary font fallback/shaping systems.

Each owner caches at most 128 CoreText font instances, 4096 width entries and 2 MiB of estimated text/key storage. Width keys retain exact UTF-16 units; image approval cache keys retain exact URI UTF-8 bytes, avoiding Swift canonical-equivalence aliases. A measurement request is capped at 65536 UTF-16 units and a finite font size from above zero through 4096. Cache saturation bypasses caching. The cache uses a lock and does not expose mutable CoreText objects.

`Mounted.invalidateResources()` clears the owner's caches, increments its generation and redraws the retained model. Font-set changes use the same path. A shared owner's explicit `invalidate()` is observed on the next mounted refresh; it does not synchronously enumerate or reenter all views. Hosts changing image-loader content should invalidate explicitly. Hidden/zero-width mounts defer redraw as before; the previous accepted image remains visible on later resource failures.

## Approved images

Actual generated image inventory: Flowchart emits `<image>` in `flowchart/render.rs`; Swimlane delegates to that renderer. No other current diagram renderer emits image references. Icon SVG and provider-returned SVG are trusted host output and remain a distinct boundary.

After `safe_url` and `image_url` accept a reference, the renderer asks `RenderResources.image`. Its portable default returns `ImageResource::Passthrough` with no extra resource allocation. Native C14 wraps the host and requires explicit raster resolution. Disabled images and disallowed URI schemes never call the loader. No loader, a missing approval, invalid format, or exceeded budget fails before the built-in SVG reaches NSImage. The library never fetches a URL or reads an image file. A host callback may implement its own acquisition policy.

The Swift loader receives the approved URI and returns optional Data. Input bytes are bounded before ImageIO inspection. Only single-image PNG/JPEG is accepted. ImageIO reads dimensions, validates the decoded bitmap and re-encodes a self-contained PNG. Rust independently validates raster headers/dimensions and embeds a bounded base64 data URI. Swift never reparses source or SVG and never estimates image placement. Existing Rust geometry and presentation frames stay unchanged.

Hard ceilings: 8 MiB encoded bytes per image, 16 MiB aggregate encoded image emissions per render operation (including adaptive attempts), 16 MiB unique cached bytes per owner generation, 128 distinct references, 4096 URI bytes, 8192 pixels per axis, 16,777,216 total decoded pixels per image. Requested budgets may lower these limits. Repeated references/adaptive passes reuse approved bytes, while each emitted copy still spends the operation byte budget; this bounds repeated base64 expansion rather than counting only distinct loader payloads. These are input/output/cache ceilings, not a universal peak-memory guarantee: platform decode and bounded bitmap re-encoding use temporary storage proportional to the pixel budget. Direct Rust consumers of `EmbeddedImage::from_encoded` receive bounded header validation; complete pixel validation remains the host decoder's responsibility.

Trusted custom providers and `TrustedSvg` icon callbacks can produce arbitrary host markup. Their code/markup is not covered by the built-in image-reference guarantee; normal provider mounting remains supported. Hosts needing stricter custom-output handling supply `preparePresentation` and own that output policy. No source/SVG scanner or blanket provider rejection is introduced.

## C14 ownership

`PeleNativeResourcesV14` is 64 bytes with version 14, exact struct size and zero reserved fields. It contains a synchronous borrowed callback plus optional tighter limits. NULL selects strict defaults without an image resolver. The callback must keep returned bytes readable until Rust copies/encodes them immediately after return, and therefore until the synchronous render call completes or the next loader callback. Swift retains NSData for that duration. All old ABI exports remain unchanged.

Source, retained immutable and mutable C14 exports reuse numeric/output/presentation envelopes and existing panic/error bounds. Mutable callbacks retain owner-bound callable resources, reject same-document reentry, and publish post-operation revision/graph on dispatched errors. Swift restores the original image/measurement/host-function Error after consuming that operation outcome. A failed native mount retains its last accepted image/metadata/actions and can recover after invalidation.

## Feature proof

`native_images.rs` covers real source/typed/mutable rendering, URI policy ordering, unchanged dimensions, presentation capture, errors/recovery and raster header/limit rejection. `ffi_native_resources_v14.c` exercises actual ABI packet validation, approved bytes, callback/reentry errors, limits and mutable publication. `NativeResourceUITests` runs in the signed active native QA app: actual red/blue raster pixels, cache generations, last-good failure/recovery, installed-font geometry, caller measurements and original callback identity. Proof status and artifact hashes are recorded in the handoff after execution; source preparation alone is not completion.

Prepared/raw C13 rendering preserves its low-level SVG contract and is not implicitly routed through C14. The initial strict native-resource surface is source, retained immutable, mutable, and built-in Mounted rendering. Trusted prepared/raw consumers remain responsible for their presentation resource policy.

### Isolated verification checkpoint

Core17/17 passed, including complete Flowchart/Swimlane historical corpora, new raster feature3 and existing selected-presentation7. Strict C14 integration passed against an isolated ARM64 debug archive. Final Swift release compile and two non-window resource features passed, including exact UTF-8 approval identity, ImageIO format/size checks, original Error identity and mutable revision recovery.

Actual signed GUI31 initially executed with active/key windows: existing26 passed; new pixel fixtures exposed transparent named-color bitmap generation and an invalid assumption that recorded mount errors throw. Corrected explicit-RGBA source then produced real colored embedded raster pixels; a device-profile transform made the old pure-channel classifier incorrect. Final tests require strong red/blue channel dominance and retain colored-area, changed-pixel and last-good assertions. Two final bounded runs could not activate the desktop (READY active=0, zero assertions); final full GUI31 remains a release gate, not a passing claim. No headless/fake-key fallback is used.
