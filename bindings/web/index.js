import initialize, { render as rawRender, version } from './pkg/pele.js';

export class PeleError extends Error {
  constructor(code, message, span) {
    super(message); this.name = 'PeleError'; this.code = code; this.span = span;
  }
}
/** Initialize once, then render synchronously. Optional URL/bytes support bundler assets. */
export async function loadPele(moduleOrPath) {
  await initialize(moduleOrPath === undefined ? undefined : { module_or_path: moduleOrPath });
  if (version() !== 1) throw new PeleError('internal', 'Unsupported engine version');
  return {
    render(source, options = {}) {
      let envelope;
      try { envelope = JSON.parse(rawRender(source, JSON.stringify(options))); }
      catch { throw new PeleError('internal', 'WASM failed; reload the engine before another render'); }
      if (envelope.version !== 1) throw new PeleError('internal', 'Unsupported result version');
      if (!envelope.ok) throw new PeleError(envelope.error.code, envelope.error.message, envelope.error.span);
      return envelope.result;
    }
  };
}
