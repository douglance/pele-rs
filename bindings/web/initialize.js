import initializeWasm from './pkg/pele.js';

// All public facades share wasm-bindgen's module-global memory. Concurrent
// instantiations would replace it while other facades already own native handles.
let pending;
export default function initialize(options) {
  if (!pending) {
    pending = initializeWasm(options).catch(error => {
      pending = undefined;
      throw error;
    });
  }
  return pending;
}
