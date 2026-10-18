import type {ProviderRendered,RenderOptions,Diagram,PieModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<PieModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'showData' | 'sections'>;
declare const diagram:Diagram<PieModel> & {render(model:RenderModel,config:Parameters<Diagram<PieModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
