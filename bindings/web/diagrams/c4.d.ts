import type {ProviderRendered,RenderOptions,Diagram,C4Model} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<C4Model,'type' | 'c4Type' | 'title' | 'accTitle' | 'accDescr' | 'shapes' | 'boundaries' | 'rels' | 'shapeInRow' | 'boundaryInRow'>;
declare const diagram:Diagram<C4Model> & {render(model:RenderModel,config:Parameters<Diagram<C4Model>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
