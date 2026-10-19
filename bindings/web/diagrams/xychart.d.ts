import type {ProviderRendered,RenderOptions,Diagram,XyChartModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<XyChartModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'orientation' | 'xAxis' | 'yAxis' | 'hasSetYAxis' | 'plots'>;
declare const diagram:Diagram<XyChartModel> & {render(model:RenderModel,config:Parameters<Diagram<XyChartModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
