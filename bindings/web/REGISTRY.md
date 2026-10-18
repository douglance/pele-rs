# Owned diagram registries

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

`register(...diagrams)` accepts ordinary caller-authored `Diagram<Model>` objects
for canonical detected types. The caller supplies its parser and renderer; every
built-in algorithm remains Rust. The host methods receive the pinned low-level
contract: prepared/entity-encoded source, merged config, title, line offset and
exact maxEdges; rendering receives the original model, retained config and options.
Rust resolves responsive defaults and applies output limits, icon-name decoding
and returned link-ID decoding.

```js
const registry = await loadRegistry();
registry.register({
  type: 'pie',
  parse(source, config, title, lineOffset, limits) {
    return {source, config, title, lineOffset, limits, width: 40};
  },
  render(model, config, options) {
    return {svg: `<svg width="${model.width}" height="20"/>`,
      width: model.width, height: 20, links: []};
  }
});
const model = registry.parse('pie');
model.width = 80;
const drawing = registry.render(model);
registry.close(); // the model retains its selected provider
registry.dispose(model);
```

Replacing registration affects future parses. Old models retain the old provider
object and observe its later render/section edits. keepComments is read per parse;
provider.type is captured as the registration key. Provider callbacks preserve
original thrown values and partial model edits. A renderer may catch resource
callback errors and recover. Same-document reentrant render/dispose is rejected;
registration or registry close inside a parser does not invalidate that parse.

Models remain ordinary JS objects: custom prototypes, Maps and aliases are not
serialized. Config crosses Rust as bounded finite JSON (1 MiB); the same JS config
object is retained for callbacks, and finite edits are synchronized before render.
Nonfinite Config and cycles are outside this transport; source/SVG use the existing
well-formed UTF-8 boundary. SVG plus returned link text is bounded independently of
numeric output limits. These bounds do not constrain arbitrary host callback work.

The `./diagrams/*` low-level built-in entries, isolated UTF-16 text and arbitrary
runtime coercion/method replacement on native built-in models remain separate API
gaps. Custom registration does not imply those capabilities or full API parity.
