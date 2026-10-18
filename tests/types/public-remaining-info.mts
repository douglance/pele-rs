import {
  loadRenderer, type DiagramModel, type C4Shape, type C4Boundary, type C4Attr,
  type UsecaseGraphStatement, type CynefinDomainName, type WardleyNode,
  type VennStyle, type RenderOptions,
} from '../../bindings/web/full.js';

// Compile-only consumer integration: public entry, discriminated returned models,
// concrete mutable collections, actual callable methods and Date render options.
export async function consumeRemainingInfo(source: string): Promise<string> {
  const renderer = await loadRenderer();
  const model: DiagramModel = renderer.parse(source);
  switch (model.type) {
    case 'c4': {
      const attrs: C4Attr[] = ['api', 'API', {key: 'techn', value: 'Rust'}];
      model.setC4Type('C4Container'); model.setTitle('Typed C4'); model.setAccDescription('Description');
      model.addPersonOrSystem('person', ['user', 'User']);
      model.addContainer('container', attrs); model.addComponent('component', ['worker', 'Worker', 'Rust']);
      const shape: C4Shape = model.element(model.shapes, model.shapeIndex, 'helper', 'Helper');
      model.finishShape(shape, 'system', ['helper', 'Helper', 'Service', 'sprite'], 3);
      const boundary: C4Boundary = model.boundary(['group', 'Group'], 'system');
      model.open(boundary); model.popBoundaryParseStack();
      model.addPersonOrSystemBoundary(['team', 'Team']); model.popBoundaryParseStack();
      model.addContainerBoundary(['containers', 'Containers']); model.popBoundaryParseStack();
      model.addDeploymentNode('nodeL', ['deployment', 'Deployment']); model.popBoundaryParseStack();
      model.addRel('rel', ['user', 'api', 'Uses']);
      model.updateElStyle('ignored', ['api', 'gold']);
      model.updateRelStyle('ignored', ['user', 'api', 'purple', 'green', '12', '-3']);
      model.updateLayoutConfig('ignored', ['3', '2']);
      const found = model.getC4Shape('api');
      if (found) {found.extra ??= new Map(); found.extra.set('consumer', 'web'); found.parent = boundary;}
      model.shapeIndex.set('helper', shape); model.boundaryIndex.set('group', boundary);
      const relation = model.rels[0];
      if (relation) {
        relation.extra ??= new Map(); relation.extra.set('consumer', 'web');
        model.relIndex.set(relation.from, new Map([[relation.to, relation]]));
      }
      model.stack.push(model.current); model.current = boundary; model.stack.pop();
      break;
    }
    case 'usecase': {
      model.actors.set('user', {id: 'user', label: 'User', labelType: 'text', type: 'normal', business: false, classes: [], styles: []});
      model.useCases.set('login', {id: 'login', label: 'Login', labelType: 'markdown', shape: 'ellipse', business: false, classes: [], styles: []});
      model.systemBoundaries.set('app', {id: 'app', label: 'App', labelType: 'text', type: 'rect', members: ['login'], classes: [], styles: []});
      model.relationships.push({id: 'edge', explicitId: true, source: 'user', target: 'login', type: 'association', arrowType: 0, minlen: 1, classes: [], styles: [], animate: true, animation: 'slow'});
      model.notes.set('note', {id: 'note', target: 'login', label: 'Note', labelType: 'text'});
      model.jsonNodes.set('json', {id: 'json', value: {name: 'value'}, propertyOrder: new Map([['', ['name']]]), classes: [], styles: []});
      const json = model.jsonNodes.get('json');
      if (json) {json.value.name = 'edited'; json.propertyOrder.set('/child', ['id']);}
      model.classDefs.set('active', {id: 'active', styles: ['fill:gold']});
      const statement: UsecaseGraphStatement = {kind: 'node', span: [0, 4], nodes: [{id: 'user', span: [0, 4], idSpan: [0, 4], metadata: [{key: 'label', span: [0, 4], keySpan: [0, 1], valueSpan: [1, 4]}]}]};
      model.statements.push(statement); model.direction = 'LR'; model.accDescription = 'Raw description';
      break;
    }
    case 'cynefin': {
      const name: CynefinDomainName = 'complex';
      model.domains.set(name, {name, items: [{label: 'Practice'}]});
      model.domains.get(name)?.items.push({label: 'Another'});
      model.transitions.push({from: name, to: 'clear', label: 'Learn'});
      model.domains = new Map([...model.domains].reverse());
      model.title = 'Typed decisions'; model.accTitle = 'Accessible';
      break;
    }
    case 'wardley': {
      const node: WardleyNode = {id: 'service', label: 'Service', sourceStrategy: 'build'};
      model.nodes.push(node); node.x = 45; node.y = 65;
      model.links.push({source: 'service', target: 'other', flow: 'forward'});
      model.trends.push({nodeId: 'service', targetX: 80, targetY: 65});
      model.pipelines.push({nodeId: 'service', componentIds: ['other']});
      model.annotations.push({number: 1, coordinates: [{x: 45, y: 65}], text: 'Review'});
      model.notes.push({text: 'Note', x: 20, y: 30});
      model.accelerators.push({name: 'Force', x: 20, y: 30});
      model.deaccelerators.push({name: 'Drag', x: 50, y: 60});
      model.axes = {xLabel: 'Evolution', yLabel: 'Visibility', stages: ['Genesis', 'Custom'], stageBoundaries: [.5, 1]};
      model.annotationsBox = {x: 70, y: 20}; model.size = {width: 800, height: 600};
      break;
    }
    case 'venn': {
      model.setDiagramTitle('Typed Venn');
      model.addSubsetData(['A'], 'Alpha', 12); model.addSubsetData(['B'], undefined, undefined);
      model.addTextData(['A'], 'note', 'Text'); model.addStyleData(['A'], [['fill', 'gold']]);
      model.validateUnionIdentifiers(['A', 'B']); model.setIndentMode(true);
      const current: string[] | undefined = model.getCurrentSets();
      if (current) current.push('B');
      const indentation: boolean = model.getIndentMode(); model.indent = indentation;
      model.known.add('C'); model.current = ['C'];
      const style: VennStyle = {targets: ['C'], styles: new Map([['color', 'purple']])};
      style.styles.set('fill', 'gold'); model.styles.push(style);
      break;
    }
  }
  const dated: RenderOptions = {now: new Date(1704067200000.875), outputLimit: Infinity};
  const result = renderer.render(model, dated);
  renderer.render(model, {now: NaN, padding: -0});
  renderer.dispose(model);
  return result.svg;
}
