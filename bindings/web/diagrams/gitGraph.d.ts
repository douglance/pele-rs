import type {ProviderRendered,RenderOptions,Diagram,GitModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<GitModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'direction' | 'commits' | 'branches' | 'lanes' | 'currentBranch' | 'head' | 'warnings'>;
declare const diagram:Diagram<GitModel> & {render(model:RenderModel,config:Parameters<Diagram<GitModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
