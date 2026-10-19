import type {ProviderRendered,RenderOptions,Diagram,RadarModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<RadarModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'axes' | 'curves' | 'options'>;
declare const diagram:Diagram<RadarModel> & {render(model:RenderModel,config:Parameters<Diagram<RadarModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
