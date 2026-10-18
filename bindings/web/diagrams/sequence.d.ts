import type {ProviderRendered,RenderOptions,Diagram,SequenceModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<SequenceModel,'type' | 'actors' | 'createdActors' | 'destroyedActors' | 'boxes' | 'messages' | 'notes' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<SequenceModel> & {render(model:RenderModel,config:Parameters<Diagram<SequenceModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
