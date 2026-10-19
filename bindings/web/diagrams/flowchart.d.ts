import type {ProviderRendered,RenderOptions,Diagram,FlowchartModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<FlowchartModel,'type' | 'direction' | 'nodes' | 'edges' | 'subgraphs' | 'classes' | 'tooltips' | 'defaultEdgeStyle' | 'defaultInterpolate' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<FlowchartModel> & {render(model:RenderModel,config:Parameters<Diagram<FlowchartModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
