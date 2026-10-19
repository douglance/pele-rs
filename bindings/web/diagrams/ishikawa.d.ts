import type {ProviderRendered,RenderOptions,Diagram,IshikawaModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<IshikawaModel,'type' | 'title' | 'root'>;
declare const diagram:Diagram<IshikawaModel> & {render(model:RenderModel,config:Parameters<Diagram<IshikawaModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
