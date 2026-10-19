import type {ProviderRendered,RenderOptions,Diagram,CynefinModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<CynefinModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'domains' | 'transitions'>;
declare const diagram:Diagram<CynefinModel> & {render(model:RenderModel,config:Parameters<Diagram<CynefinModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
