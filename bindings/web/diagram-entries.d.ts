/** Initialize the Rust WASM runtime once before synchronous low-level entry calls. */
export function loadDiagramEntries(moduleOrPath?:string|URL|Response|BufferSource|WebAssembly.Module):Promise<void>;
/** Dispose a model returned by one of this runtime's low-level entries. */
export function disposeDiagramModel(model:object):void;
export type {Diagram,ProviderRendered,RenderOptions,DiagramModel} from './full.js';
