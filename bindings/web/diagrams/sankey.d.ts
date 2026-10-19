import type {ProviderRendered,RenderOptions,Diagram,SankeyModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<SankeyModel,'type' | 'nodes' | 'links' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<SankeyModel> & {render(model:RenderModel,config:Parameters<Diagram<SankeyModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
