import {loadRegistry, type Registry} from '../../bindings/web/registry.js';
import {loadLazy, withLazy, type LazyRegistry} from '../../bindings/web/lazy.js';
import type {Mounted} from '../../bindings/web/full.js';

async function ownedSelection(element: HTMLElement): Promise<string> {
  const registry: Registry = await loadRegistry();
  registry.registerBuiltin('pie');
  const model = registry.parse('pie\n"A":1', {limit: Infinity, maxEdges: 2.5});
  if (model.type === 'pie') model.sections.set('B', 2);
  const lazy: LazyRegistry = withLazy(registry);
  const selected: boolean = await lazy.load(registry.detectType('sequenceDiagram\nA->>B: Hi'));
  const mounted: Mounted = await lazy.mountAsync(element, 'pie\n"A":1', {zoom: false});
  if (selected) mounted.refresh();
  const second = await loadLazy();
  const result = await second.renderAsync('info');
  mounted.destroy();registry.close();second.close();
  const retained = registry.render(model); // A parsed model retains its native registration.
  registry.dispose(model);
  return result.svg + retained.svg;
}
void ownedSelection;
