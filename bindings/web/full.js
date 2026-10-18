import initialize, { parse_v2, render_with_resources, builtin_capabilities } from './pkg/pele.js';
import { PeleCompatibilityError } from './core.js';
import { defaultMeasurer } from './resources.js';
export { canvasMeasurer, metricsMeasurer, forgetTextWidths } from './resources.js';
function result(json) {
  const envelope = JSON.parse(json);
  if (envelope.version !== 2) throw new Error('Unsupported compatibility contract');
  if (!envelope.ok) throw new PeleCompatibilityError(envelope.error);
  return envelope.result;
}
/** Implemented built-ins. Core rendering remains Rust; this adapter owns model lifetimes only. */
export async function loadRenderer(moduleOrPath) {
  await initialize(moduleOrPath === undefined ? undefined : { module_or_path: moduleOrPath });
  const documents = new WeakMap();
  const renderer = {
    capabilities() { return result(builtin_capabilities()); },
    parse(source, options = {}) {
      let document;
      try { document = parse_v2(source, JSON.stringify(options)); }
      catch (error) { if (typeof error === 'string') return result(error); throw error; }
      try {
        const projection = result(document.model());
        const model = projection.model;
        if (projection.type === 'pie') model.sections = new Map(model.sections);
        if (projection.type === 'quadrantChart') model.classes = new Map(model.classes);
        documents.set(model, document);
        return model;
      } catch (error) { document.free(); throw error; }
    },
    render(input, options = {}) {
      const measurer = options.measurer ?? defaultMeasurer(options.fontFamily);
      let callbackError;
      const width = (text, size, style) => { try { return measurer.width(text, size, style); } catch (e) { callbackError = e; throw e; } };
      const icons = options.icons && (name => { try { return options.icons(name); } catch (e) { callbackError = e; throw e; } });
      let json;
      if (typeof input === 'string') json = render_with_resources(input, JSON.stringify(options), width, icons);
      else {
        const document = documents.get(input);
        if (!document) throw new TypeError('render() takes diagram text or a model that parse() returned.');
        json = document.render_with_resources(JSON.stringify(options), width, icons);
      }
      if (callbackError) {
        if (callbackError instanceof RangeError) throw new PeleCompatibilityError({name:'PeleError',message:'Diagram is nested too deeply.',code:'limit',type:null,line:0,column:0,snippet:''});
        throw callbackError;
      }
      return result(json);
    },
    dispose(model) {
      const document = documents.get(model);
      if (document) { documents.delete(model); document.free(); }
    }
  };
  return renderer;
}
