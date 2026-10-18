import type {ProviderRendered,RenderOptions,Diagram,BlockModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<BlockModel,'type' | 'root' | 'blocks' | 'edges' | 'classes' | 'byId' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<BlockModel> & {render(model:RenderModel,config:Parameters<Diagram<BlockModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
