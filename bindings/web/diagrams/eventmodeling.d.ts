import type {ProviderRendered,RenderOptions,Diagram,EventModelingModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<EventModelingModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'boxes' | 'lanes' | 'relations' | 'notes' | 'specifications' | 'entities'>;
declare const diagram:Diagram<EventModelingModel> & {render(model:RenderModel,config:Parameters<Diagram<EventModelingModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
