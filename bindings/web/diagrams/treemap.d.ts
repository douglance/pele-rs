import type {ProviderRendered,RenderOptions,Diagram,TreemapModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<TreemapModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'roots' | 'classes'>;
declare const diagram:Diagram<TreemapModel> & {render(model:RenderModel,config:Parameters<Diagram<TreemapModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
