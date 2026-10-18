# Owned built-in registries

`loadRegistry()` creates an independent, empty native registry. Select a linked
built-in with `registerBuiltin('pie')`; `loadRegistry(undefined, {allBuiltins:true})`
selects all 33. Discovery preprocesses independently of registry membership.
Parsing retains the chosen native module and config. A parsed model can still be
edited and rendered after replacement or registry `close()`; dispose it separately.

`loadLazy()` adds `load(type)`, `renderAsync(source, options)` and
`mountAsync(element, source, options)`. Concurrent pending selection shares its
Promise, failed selection can retry, and async source operations select the
detected family before parsing. A mounted document's later synchronous `update`
does not load another family automatically. Native built-ins are already linked
in the WASM artifact; lazy selection does not promise binary code splitting.

This is a supported built-in layer, not full JavaScript provider registration.
Pinned Pele `register(...Diagram)` accepts arbitrary host-authored parse/render
functions and per-diagram entries expose those low-level functions with encoded
source, config, title, line offset and parse policies. The current WASM boundary
does not expose an owned foreign `DiagramModule` provider or that low-level parse
entry. Consequently `register(Diagram)` and `./diagrams/*` are deliberately absent;
there are no stand-in JavaScript renderers or fabricated native descriptors.

Source/config domains retain the documented UTF-8 and native transport bounds.
No malformed UTF-16, arbitrary JavaScript coercion, constructor or method-replacement
capability is added by selecting a built-in.
