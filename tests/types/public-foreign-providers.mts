import {loadRegistry, type Diagram} from '../../bindings/web/full.js';
class Model { width = 40; entries = new Map<string, number>(); }
const diagram: Diagram<Model> = {
  type: 'pie', section: 'pie', keepComments: true,
  parse(source, config, title, lineOffset, limits) {
    const limit: number = limits.maxEdges;
    void [source, config, title, lineOffset, limit];return new Model();
  },
  render(model, config, options) {
    const width: number = options.measurer?.width('custom', 13, 0) ?? model.width;
    void config;return {svg: '<svg/>', width, height: 10, links: []};
  }
};
const registry = await loadRegistry();registry.register(diagram);
const model = registry.parse<Model>('pie');model.entries.set('A', 1);
registry.render(model);registry.dispose(model);registry.close();
