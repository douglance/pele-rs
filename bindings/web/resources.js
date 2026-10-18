// Browser font resource adapter, Pele text/measurer.ts at 458322a5 (MIT, kepano).
// Layout and fallback advances remain Rust. Canvas supplies only host font measurements.
import { measure_text } from './pkg/pele.js';
export const metricsMeasurer = { width(text, size, style) { return measure_text(text, size, style); } };
const shared = new Map();
function fontSet() { return globalThis.document?.fonts ?? globalThis.fonts; }
export function canvasMeasurer(fontFamily = 'sans-serif', monoFamily = 'monospace') {
  let context;
  try { context = globalThis.OffscreenCanvas ? new OffscreenCanvas(1, 1).getContext('2d') : globalThis.document?.createElement('canvas').getContext('2d'); }
  catch { return null; }
  if (!context) return null;
  const caches = new Map(); let currentFont = '', faces = fontSet()?.size;
  return { width(text, size, style) {
    const fonts = fontSet(); if (fonts?.size !== faces) { faces = fonts?.size; caches.clear(); }
    const font = (style & 2 ? 'italic ' : '') + (style & 1 ? 'bold ' : '') + size + 'px ' + (style & 4 ? monoFamily : fontFamily);
    let cache = caches.get(font); if (!cache) caches.set(font, cache = new Map());
    let width = cache.get(text);
    if (width === undefined) {
      if (font !== currentFont) context.font = currentFont = font;
      width = context.measureText(text).width;
      let loaded = true; try { loaded = fonts?.check(font, text) ?? true; } catch {}
      if (loaded) { if (cache.size >= 20000) cache.clear(); cache.set(text, width); }
    }
    return width;
  } };
}
export function defaultMeasurer(fontFamily = 'sans-serif', monoFamily = 'monospace') {
  const key = fontFamily + '\n' + monoFamily;
  let measurer = shared.get(key);
  if (!measurer) shared.set(key, measurer = canvasMeasurer(fontFamily, monoFamily) ?? metricsMeasurer);
  return measurer;
}
export function forgetTextWidths() { shared.clear(); }
