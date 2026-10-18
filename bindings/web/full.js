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
        if (projection.type === 'state') for (const key of ['states', 'classes', 'links']) model[key] = new Map(model[key]);
        if (projection.type === 'gantt') {
          model.links = new Map(model.links);
          model.byId = new Map(model.byId);
          for (const task of [...model.tasks, ...model.byId.values()]) for (const key of ['startTime', 'endTime', 'renderEndTime']) if (typeof task[key] === 'string') task[key] = new Date(task[key]);
        }
        if (projection.type === 'sequence') {
          for (const key of ['actors', 'createdActors', 'destroyedActors']) model[key] = new Map(model[key]);
          for (const actor of model.actors.values()) for (const key of ['links', 'properties']) actor[key] = new Map(actor[key]);
          for (const box of model.boxes) for (const id of box.actorKeys) { const actor = model.actors.get(id); if (actor) actor.box = box; }
        }
        if (projection.type === 'er') for (const key of ['entities', 'classes']) model[key] = new Map(model[key]);
        if (projection.type === 'class') {
          const namespaces = values => new Map(values.map(([id, value]) => [id, {...value, classes: new Map(value.classes), notes: new Map(value.notes), children: namespaces(value.children)}]));
          for (const key of ['classes', 'notes', 'styleClasses', 'withCssClass']) model[key] = new Map(model[key]);
          model.namespaces = namespaces(model.namespaces);
        }
        if (projection.type === 'flowchart') for (const key of ['nodes', 'classes', 'tooltips']) model[key] = new Map(model[key]);
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
