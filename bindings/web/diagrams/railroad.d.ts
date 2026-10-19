import type {ProviderRendered,RenderOptions,Diagram,RailroadModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<RailroadModel,'type' | 'notation' | 'title' | 'accTitle' | 'accDescr' | 'rules'>;
declare const diagram:Diagram<RailroadModel> & {render(model:RenderModel,config:Parameters<Diagram<RailroadModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
