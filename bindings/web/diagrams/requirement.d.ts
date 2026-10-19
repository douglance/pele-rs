import type {ProviderRendered,RenderOptions,Diagram,RequirementModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<RequirementModel,'type' | 'direction' | 'requirements' | 'elements' | 'relations' | 'classes' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<RequirementModel> & {render(model:RenderModel,config:Parameters<Diagram<RequirementModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
