import type {ProviderRendered,RenderOptions,Diagram,InfoModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<InfoModel,'type' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<InfoModel> & {render(model:RenderModel,config:Parameters<Diagram<InfoModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
