import type {ProviderRendered,RenderOptions,Diagram,PacketModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<PacketModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'bitsPerRow' | 'rows'>;
declare const diagram:Diagram<PacketModel> & {render(model:RenderModel,config:Parameters<Diagram<PacketModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
