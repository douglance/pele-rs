import type {ProviderRendered,RenderOptions,Diagram,WardleyModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<WardleyModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'nodes' | 'links' | 'trends' | 'pipelines' | 'annotations' | 'notes' | 'accelerators' | 'deaccelerators' | 'annotationsBox' | 'axes' | 'size'>;
declare const diagram:Diagram<WardleyModel> & {render(model:RenderModel,config:Parameters<Diagram<WardleyModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
