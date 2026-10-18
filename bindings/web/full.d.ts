import type { ConfigValue, DiagramId, Capability } from './core.js';
export interface ParseOptions { limit?: number; maxEdges?: number; config?: { [key: string]: ConfigValue } }
export interface RenderOptions extends ParseOptions {
  measurer?: { width(text: string, size: number, style: number): number };
  icons?: (name: string) => string | null | undefined;
  nativeTheme?: { background?: string; foreground?: string; muted?: string; line?: string; surface?: string; surfaceAlt?: string; border?: string; accent?: string; series?: [string,string,string,string,string,string,string,string] };
  fontFamily?: string; fontFamilyMono?: string; fontSize?: number; idPrefix?: string;
  responsive?: boolean; maxWidth?: number; autoDirection?: boolean; directionBreakpoint?: number;
  padding?: number; outputLimit?: number; linkSchemes?: string[]; imageSchemes?: string[];
  links?: boolean; images?: boolean; linkRel?: string; now?: number;
}
export interface PieModel { type: 'pie'; title?: string; accTitle?: string; accDescr?: string; showData: boolean; sections: ReadonlyMap<string, number> }
export interface AccessibleModel { title?: string; accTitle?: string; accDescr?: string }
export interface PacketBlock { start: number; end: number; bits: number; label: string }
export interface PacketModel extends AccessibleModel { type: 'packet'; bitsPerRow: number; rows: PacketBlock[][] }
export interface RadarAxis { name: string; label: string }
export interface RadarCurve { name: string; label: string; entries: number[] }
export interface RadarModel extends AccessibleModel { type: 'radar'; axes: RadarAxis[]; curves: RadarCurve[]; options: { showLegend: boolean; ticks: number; max: number | null; min: number; graticule: 'circle' | 'polygon' } }
export interface QuadrantText { text: string; type: string }
export interface PointStyle { radius?: number; color?: string; strokeColor?: string; strokeWidth?: string }
export interface QuadrantPoint extends PointStyle { x: number; y: number; text: QuadrantText; className: string }
export interface QuadrantModel extends AccessibleModel { type: 'quadrantChart'; quadrants: [QuadrantText | null, QuadrantText | null, QuadrantText | null, QuadrantText | null]; xAxisLeft?: QuadrantText; xAxisRight?: QuadrantText; yAxisBottom?: QuadrantText; yAxisTop?: QuadrantText; points: QuadrantPoint[]; classes: ReadonlyMap<string, PointStyle> }
export interface TimelinePeriod { text: string; events: string[]; section: string; sectionIndex: number }
export interface TimelineModel extends AccessibleModel { type: 'timeline'; direction: string; sections: string[]; periods: TimelinePeriod[]; currentSection: string }
export interface JourneyTask { task: string; score: number; people: string[]; section: string }
export interface JourneyModel extends AccessibleModel { type: 'journey'; sections: string[]; tasks: JourneyTask[]; currentSection: string }
export interface FlowNode { id: string; text?: string; labelType: string; type?: string; styles: string[]; classes: string[]; dir?: string; props?: { [key: string]: ConfigValue }; link?: string; linkTarget?: string; icon?: string; form?: string; pos?: string; img?: string; constraint?: string; assetWidth?: ConfigValue; assetHeight?: ConfigValue }
export interface FlowEdge { start: string; end: string; type?: string; text: string; labelType: string; stroke?: string; length?: ConfigValue; style?: string[]; classes: string[]; id?: string; isUserDefinedId: boolean; interpolate?: ConfigValue; animate?: ConfigValue; animation?: ConfigValue }
export interface FlowClassDef { id: string; styles: string[]; textStyles: string[] }
export interface FlowSubgraph { id: string; nodes: string[]; title: string; classes: string[]; dir?: string; labelType: string; metadata?: { [key: string]: ConfigValue } }
export interface FlowchartModel extends AccessibleModel { type: 'flowchart'; direction?: string; nodes: ReadonlyMap<string, FlowNode>; edges: FlowEdge[]; subgraphs: FlowSubgraph[]; classes: ReadonlyMap<string, FlowClassDef>; tooltips: ReadonlyMap<string, string>; defaultEdgeStyle?: string[]; defaultInterpolate?: ConfigValue }
export interface KanbanNode { id: string; level: number; label: string; icon?: string; assigned?: string; ticket?: string; priority?: string; cssClasses?: string }
export interface KanbanSection extends KanbanNode { items: KanbanNode[] }
export interface KanbanModel { type: 'kanban'; title?: string; sections: KanbanSection[]; last?: KanbanNode | KanbanSection; sectionLevel: number; misplaced?: KanbanNode }
export interface MindmapNode { id: number; nodeId: string; level: number; descr: string; type: number; children: MindmapNode[]; isRoot: boolean; section?: number; class?: string; icon?: string }
export interface MindmapModel { type: 'mindmap'; title?: string; nodes: MindmapNode[]; baseLevel: number; open: MindmapNode[] }
export interface ClassMember { id: string; memberType: string; visibility: string; classifier: string; parameters: string; returnType: string }
export interface ClassNode { id: string; type: string; label: string; cssClasses: string; methods: ClassMember[]; members: ClassMember[]; annotations: string[]; styles: string[]; parent?: string; link?: string; linkTarget?: string; haveCallback?: boolean; tooltip?: string }
export interface ClassRelation { id1: string; id2: string; relationTitle1: string; relationTitle2: string; title?: string; relation: { type1: number | 'none'; type2: number | 'none'; lineType: number } }
export interface ClassNote { id: string; class?: string; text: string; index: number; parent?: string }
export interface ClassNamespace { id: string; label: string; classes: ReadonlyMap<string, ClassNode>; notes: ReadonlyMap<string, ClassNote>; children: ReadonlyMap<string, ClassNamespace>; parent?: string; explicit: boolean }
export interface ClassModel extends AccessibleModel { type: 'class'; lineType: { LINE: 0; DOTTED_LINE: 1 }; relationType: { AGGREGATION: 0; EXTENSION: 1; COMPOSITION: 2; DEPENDENCY: 3; LOLLIPOP: 4 }; direction: string; classes: ReadonlyMap<string, ClassNode>; relations: ClassRelation[]; notes: ReadonlyMap<string, ClassNote>; namespaces: ReadonlyMap<string, ClassNamespace>; interfaces: { id: string; label: string; classId: string }[]; styleClasses: ReadonlyMap<string, FlowClassDef>; namespaceStack: string[]; withCssClass: ReadonlyMap<string, Readonly<Record<string, never>>> }
export interface ErAttribute { type: string; name: string; keys: string[]; comment: string }
export interface ErEntity { id: string; label: string; alias: string; attributes: ErAttribute[]; cssClasses: string; cssStyles: string[] }
export type ErCardinality = 'ZERO_OR_ONE' | 'ZERO_OR_MORE' | 'ONE_OR_MORE' | 'ONLY_ONE' | 'MD_PARENT';
export interface ErRelationship { entityA: string; roleA: string; entityB: string; relSpec: { cardA: ErCardinality; relType: 'NON_IDENTIFYING' | 'IDENTIFYING'; cardB: ErCardinality } }
export interface ErSubgraph extends FlowSubgraph { cssStyles: string[] }
export interface ErModel extends AccessibleModel { type: 'er'; direction: string; entities: ReadonlyMap<string, ErEntity>; relationships: ErRelationship[]; classes: ReadonlyMap<string, FlowClassDef>; subgraphs: ErSubgraph[] }
export interface XyLinearAxis { type: 'linear'; title: string; min: number | null; max: number | null }
export interface XyBandAxis { type: 'band'; title: string; categories: string[] }
export interface XyPlot { type: 'line' | 'bar'; title: string; data: [string, number | null][]; pointLabels?: string[] }
export interface XyChartModel extends AccessibleModel { type: 'xychart'; orientation?: 'vertical' | 'horizontal'; xAxis: XyLinearAxis | XyBandAxis; yAxis: XyLinearAxis; hasSetXAxis: boolean; hasSetYAxis: boolean; plots: XyPlot[] }
export interface StateDeclaration { stmt: 'state'; id: string; type?: string; description?: string | string[]; descriptions?: string[]; doc?: StateStatement[]; note?: { position?: string; text: string }; start?: boolean; classes?: string[]; styles?: string[]; textStyles?: string[] }
export type StateStatement = string | StateDeclaration | { stmt: string; [key: string]: unknown };
export interface StateNode { id: string; shape?: string; label?: string; description: string[]; cssClasses: string; cssCompiledStyles: string[]; cssStyles: string[]; type?: string; isGroup?: boolean; parentId?: string; dir?: string; position?: string; noteFor?: string; colorIndex?: number }
export interface StateEdge { id: string; start: string; end: string; label: string; classes: string; pattern?: string }
export interface StateModel extends AccessibleModel { type: 'state'; direction: string; rootDoc: StateStatement[]; states: ReadonlyMap<string, StateDeclaration>; relations: { id1: string; id2: string; relationTitle?: string }[]; classes: ReadonlyMap<string, FlowClassDef>; links: ReadonlyMap<string, { url: string; tooltip: string }>; graph: { nodes: StateNode[]; edges: StateEdge[] } }
export interface GanttTask { id: string; task: string; section: string; startTime?: Date | null; endTime?: Date | null; renderEndTime: Date | null; manualEndTime: boolean; processed: boolean; active: boolean; done: boolean; crit: boolean; milestone: boolean; vert: boolean; order: number; classes: string[]; prevTaskId?: string; raw: { data: string; start?: string; end: string } }
export interface GanttModel extends AccessibleModel { type: 'gantt'; dateFormat: string; axisFormat: string; tickInterval?: string; todayMarker: string; includes: string[]; excludes: string[]; inclusiveEndDates: boolean; topAxis: boolean; displayMode: string; weekday?: string; weekend: string; sections: string[]; links: ReadonlyMap<string, string>; tasks: GanttTask[]; byId: ReadonlyMap<string, GanttTask>; currentSection: string; taskCount: number; lastOrder: number; lastTaskId?: string; options: Record<string, ConfigValue>; now: number; work: number; includeSet: Record<string, never>; excludeSet: Record<string, never>; excludedDays: boolean[]; datedTokens: boolean }
export interface SequenceBox { name?: string; wrap: boolean; fill: string; actorKeys: string[] }
export interface SequenceActor { box?: SequenceBox; name: string; description: string; wrap: boolean; prevActor?: string; nextActor?: string; links: ReadonlyMap<string, ConfigValue>; properties: ReadonlyMap<string, ConfigValue>; details?: string; type: string }
export interface SequenceNumbering { start?: number; step?: number; visible: boolean }
export interface SequenceMessage { id: string; from?: string; to?: string; message: string | SequenceNumbering; wrap: boolean; type?: number; activate?: boolean; placement?: number; centralConnection?: number; answer?: ConfigValue }
export interface SequenceNote { actor: string | string[]; placement: number; message: string; wrap: boolean }
export interface SequenceModel extends AccessibleModel { type: 'sequence'; actors: ReadonlyMap<string, SequenceActor>; createdActors: ReadonlyMap<string, number>; destroyedActors: ReadonlyMap<string, number>; boxes: SequenceBox[]; messages: SequenceMessage[]; notes: SequenceNote[] }
export interface GitCommit { id: string; message: string; seq: number; type: number; tags: string[]; parents: string[]; branch: string; customType?: number; customId?: boolean }
export interface GitModel extends AccessibleModel { type: 'gitGraph'; direction: string; commits: ReadonlyMap<string, GitCommit>; branches: ReadonlyMap<string, string | null>; lanes: string[]; currentBranch: string; head: GitCommit | null; warnings: string[] }
export interface ArchitectureNode { id: string; type: string; icon?: string; iconText?: string; title?: string; in?: string }
export interface ArchitectureGroup { id: string; icon?: string; title?: string; in?: string }
export interface ArchitectureEdge { lhsId: string; lhsDir: string; lhsInto: boolean; lhsGroup: boolean; rhsId: string; rhsDir: string; rhsInto: boolean; rhsGroup: boolean; title?: string }
export interface ArchitectureModel extends AccessibleModel { type: 'architecture'; nodes: ReadonlyMap<string, ArchitectureNode>; groups: ReadonlyMap<string, ArchitectureGroup>; edges: ArchitectureEdge[]; layoutHints: { direction: string; members: string[] }[]; registered: ReadonlyMap<string, string> }
export interface SankeyNode { id: string; index: number }
export interface SankeyModel extends AccessibleModel { type: 'sankey'; nodes: SankeyNode[]; links: { source: SankeyNode; target: SankeyNode; value: number }[] }
export interface TreemapNode { name: string; children?: TreemapNode[]; value?: number; classSelector?: string; cssCompiledStyles?: string[] }
export interface TreemapModel extends AccessibleModel { type: 'treemap'; roots: TreemapNode[]; classes: ReadonlyMap<string, string[]> }
export interface InfoModel extends AccessibleModel { type: 'info' }
export interface C4Named { key: string; value: string }
export interface C4Element { alias: string; label: string; aliasAttr?: C4Named; labelAttr?: C4Named; parent?: C4Element; type?: string; descr?: string; techn?: string; sprite?: string; tags?: string; link?: string; bgColor?: string; fontColor?: string; borderColor?: string; shadowing?: string; shape?: string; legendText?: string; legendSprite?: string; extra?: ReadonlyMap<string,string> }
export interface C4Relation { type: string; from: string; to: string; label: string; fromAttr?: C4Named; toAttr?: C4Named; labelAttr?: C4Named; techn?: string; descr?: string; sprite?: string; tags?: string; link?: string; textColor?: string; lineColor?: string; offsetX?: number; offsetY?: number; extra?: ReadonlyMap<string,string> }
export interface C4Model extends AccessibleModel { type: 'c4'; c4Type?: string; shapes: (C4Element & {kind:string})[]; boundaries: (C4Element & {nodeType?:string})[]; rels: C4Relation[]; shapeInRow: number; boundaryInRow: number }
export interface RequirementNode { name: string; type: string; requirementId: string; text: string; risk: string; verifyMethod: string; cssStyles: string[]; classes: string[] }
export interface RequirementElement { name: string; type: string; docRef: string; cssStyles: string[]; classes: string[] }
export interface RequirementModel extends AccessibleModel { type: 'requirement'; direction: string; requirements: ReadonlyMap<string, RequirementNode>; elements: ReadonlyMap<string, RequirementElement>; relations: {type:string;src:string;dst:string}[]; classes: ReadonlyMap<string, {id:string;styles:string[];textStyles:string[]}> }
export interface BlockNode { id?: string; type?: string; label?: string; children?: BlockNode[]; columns?: number; widthInColumns?: number; width?: number; directions?: string[]; classes?: string[]; styles?: string[]; colorIndex?: number; start?: string; end?: string; arrowTypeStart?: string; arrowTypeEnd?: string; thickness?: string; pattern?: string; css?: string; styleClass?: string; stylesStr?: string }
export interface BlockModel { type: 'block'; title?: string; root: BlockNode; blocks: BlockNode[]; edges: BlockNode[]; classes: ReadonlyMap<string, {id:string;styles:string[];textStyles:string[]}>; byId: ReadonlyMap<string, BlockNode> }
export type RailroadNode = {type:'terminal';value:string} | {type:'nonterminal';name:string} | {type:'special';text:string} | {type:'sequence';elements:RailroadNode[]} | {type:'choice';alternatives:RailroadNode[]} | {type:'optional';element:RailroadNode} | {type:'repetition';element:RailroadNode;min:number;max:number|null};
export interface RailroadModel extends AccessibleModel { type:'railroad';notation:string;rules:{name:string;definition:RailroadNode}[] }
export type CynefinDomainName = 'complex'|'complicated'|'clear'|'chaotic'|'confusion';
export interface CynefinModel extends AccessibleModel { type:'cynefin';domains:ReadonlyMap<CynefinDomainName,{name:CynefinDomainName;items:{label:string}[]}>;transitions:{from:CynefinDomainName;to:CynefinDomainName;label?:string}[] }
export interface TreeViewNode { id:number;level:number;name:string;nodeType:'file'|'directory';icon?:string;cssClass?:string;description?:string;children:TreeViewNode[] }
export interface TreeViewModel extends AccessibleModel { type:'treeView';root:TreeViewNode;count:number }
export interface AgentNode extends FlowNode { metadata?:{[key:string]:ConfigValue} }
export interface AgentflowModel extends AccessibleModel { type:'agentflow';direction?:string;nodes:ReadonlyMap<string,AgentNode>;connectors:ReadonlyMap<string,AgentNode>;edges:FlowEdge[];subgraphs:{id:string;type:string;nodes:string[];title:string;classes:string[];dir?:string;labelType:string;metadata?:{[key:string]:ConfigValue}}[];classes:ReadonlyMap<string,{id:string;styles:string[];textStyles:string[]}>;tooltips:ReadonlyMap<string,string>;mappings:ConfigValue[];diagnostics:ConfigValue[];lineOffset:number;defaultEdgeStyle?:string[];defaultInterpolate?:ConfigValue }
export interface WardleyNode { id:string;label:string;x:number;y:number;className?:string;labelOffsetX?:number;labelOffsetY?:number;inPipeline?:boolean;isPipelineParent?:boolean;inertia?:boolean;sourceStrategy?:string }
export interface WardleyPoint { x:number;y:number }
export interface WardleyModel extends AccessibleModel { type:'wardley';nodes:WardleyNode[];links:{source:string;target:string;dashed:boolean;label?:string;flow?:string}[];trends:{nodeId:string;targetX:number;targetY:number}[];pipelines:{nodeId:string;componentIds:string[]}[];annotations:{number:number;coordinates:WardleyPoint[];text?:string}[];notes:({text:string}&WardleyPoint)[];accelerators:({name:string}&WardleyPoint)[];deaccelerators:({name:string}&WardleyPoint)[];annotationsBox?:WardleyPoint;axes:{xLabel?:string;yLabel?:string;stages?:string[];stageBoundaries?:number[]};size?:{width:number;height:number} }
export interface EventLane { index:number;label:string;group:'trigger'|'state'|'stream';namespace?:string }
export interface EventBox { id:string;kind:'ui'|'processor'|'command'|'readmodel'|'event';entity:string;name:string;namespace?:string;reset:boolean;lane:EventLane;data?:string;dataType?:string }
export interface EventModelingModel extends AccessibleModel { type:'eventmodeling';boxes:EventBox[];lanes:EventLane[];relations:{source:EventBox;target:EventBox}[];notes:{frame:string;text:string;dataType?:string}[];specifications:{frame:string;given:string[];when:string[];then:string[]}[];entities:string[] }
export type SourceSpan = [number,number];
export interface UsecaseActor { id:string;label:string;labelType:string;type:string;icon?:string;business:boolean;stereotype?:string;parentId?:string;classes:string[];styles:string[] }
export interface UseCase { id:string;label:string;labelType:string;shape:string;business:boolean;stereotype?:string;parentId?:string;classes:string[];styles:string[] }
export interface UsecaseModel { type:'usecase';actors:ReadonlyMap<string,UsecaseActor>;useCases:ReadonlyMap<string,UseCase>;systemBoundaries:ReadonlyMap<string,{id:string;label:string;labelType:string;type:string;members:string[];classes:string[];styles:string[]}>;relationships:{id:string;explicitId:boolean;source:string;target:string;type:string;arrowType:number;label?:string;labelType?:string;minlen:number;classes:string[];styles:string[];animate:boolean;animation?:string}[];notes:ReadonlyMap<string,{id:string;target:string;label:string;labelType:string}>;jsonNodes:ReadonlyMap<string,{id:string;value:ConfigValue;propertyOrder:ReadonlyMap<string,string[]>;classes:string[];styles:string[]}>;classDefs:ReadonlyMap<string,{id:string;styles:string[]}>;direction:string;title?:string;accTitle:string;accDescr:string;accDescription:string;source:string;headerSpan:SourceSpan;statements:ConfigValue[] }
export interface VennModel { type:'venn';title?:string;subsets:{sets:string[];size:number;label?:string}[];textNodes:{sets:string[];id:string;label?:string}[];styles:{targets:string[];styles:ReadonlyMap<string,string>}[] }
export interface IshikawaNode { text:string;children:IshikawaNode[] }
export interface IshikawaModel { type:'ishikawa';title?:string;root?:IshikawaNode }
export type DiagramModel = VennModel | IshikawaModel | PieModel | PacketModel | RadarModel | QuadrantModel | TimelineModel | JourneyModel | FlowchartModel | KanbanModel | MindmapModel | ClassModel | ErModel | XyChartModel | StateModel | GanttModel | SequenceModel | GitModel | ArchitectureModel | SankeyModel | TreemapModel | C4Model | InfoModel | RequirementModel | BlockModel | RailroadModel | CynefinModel | TreeViewModel | AgentflowModel | WardleyModel | EventModelingModel | UsecaseModel;
export interface Rendered { type: DiagramId; svg: string; width: number; height: number; links: { id: string; href: string; internal: boolean }[] }
export interface Renderer {
  capabilities(): Capability[];
  parse(source: string, options?: ParseOptions): DiagramModel;
  render(input: string | DiagramModel, options?: RenderOptions): Rendered;
  query(model: DiagramModel, operation: string, argumentsValue?: ConfigValue): ConfigValue;
  dispose(model: DiagramModel): void;
}
export function loadRenderer(moduleOrPath?: string | URL | Response | BufferSource | WebAssembly.Module): Promise<Renderer>;

export interface TextMeasurer { width(text: string, size: number, style: number): number }
export const metricsMeasurer: TextMeasurer;
export function canvasMeasurer(fontFamily?: string, monoFamily?: string): TextMeasurer | null;
export function forgetTextWidths(): void;
