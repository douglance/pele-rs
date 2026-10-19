// Pele lazy.ts at 458322a5 (MIT, kepano): pending selection and async ordering.
// Native built-ins are already linked; this selects them, not a code-split WASM.
import {loadRegistry} from './registry.js';
export {loadRegistry} from './registry.js';
export {PeleError, PeleError as PeleCompatibilityError} from './errors.js';
export {enableZoom} from './zoom.js';
export {forgetTextWidths} from './resources.js';

/** Add pinned asynchronous selection semantics to an owned native registry. */
export function withLazy(registry) {
  const pending = new Map();
  const known = new Set(registry.capabilities().filter(capability => capability.known).map(capability => capability.type));
  const load = type => {
    if (type === null) return Promise.resolve(false);
    if (registry.registered(type)) return Promise.resolve(true);
    let promise = pending.get(type);
    if (promise === undefined) {
      if (!known.has(type)) return Promise.resolve(false);
      promise = Promise.resolve().then(async () => {
        const [{default: diagram},{loadDiagramEntries}] = await Promise.all([import('./diagrams/'+type+'.js'),import('./diagram-entries.js')]);
        await loadDiagramEntries();
        registry.registerBuiltin(type);
        registry.register(diagram);
        return true;
      }).catch(error => {
        pending.delete(type);
        throw error;
      });
      pending.set(type, promise);
    }
    return promise;
  };
  return Object.assign(registry, {
    load,
    async renderAsync(source, options = {}) {
      let type = null;
      try {type = registry.detectType(source);} catch { /* render reports source errors. */ }
      await load(type);
      return registry.render(source, options);
    },
    async mountAsync(element, source, options = {}) {
      let type = null;
      try {type = registry.detectType(source);} catch { /* mount presents source errors. */ }
      await load(type);
      return registry.mount(element, source, options);
    },
  });
}

/** Creates an independent empty native registry and asynchronous convenience API. */
export async function loadLazy(moduleOrPath) {
  return withLazy(await loadRegistry(moduleOrPath));
}
