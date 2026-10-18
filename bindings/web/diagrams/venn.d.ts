import type {ProviderRendered,RenderOptions,Diagram,VennModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<VennModel,'type' | 'title' | 'subsets' | 'textNodes' | 'styles'>;
declare const diagram:Diagram<VennModel> & {render(model:RenderModel,config:Parameters<Diagram<VennModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
