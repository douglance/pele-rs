import type {ProviderRendered,RenderOptions,Diagram,AgentflowModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<AgentflowModel,'type' | 'direction' | 'nodes' | 'edges' | 'subgraphs' | 'connectors' | 'classes' | 'tooltips' | 'defaultEdgeStyle' | 'defaultInterpolate' | 'mappings' | 'diagnostics' | 'title' | 'accTitle' | 'accDescr' | 'semanticModel'>;
declare const diagram:Diagram<AgentflowModel> & {render(model:RenderModel,config:Parameters<Diagram<AgentflowModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
