import type {ProviderRendered,RenderOptions,Diagram,KanbanModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<KanbanModel,'type' | 'title' | 'sections'>;
declare const diagram:Diagram<KanbanModel> & {render(model:RenderModel,config:Parameters<Diagram<KanbanModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
