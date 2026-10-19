import type {ProviderRendered,RenderOptions,Diagram,JourneyModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<JourneyModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'sections' | 'tasks'>;
declare const diagram:Diagram<JourneyModel> & {render(model:RenderModel,config:Parameters<Diagram<JourneyModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
