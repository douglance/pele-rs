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
export type DiagramModel = PieModel | PacketModel | RadarModel | QuadrantModel | TimelineModel | JourneyModel | FlowchartModel | KanbanModel | MindmapModel;
export interface Rendered { type: DiagramId; svg: string; width: number; height: number; links: { id: string; href: string; internal: boolean }[] }
export interface Renderer {
  capabilities(): Capability[];
  parse(source: string, options?: ParseOptions): DiagramModel;
  render(input: string | DiagramModel, options?: RenderOptions): Rendered;
  dispose(model: DiagramModel): void;
}
export function loadRenderer(moduleOrPath?: string | URL | Response | BufferSource | WebAssembly.Module): Promise<Renderer>;

export interface TextMeasurer { width(text: string, size: number, style: number): number }
export const metricsMeasurer: TextMeasurer;
export function canvasMeasurer(fontFamily?: string, monoFamily?: string): TextMeasurer | null;
export function forgetTextWidths(): void;
