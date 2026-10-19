import type {ProviderRendered,RenderOptions,Diagram,ArchitectureModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<ArchitectureModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'nodes' | 'groups' | 'edges' | 'layoutHints'>;
declare const diagram:Diagram<ArchitectureModel> & {render(model:RenderModel,config:Parameters<Diagram<ArchitectureModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
