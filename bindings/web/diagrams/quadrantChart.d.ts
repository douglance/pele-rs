import type {ProviderRendered,RenderOptions,Diagram,QuadrantModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<QuadrantModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'quadrants' | 'xAxisLeft' | 'xAxisRight' | 'yAxisBottom' | 'yAxisTop' | 'points' | 'classes'>;
declare const diagram:Diagram<QuadrantModel> & {render(model:RenderModel,config:Parameters<Diagram<QuadrantModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
