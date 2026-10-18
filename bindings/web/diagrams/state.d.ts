import type {ProviderRendered,RenderOptions,Diagram,StateModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<StateModel,'type' | 'direction' | 'rootDoc' | 'states' | 'relations' | 'classes' | 'links' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<StateModel> & {render(model:RenderModel,config:Parameters<Diagram<StateModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
