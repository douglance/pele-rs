import type {ProviderRendered,RenderOptions,Diagram,ErModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<ErModel,'type' | 'direction' | 'entities' | 'relationships' | 'classes' | 'subgraphs' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<ErModel> & {render(model:RenderModel,config:Parameters<Diagram<ErModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
