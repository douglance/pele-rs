import type {ProviderRendered,RenderOptions,Diagram,MindmapModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<MindmapModel,'type' | 'title' | 'nodes'>;
declare const diagram:Diagram<MindmapModel> & {render(model:RenderModel,config:Parameters<Diagram<MindmapModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
