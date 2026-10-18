import type {ProviderRendered,RenderOptions,Diagram,TimelineModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<TimelineModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'direction' | 'sections' | 'periods'>;
declare const diagram:Diagram<TimelineModel> & {render(model:RenderModel,config:Parameters<Diagram<TimelineModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
