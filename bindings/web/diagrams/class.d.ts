import type {ProviderRendered,RenderOptions,Diagram,ClassModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<ClassModel,'type' | 'direction' | 'classes' | 'relations' | 'notes' | 'namespaces' | 'interfaces' | 'styleClasses' | 'title' | 'accTitle' | 'accDescr'>;
declare const diagram:Diagram<ClassModel> & {render(model:RenderModel,config:Parameters<Diagram<ClassModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
