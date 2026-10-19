import type {ProviderRendered,RenderOptions,Diagram,GanttModel} from '../full.js';
/** Data required by the pinned low-level renderer; returned DB models retain their richer methods. */
export type RenderModel=Pick<GanttModel,'type' | 'title' | 'accTitle' | 'accDescr' | 'dateFormat' | 'axisFormat' | 'tickInterval' | 'todayMarker' | 'includes' | 'excludes' | 'inclusiveEndDates' | 'topAxis' | 'displayMode' | 'weekday' | 'weekend' | 'sections' | 'links' | 'tasks' | 'getTasks'>;
declare const diagram:Diagram<GanttModel> & {render(model:RenderModel,config:Parameters<Diagram<GanttModel>['render']>[1],options:RenderOptions):ProviderRendered};
export default diagram;
