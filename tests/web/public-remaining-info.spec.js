import {test, expect} from '@playwright/test';

// Public feature slices exercise the packed consumer package and pinned Pele,
// including retained native dispatch. No family implementation runs in JS here.
async function exercise({family}) {
  const {loadRenderer} = await import('/dist/package-web/package/full.js');
  const oracle = await import('/dist/evidence/pele-oracle.js');
  const engine = await loadRenderer(), failures = [], intrinsicErrors = [], now = 1704067200000;
  const sources = {
    c4: 'C4Context\nPerson(user,"Customer")\nSystem_Boundary(bank,"Bank") {\nSystem(app,"Banking")\nContainerDb(store,"Ledger","SQL")\n}\nRel(user,app,"Uses","HTTPS")\nRel(app,store,"Reads")',
    usecase: 'usecase-beta\ndirection LR\nactor Customer("Customer")\nsystemBoundary "Order system"\n  Checkout("Place order")\nend\nCustomer --> Checkout',
    cynefin: 'cynefin-beta\n  complex\n    "Emergent practice"\n  clear\n    "Best practice"',
    wardley: 'wardley-beta\ntitle Dependency map\nevolution Genesis -> Custom -> Product -> Commodity\ncomponent App [0.8, 0.3]\ncomponent Core [0.4, 0.7]\nApp -> Core',
    venn: 'venn-beta\nset A["Alpha"]:20\nset B["Beta"]:14\nunion A,B["Both"]:3',
  };
  const source = sources[family], model = engine.parse(source), reference = oracle.parse(source);
  const second = engine.parse(source);
  const check = (condition, stage) => {if (!condition) failures.push(`${family}: ${stage}`);};
  const bits = number => {
    const buffer = new ArrayBuffer(8), view = new DataView(buffer);
    view.setFloat64(0, number, false);
    return view.getBigUint64(0, false).toString(16).padStart(16, '0');
  };
  // Breadth-first, identity-aware complete reachable graph. Properties preserve
  // own order/presence/flags; numbers and Dates retain IEEE bits; cycles stay refs.
  const canonical = roots => {
    const seen = new Map(), pending = [], nodes = [];
    const atom = value => {
      if (value === undefined) return ['undefined'];
      if (value === null) return ['null'];
      if (typeof value === 'number') return ['number', bits(value)];
      if (typeof value === 'function') return ['function', value.length, Reflect.ownKeys(value).filter(k => k !== 'name')];
      if (typeof value !== 'object') return [typeof value, value];
      if (!seen.has(value)) {seen.set(value, pending.length); pending.push(value);}
      return ['ref', seen.get(value)];
    };
    const root = roots.map(atom);
    for (let index = 0; index < pending.length; index++) {
      const value = pending[index], prototype = Object.getPrototypeOf(value);
      const members = prototype && ![Object.prototype, Array.prototype, Map.prototype, Set.prototype, Date.prototype].includes(prototype)
        ? Reflect.ownKeys(prototype).filter(k => k !== 'constructor').map(key => {
          const d = Object.getOwnPropertyDescriptor(prototype, key);
          return [key, d.enumerable, d.configurable, d.writable, typeof d.value === 'function' ? d.value.length : null];
        }) : [];
      const properties = Reflect.ownKeys(value).map(key => {
        const d = Object.getOwnPropertyDescriptor(value, key);
        return [key, d.enumerable, d.configurable, d.writable, atom(d.value)];
      });
      const kind = value instanceof Map ? 'Map' : value instanceof Set ? 'Set' : value instanceof Date ? 'Date' : Array.isArray(value) ? 'Array' : prototype === null ? 'NullObject' : 'Object';
      const content = kind === 'Map' ? [...value].map(([k, v]) => [atom(k), atom(v)])
        : kind === 'Set' ? [...value].map(atom) : kind === 'Date' ? bits(+value) : null;
      nodes.push([kind, members, properties, content]);
    }
    return JSON.stringify([root, nodes]);
  };
  const compareGraph = (stage, actualReturn, expectedReturn) => {
    check(canonical([model, actualReturn]) === canonical([reference, expectedReturn]), `complete graph ${stage}`);
  };
  const outcome = operation => {
    try {return {value: operation()};}
    catch (error) {
      // Error line/column/stack are host VM diagnostics. PeleError locations are
      // library fields and remain part of every strict comparison.
      return {error: {name: error?.name, message: error?.message, ...(error?.name === 'PeleError' ? {code:error.code, type:error.type, line:error.line, column:error.column, snippet:error.snippet} : {})}};
    }
  };
  // Only these named invalid mutations have measured VM-specific intrinsic
  // diagnostics. Raw native messages stay pinned; live oracle text is attached.
  const intrinsicMessages = {
    "c4.addPersonOrSystem.label-null": "Cannot destructure property 'key' of 't' as it is null.",
    "venn.addSubsetData.subsets-null": "Cannot read properties of null (reading 'push')",
  };
  const both = (name, argumentsFor = () => [], intrinsicVector = undefined) => {
    const actual = outcome(() => model[name](...argumentsFor(model)));
    const expected = outcome(() => reference[name](...argumentsFor(reference)));
    if (intrinsicVector !== undefined) {
      check(Object.hasOwn(intrinsicMessages, intrinsicVector), 'known intrinsic vector '+intrinsicVector);
      check(JSON.stringify(actual.error) === JSON.stringify({name:'TypeError', message:intrinsicMessages[intrinsicVector]}), 'pinned native intrinsic '+intrinsicVector+': '+JSON.stringify(actual.error));
      check(expected.error?.name === 'TypeError', 'pinned live intrinsic class '+intrinsicVector);
      intrinsicErrors.push({vector:intrinsicVector, method:name, native:actual.error, pinned:expected.error});
    } else {
      check(JSON.stringify(actual.error) === JSON.stringify(expected.error), 'method error '+name+': '+JSON.stringify({native:actual.error,pinned:expected.error}));
    }
    compareGraph('method '+name, actual.value, expected.value);
    return actual;
  };
  const mutate = mutation => {mutation(model); mutation(reference);};
  const compareRender = (stage, additional = {}) => {
    const traces = [[], []];
    const options = index => ({now, ...additional, measurer: {width(text, size, style) {
      const width = text.length * size * (style & 1 ? .71 : .6);
      traces[index].push([text, bits(size), style, bits(width)]); return width;
    }}});
    const actual = outcome(() => engine.render(model, options(0)));
    const expected = outcome(() => oracle.render(reference, options(1)));
    check(!actual.error && !expected.error, `successful render ${stage}: ${JSON.stringify([actual.error, expected.error])}`);
    check(JSON.stringify(actual) === JSON.stringify(expected), `whole SVG/result ${stage}`);
    check(JSON.stringify(traces[0]) === JSON.stringify(traces[1]), `measurement order ${stage}`);
    compareGraph(`post-render ${stage}`);
  };
  const compareOutputFailure = stage => {
    const measurer = {width: (text, size, style) => text.length * size * (style & 1 ? .71 : .6)};
    const actual = outcome(() => engine.render(model, {now, outputLimit: 1, measurer}));
    const expected = outcome(() => oracle.render(reference, {now, outputLimit: 1, measurer}));
    check(actual.error?.code === 'limit' && JSON.stringify(actual.error) === JSON.stringify(expected.error), `output failure ${stage}`);
    compareGraph(`published output failure ${stage}`);
  };
  const prototype = Object.getPrototypeOf(model), expectedPrototype = Object.getPrototypeOf(reference);
  const instance = ['c4', 'venn'].includes(family);
  check((prototype !== Object.prototype) === instance, 'actual prototype kind');
  if (instance) for (const name of Reflect.ownKeys(expectedPrototype).filter(key => key !== 'constructor')) {
    const actual = Object.getOwnPropertyDescriptor(prototype, name), expected = Object.getOwnPropertyDescriptor(expectedPrototype, name);
    check(!!actual && actual.enumerable === expected.enumerable && actual.configurable === expected.configurable && actual.writable === expected.writable && actual.value.length === expected.value.length, `descriptor ${String(name)}`);
    check(model[name] === second[name], `shared native function ${String(name)}`);
    check(JSON.stringify(Reflect.ownKeys(model[name])) === JSON.stringify(Reflect.ownKeys(reference[name])), `function own keys ${String(name)}`);
  }
  compareGraph('parsed own fields and private state');
  compareRender('initial');

  if (family === 'c4') {
    const original = model.shapes[0];
    check(model.shapeIndex.get('user') === original, 'shape array/index alias');
    const found = both('getC4Shape', () => ['user']);
    check(found.value === original, 'getC4Shape aliases existing record');
    both('setC4Type', () => ['C4Dynamic']);
    both('setTitle', () => ['Method diagram']);
    both('setAccDescription', () => ['First\n   Second\n\n end']);
    both('addPersonOrSystem', () => ['external_person', ['guest', 'Guest', 'Outside']]);
    both('addPersonOrSystem', () => ['person', ['user', 'Edited user']]);
    both('addContainer', () => ['container_queue', ['queue', 'Queue', 'AMQP', 'Jobs']]);
    both('addComponent', () => ['component', ['service', 'Service', 'Rust', 'Worker']]);
    both('addPersonOrSystemBoundary', () => [['team', 'Team']]);
    both('addPersonOrSystem', () => ['system', ['inside', 'Inside']]);
    both('popBoundaryParseStack');
    both('addContainerBoundary', () => [['container', 'Container']]);
    both('addComponent', () => ['component', ['child', 'Child']]);
    both('popBoundaryParseStack');
    both('addDeploymentNode', () => ['nodeL', ['node', 'Node', undefined, 'Runs']]);
    both('addContainer', () => ['container', ['runtime', 'Runtime', 'Rust']]);
    both('popBoundaryParseStack');
    both('addRel', () => ['birel', ['guest', 'queue', 'Sends', 'TLS']]);
    both('updateElStyle', () => ['ignored', ['queue', 'gold', 'blue', 'red', undefined, 'queue']]);
    both('updateRelStyle', () => ['ignored', ['guest', 'queue', 'purple', 'green', '12.5px', '-3']]);
    both('updateLayoutConfig', () => ['ignored', ['3', '2']]);
    const added = both('element', value => [value.shapes, value.shapeIndex, 'extra', 'Extra']);
    check(added.value === model.shapeIndex.get('extra'), 'private helper return alias');
    both('finishShape', value => [value.shapeIndex.get('extra'), 'system', ['extra', 'Extra', undefined, 'sprite'], 3]);
    const boundary = both('boundary', () => [['helper', 'Helper'], 'custom']);
    const actualBoundary = boundary.value;
    both('open', value => [value.boundaryIndex.get('helper')]);
    check(model.current === actualBoundary, 'boundary/open identity');
    both('addPersonOrSystem', () => ['system', ['nested', 'Nested']]);
    both('popBoundaryParseStack');
    both('addRel', () => ['rel', ['extra', 'nested', 'Connects']]);
    compareRender('all nineteen real methods', {maxWidth: 280});
    const failed = both('addPersonOrSystem', () => ['system', ['user', null]], 'c4.addPersonOrSystem.label-null');
    check(failed.error?.name === 'TypeError' && original.label === '' && original.labelAttr === null, 'partial label write before native TypeError');
    compareRender('partial error state');
    both('addPersonOrSystem', () => ['person', ['user', 'Recovered user']]);
    mutate(value => {value.shapeIndex = new Map([...value.shapeIndex].reverse());});
    check(model.shapeIndex.get('user') === original, 'replaced lookup retains original record');
  } else if (family === 'venn') {
    both('setDiagramTitle', () => ['Method Venn']);
    both('addSubsetData', () => [['C'], 'Gamma', 12]);
    const current = both('getCurrentSets');
    check(current.value === model.current && current.value === model.subsets.at(-1).sets, 'current/getter/subset array alias');
    both('addSubsetData', () => [['C', 'A'], 'AC', 2]);
    both('addTextData', () => [['C', 'A'], 'text', 'Method content']);
    both('addStyleData', () => [['C', 'A'], [['fill', 'tomato'], ['color', 'purple'], ['fill', 'gold']]]);
    both('validateUnionIdentifiers', () => [['A', 'B', 'C']]);
    both('setIndentMode', () => [true]);
    const indent = both('getIndentMode');
    check(indent.value === true, 'indent setter/getter');
    compareRender('all eight real methods', {maxWidth: 280});
    const saved = [model.subsets, reference.subsets];
    mutate(value => {value.subsets = null;});
    const failed = both('addSubsetData', () => [['D'], 'Partial', 4], 'venn.addSubsetData.subsets-null');
    check(failed.error?.name === 'TypeError' && model.known.has('D') && model.current[0] === 'D', 'partial current/known mutation before TypeError');
    model.subsets = saved[0]; reference.subsets = saved[1];
    both('addSubsetData', () => [['D'], 'Recovered', 4]);
    const unknown = both('validateUnionIdentifiers', () => [['missing']]);
    check(unknown.error?.code === 'semantic', 'pinned semantic method error');
    mutate(value => {value.known = new Set([...value.known].reverse());});
  } else if (family === 'usecase') {
    const actor = model.actors.get('Customer'), usecase = model.useCases.get('Checkout'), actors = model.actors;
    mutate(value => {
      value.actors.get('Customer').label = 'Changed customer';
      value.useCases.get('Checkout').label = 'Confirm **order**';
      value.relationships[0].label = 'Places';
      value.relationships[0].styles.push('stroke:#e04773');
      value.actors.get('Customer').styles.push('fill:#caf0f8');
      value.title = 'Mutable usecase'; value.accTitle = 'Accessible usecase'; value.accDescr = 'Edited records';
    });
    compareRender('Map records, relationships and styles', {maxWidth: 280});
    mutate(value => {value.actors = new Map([...value.actors].reverse()); value.useCases = new Map([...value.useCases].reverse());});
    check(model.actors !== actors && model.actors.get('Customer') === actor && model.useCases.get('Checkout') === usecase, 'replacement Maps preserve record aliases');
  } else if (family === 'cynefin') {
    const complex = model.domains.get('complex'), items = complex.items;
    mutate(value => {
      value.domains.get('complex').items[0].label = 'Changed practice';
      value.domains.get('complex').items.push({label: 'Second practice'});
      value.transitions.push({from: 'complex', to: 'clear', label: 'Learn'});
      value.title = 'Mutable domains'; value.accTitle = 'Accessible domains'; value.accDescr = 'Edited decisions';
    });
    compareRender('domain items and transition', {maxWidth: 280});
    mutate(value => {value.domains = new Map([...value.domains].reverse());});
    check(model.domains.get('complex') === complex && complex.items === items, 'Map replacement retains domain and items');
  } else {
    const nodes = model.nodes, first = nodes[0], link = model.links[0];
    mutate(value => {
      value.nodes[0].label = 'Changed application'; value.nodes[0].x = .45; value.nodes[0].y = .65;
      value.links[0].label = 'Depends on'; value.links[0].dashed = true;
      value.notes.push({text: 'Edited note', x: .25, y: .4});
      value.title = 'Mutable map';
    });
    compareRender('coordinates links and notes', {maxWidth: 280});
    mutate(value => {value.nodes.reverse(); value.links = [...value.links];});
    check(model.nodes === nodes && model.nodes.at(-1) === first && model.links[0] === link, 'array reorder/replacement preserves records');
  }

  if (instance) {
    const name = family === 'c4' ? 'setTitle' : 'setDiagramTitle';
    const borrowed = {...model}, expectedBorrowed = {...reference};
    prototype[name].call(borrowed, 'Borrowed'); expectedPrototype[name].call(expectedBorrowed, 'Borrowed');
    check(canonical([model, borrowed]) === canonical([reference, expectedBorrowed]), 'borrowed native receiver graph');
  }
  const extras = [];
  for (const value of [model, reference]) {
    const extra = {raw: '\ud800A\udc00', nan: NaN, zero: -0, infinity: Infinity, shared: value};
    extra.self = extra; extras.push(extra);
    Object.defineProperty(value, 'extra', {value: extra, enumerable: false, writable: true, configurable: true});
    let at = extra;
    for (let i = 0; i < 1200; i++) {at.next = {}; at = at.next;}
    at.end = extra;
    value.type = 'consumer-extension';
  }
  compareRender('retained dispatch, descriptors and deep cyclic extras');
  check(model.extra === extras[0] && model.extra.shared === model && model.extra.self === model.extra && model.extra.raw.charCodeAt(0) === 0xd800 && Object.is(model.extra.zero, -0), 'UTF16/IEEE/descriptor identity');
  const sentinel = new Error(`${family} callback identity`); let caught;
  try {engine.render(model, {now, measurer: {width() {throw sentinel;}}});} catch (error) {caught = error;}
  check(caught === sentinel, 'original host exception identity');
  try {oracle.render(reference, {now, measurer: {width() {throw sentinel;}}});} catch {}
  compareGraph('published host failure');
  compareOutputFailure('edited retained root');
  compareRender('host recovery', {maxWidth: 240});
  check(outcome(() => engine.render({...model}, {now})).error?.name === 'TypeError', 'unowned lookalike rejected');
  const retainedMethod = instance ? model[family === 'c4' ? 'setTitle' : 'setDiagramTitle'] : null;
  engine.dispose(second); engine.dispose(model);
  check(outcome(() => engine.render(model, {now})).error?.name === 'TypeError', 'disposed render rejected');
  if (retainedMethod) check(outcome(() => retainedMethod.call(model, 'Disposed')).error?.name === 'TypeError', 'disposed method rejected');
  return {failures, intrinsicErrors};
}
for (const family of ['c4', 'usecase', 'cynefin', 'wardley', 'venn']) {
  test(`remaining mutable ${family}: complete graph, native methods, pinned SVG and recovery`, async ({page}, testInfo) => {
    await page.goto('/examples/web/');
    const report = await page.evaluate(exercise, {family});
    await testInfo.attach('intrinsic-vm-diagnostics', {body:Buffer.from(JSON.stringify({browser:testInfo.project.name, family, observations:report.intrinsicErrors}, null, 2)), contentType:'application/json'});
    expect(report.intrinsicErrors.length).toBe(['c4', 'venn'].includes(family) ? 1 : 0);
    expect(report.failures).toEqual([]);
  });
}
