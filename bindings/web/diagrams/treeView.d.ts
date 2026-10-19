import type {ProviderRendered,RenderOptions,Diagram,TreeViewModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<TreeViewModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'root' | 'count'>;
declare const diagram:Diagram<TreeViewModel> & {render(model:RenderModel,config:Parameters<Diagram<TreeViewModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
