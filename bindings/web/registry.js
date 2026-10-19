import initialize from './initialize.js';
// Owned built-in selection. Registration and parsing remain in Rust.
import {NativeRegistry} from './pkg/pele.js';
import {createRenderer} from './full.js';
export {PeleError, PeleError as PeleCompatibilityError} from './errors.js';

/** Select linked native built-ins in an independent registry (empty by default). */
export async function loadRegistry(moduleOrPath, options = {}) {
  await initialize(moduleOrPath === undefined ? undefined : {module_or_path: moduleOrPath});
  return createRenderer(new NativeRegistry(options.allBuiltins === true));
}
