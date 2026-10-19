import initialize from './initialize.js';
export {loadRegistry} from './registry.js';
import { inspect as rawInspect, capabilities as rawCapabilities } from './pkg/pele.js';
import {PeleError, fromNativeError} from './errors.js';
export {PeleError, PeleError as PeleCompatibilityError};
function result(json) {
  const value = JSON.parse(json);
  if (value.version !== 2) throw new Error('Unsupported Pele compatibility contract');
  if (!value.ok) throw fromNativeError(value.error);
  return value.result;
}
/** Minimal compatibility entry. Capabilities report implementation progress truthfully. */
export async function loadCore(moduleOrPath) {
  await initialize(moduleOrPath === undefined ? undefined : { module_or_path: moduleOrPath });
  const inspect = (source, options = {}) => result(rawInspect(source, JSON.stringify(options)));
  return {
    inspect,
    detectType(source) { return inspect(source, { limit: source.length }).type; },
    supports(source) { try { return inspect(source, { limit: source.length }).registered; } catch { return false; } },
    capabilities() { return result(rawCapabilities()); }
  };
}
