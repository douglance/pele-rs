export interface SourceSpan { start: number; end: number; line: number; column: number }
export class PeleError extends Error { code: string; span?: SourceSpan }
export interface Theme { background?: string; foreground?: string; surface?: string; border?: string; line?: string }
export interface RenderOptions {
  theme?: Theme; font_size?: number; edge_font_size?: number; node_gap?: number;
  rank_gap?: number; padding?: number; node_padding?: number; radius?: number;
  stroke_width?: number; title?: string;
}
export interface Point { x: number; y: number }
export interface Rect extends Point { width: number; height: number }
export interface DiagramNode { id: string; label: string; shape: 'rectangle' | 'rounded' | 'diamond' | 'circle' | 'stadium'; bounds: Rect }
export interface DiagramEdge {
  index: number; source: string; target: string; label: string;
  style: 'solid' | 'dashed' | 'thick'; arrow: boolean; points: Point[]; label_bounds?: Rect;
}
export interface RenderedDiagram {
  svg: string; width: number; height: number;
  geometry: { version: 1; direction: 'TB' | 'BT' | 'LR' | 'RL'; nodes: DiagramNode[]; edges: DiagramEdge[] };
}
export interface Pele { render(source: string, options?: RenderOptions): RenderedDiagram }
export function loadPele(moduleOrPath?: string | URL | Response | BufferSource | WebAssembly.Module): Promise<Pele>;
