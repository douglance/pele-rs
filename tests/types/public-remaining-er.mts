import {
  loadRenderer, type DiagramModel, type GanttTask, type SequenceActor,
  type SequenceItem, type RequirementNode, type BlockNode, type RailroadNode,
  type RailroadRule,
} from '../../bindings/web/full.js';

// Consumer feature slice: narrow the public union, use real mutable collections
// and native builders, retain returned aliases and schedule with Date or number.
async function consumeRemaining(source: string): Promise<string> {
  const renderer = await loadRenderer();
  const model: DiagramModel = renderer.parse(source);
  switch (model.type) {
    case 'gantt': {
      model.setDateFormat('YYYY-MM-DD'); model.setAxisFormat('%Y-%m-%d');
      model.setIncludes('2024-01-06'); model.setExcludes('weekends');
      model.setWeekend('saturday'); model.setWeekday('monday');
      model.enableInclusiveEndDates(); model.TopAxis();
      model.addSection('Delivery'); model.addTask('Compile', ':native,2024-01-01,1d');
      model.setLink('native', 'https://example.com'); model.setClass('native', 'active');
      const tasks: GanttTask[] = model.getTasks(new Date(1704067200000));
      const repeated: GanttTask[] = model.getTasks(1704067200000.5);
      const task = model.byId.get('native');
      if (task) {
        model.byId.set('alias', task); model.tasks = [task];
        const start: Date | undefined = task.startTime;
        if (start) model.isInvalidDate(start);
        const parsed: Date | undefined = model.parse('2024-01-02', 'YYYY-MM-DD');
        if (parsed) model.checked(parsed, '2024-01-02');
      }
      model.includeSet.add('2024-01-06'); model.excludeSet.delete('sunday');
      model.links = new Map(model.links); model.byId.delete('alias');
      model.setDiagramTitle(String(tasks.length + repeated.length));
      break;
    }
    case 'sequence': {
      model.addBox(model.parseBoxData('pink Clients'));
      model.addActor('A', 'A', {text: 'Client', wrap: true}, 'participant');
      model.addActor('B', 'B', {text: 'Server'}, 'participant');
      const accepted: boolean = model.addSignal('A', 'B', model.parseMessage('Request'), 0);
      model.addMessage('B', 'A', {text: 'Reply'}, {custom: accepted});
      model.addNote(['A', 'B'], 2, {text: 'Native API'});
      model.addALink('A', {text: 'Docs @ https://example.com'});
      model.addLinks('A', {text: '{"Home":"https://example.com"}'});
      model.addProperties('A', {text: '{"kind":"client"}'});
      model.addDetails('A', {text: 'panel-id'});
      const statements: SequenceItem = [[{type: 'boxEnd'}], 'ignored'];
      model.apply(statements); model.applyOne({type: 'boxEnd'});
      const actor: SequenceActor | undefined = model.actors.get('A');
      if (actor) {actor.links.set('custom', {deep: ['value']}); model.actors.set('alias', actor);}
      model.createdActors.set('A', 1); model.destroyedActors.delete('A');
      model.active.set('A', 0); model.actors = new Map(model.actors);
      model.setAccTitle(String(model.autoWrap()));
      break;
    }
    case 'requirement': {
      model.setDirection('LR'); model.setNewReqId('native'); model.setNewReqText('Native builder');
      model.setNewReqRisk('High'); model.setNewReqVerifyMethod('Test');
      const requirement: RequirementNode = model.addRequirement('A', 'Requirement');
      model.requirements.set('alias', requirement);
      model.setNewElementType('service'); model.setNewElementDocRef('docs/service');
      const element = model.addElement('E'); model.elements.set('E', element);
      model.addRelationship('satisfies', 'E', 'A');
      model.defineClass(['active'], ['fill:red']); model.setClass(['A'], ['active']);
      model.setCssStyle(['A'], ['stroke:blue']); model.copy(requirement.cssStyles, ['opacity:.8']);
      model.node('A')?.classes.push('consumer');
      model.classes = new Map(model.classes); model.requirements.delete('alias');
      model.setAccDescription(model.newText);
      break;
    }
    case 'block': {
      model.addStyleClass('active', 'fill:red,color:white'); model.addStyle2Node('A', 'stroke:blue');
      model.setCssClass('A', 'active');
      const node: BlockNode = {id: 'A', type: model.typeStr2Type('[]'), label: 'Native'};
      model.setHierarchy([{id: 'columns', type: 'column-setting', columns: 2}, [node]]);
      model.byId.set('alias', model.named('A')); model.byId.delete('alias');
      model.classes = new Map(model.classes); model.pending.add('later'); model.edgeCount.set('A-B', 1);
      const arrow: string = model.edgeStrToEdgeData('-->');
      model.edgeStrToEdgeStartData('<--'); model.edgeStrToThickness('==>'); model.edgeStrToPattern('-.->');
      model.title = `${model.generateId()} ${arrow}`;
      break;
    }
    case 'railroad': {
      const repeated: RailroadNode = {type: 'repetition', element: {type: 'terminal', value: 'x'}, min: 1, max: Infinity};
      const rule: RailroadRule = {name: 'native', definition: {type: 'sequence', elements: [repeated, {type: 'nonterminal', name: 'other'}]}};
      model.addRule(rule); model.byName.set('alias', rule);
      const shared: RailroadRule | undefined = model.getRule('native');
      if (shared) model.rules = [shared];
      model.byName = new Map(model.byName); model.byName.delete('alias');
      model.notation = 'railroad'; model.setTitle('Grammar');
      model.setAccTitle('Native grammar'); model.setAccDescription('Mutable rules');
      break;
    }
  }
  const rendered = renderer.render(model);
  renderer.dispose(model);
  return rendered.svg;
}
void consumeRemaining;
