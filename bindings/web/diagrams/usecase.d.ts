import type {ProviderRendered,RenderOptions,Diagram,UsecaseModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<UsecaseModel,'type' | 'actors' | 'useCases' | 'systemBoundaries' | 'relationships' | 'notes' | 'jsonNodes' | 'classDefs' | 'direction' | 'title' | 'accTitle' | 'accDescr' | 'accDescription' | 'source' | 'headerSpan' | 'statements'>;
declare const diagram:Diagram<UsecaseModel> & {render(model:RenderModel,config:Parameters<Diagram<UsecaseModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
