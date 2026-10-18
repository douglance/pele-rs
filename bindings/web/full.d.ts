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
export interface Rendered { type: DiagramId; svg: string; width: number; height: number; links: { id: string; href: string; internal: boolean }[] }
export interface Renderer {
  capabilities(): Capability[];
  parse(source: string, options?: ParseOptions): PieModel;
  render(input: string | PieModel, options?: RenderOptions): Rendered;
  dispose(model: PieModel): void;
}
export function loadRenderer(moduleOrPath?: string | URL | Response | BufferSource | WebAssembly.Module): Promise<Renderer>;

export interface TextMeasurer { width(text: string, size: number, style: number): number }
export const metricsMeasurer: TextMeasurer;
export function canvasMeasurer(fontFamily?: string, monoFamily?: string): TextMeasurer | null;
export function forgetTextWidths(): void;
