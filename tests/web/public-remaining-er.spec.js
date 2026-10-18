import {test, expect} from '@playwright/test';

// Public feature slices exercise the packed consumer package and pinned Pele,
// including retained native dispatch. No family implementation runs in JS here.
async function exercise({family}) {
  const {loadRenderer} = await import('/dist/package-web/package/full.js');
  const oracle = await import('/dist/evidence/pele-oracle.js');
  const engine = await loadRenderer(), failures = [], intrinsicErrors = [], now = 1704067200000;
  const sources = {
    gantt: 'gantt\ndateFormat YYYY-MM-DD\nsection S\nA:a,2024-01-01,1d\nB:b,after a,2d',
    sequence: 'sequenceDiagram\nbox Aquamarine Clients\nparticipant A as Client\nend\nparticipant B as Server\nA->>B: Request\nB-->>A: Reply',
    requirement: 'requirementDiagram\nrequirement A {\n text: first\n}\nelement E {\n type: Thing\n}\nA - contains -> E',
    block: 'block\ncolumns 2\na["Alpha"] b(("Beta"))\na --> b\nclassDef hot fill:#f96,color:blue\nclass a hot',
    railroad: 'railroad-beta\nr = sequence(terminal("first"),nonterminal("other"));\nother = terminal("second");',
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
    "gantt.addSection.sections-null": "Cannot read properties of null (reading 'push')",
    "sequence.addBox.boxes-null": "Cannot read properties of null (reading 'push')",
    "requirement.setCssStyle.styles-null": "Cannot read properties of null (reading 'push')",
    "block.setCssClass.readonly-length": "Cannot assign to read only property 'length' of object '[object Array]'",
    "railroad.addRule.lookup-null": "Cannot read properties of null (reading 'set')",
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
  check(prototype !== Object.prototype, 'native instance prototype');
  for (const name of Reflect.ownKeys(expectedPrototype).filter(key => key !== 'constructor')) {
    const actual = Object.getOwnPropertyDescriptor(prototype, name), expected = Object.getOwnPropertyDescriptor(expectedPrototype, name);
    check(!!actual && actual.enumerable === expected.enumerable && actual.configurable === expected.configurable && actual.writable === expected.writable && actual.value.length === expected.value.length, `descriptor ${String(name)}`);
    check(model[name] === second[name], `shared prototype function ${String(name)}`);
  }
  compareGraph('parsed own fields and private state');
  if (family === 'gantt') {
    compareOutputFailure('fresh scheduling');
    check(model.tasks[0].processed === true && model.tasks[0].endTime instanceof Date, 'failed first render publishes schedule');
  }
  compareRender('initial');

  if (family === 'gantt') {
    const tasks = model.tasks, first = model.tasks[0];
    check(model.byId.get('a') === first, 'task/byId alias');
    const scheduled = both('getTasks', () => [now]);
    check(scheduled.value === tasks && model.tasks === tasks, 'getTasks returns original task array');
    check(first.endTime instanceof Date && model.tasks[1].startTime === first.endTime, 'dependent shared Date');
    const oldDate = first.endTime, oldTime = +oldDate;
    both('getTasks', () => [new Date(now + 1000)]);
    check(first.endTime !== oldDate && +oldDate === oldTime, 'old Date remains detached');
    mutate(value => {value.byId.get('a').task = 'Edited task'; value.links.set('a', 'https://example.com/a');});
    both('addSection', () => ['Next']);
    both('addTask', () => ['Build', ':c,after b,1d']);
    both('setClass', () => ['c', 'hot']);
    compareRender('scheduling and builders', {maxWidth: 320});
    const sections = [model.sections, reference.sections];
    mutate(value => {value.sections = null;});
    const failed = both('addSection', () => ['Published before failure'], 'gantt.addSection.sections-null');
    check(failed.error?.name === 'TypeError' && model.currentSection === 'Published before failure', 'partial section update published');
    model.sections = sections[0]; reference.sections = sections[1];
    mutate(value => {value.tasks = [value.byId.get('b')];});
    const replacement = model.tasks;
    both('getTasks', () => [now]);
    check(model.tasks === replacement && model.byId.get('a') === first, 'replacement array and detached byId');
  } else if (family === 'sequence') {
    const actors = model.actors, boxed = actors.get('A').box;
    check(boxed === model.boxes[0], 'actor.box shared with boxes');
    mutate(value => {value.actors.get('A').box.fill = 'Lime'; value.actors.get('A').description = 'Changed client'; value.boxes = [];});
    both('addSignal', () => ['B', 'A', {text: 'Acknowledged'}, 0]);
    const menu = both('addALink', () => ['A', {text: 'Docs @ https://example.com/docs'}]);
    check(!menu.error && model.actors.get('A').links.get('Docs') === 'https://example.com/docs', 'link menu builder');
    compareRender('live box and message', {maxWidth: 320});
    check(model.actors === actors && actors.get('A').box === boxed, 'Map and detached box retained');
    const boxes = [model.boxes, reference.boxes];
    mutate(value => {value.boxes = null;});
    const failed = both('addBox', () => [{text: 'Published box', color: 'pink'}], 'sequence.addBox.boxes-null');
    check(failed.error?.name === 'TypeError' && model.currentBox.name === 'Published box', 'partial box update published');
    model.boxes = boxes[0]; reference.boxes = boxes[1];
    mutate(value => {value.currentBox = undefined; value.actors = new Map([...value.actors].reverse());});
    both('setAccTitle', () => ['  Accessible sequence']);
  } else if (family === 'requirement') {
    const requirements = model.requirements;
    both('setNewReqId', () => ['native-2']);
    both('setNewReqText', () => ['Added requirement']);
    const added = both('addRequirement', () => ['B', 'Requirement']);
    check(added.value === requirements.get('B') && model.newText === '', 'builder return alias and pending reset');
    both('addRelationship', () => ['satisfies', 'E', 'B']);
    both('defineClass', () => [['hot'], ['fill:red', 'color:blue']]);
    both('setClass', () => [['A', 'B'], ['hot']]);
    both('setCssStyle', () => [['A'], ['stroke:green,stroke-width:2']]);
    both('setNewReqText', () => ['\ud800 pending']);
    check(model.newText.charCodeAt(0) === 0xd800, 'pending UTF16 remains lossless');
    both('setNewReqText', () => ['']);
    compareRender('requirements classes and styles', {maxWidth: 320});
    const target = model.requirements.get('A'), oldStyles = [target.cssStyles, reference.requirements.get('A').cssStyles];
    mutate(value => {value.requirements.get('A').cssStyles = null;});
    const failed = both('setCssStyle', () => [['A'], ['fill:gold']], 'requirement.setCssStyle.styles-null');
    check(failed.error?.name === 'TypeError', 'style push error');
    check(model.styleCopies === reference.styleCopies, 'style counter mutation published before throw');
    model.requirements.get('A').cssStyles = oldStyles[0];
    reference.requirements.get('A').cssStyles = oldStyles[1];
    mutate(value => {value.requirements = new Map([...value.requirements].reverse());});
    check(model.requirements !== requirements && model.requirements.get('A') === target, 'replacement Map preserves record');
  } else if (family === 'block') {
    const block = model.byId.get('a');
    check(model.blocks.includes(block) && model.root.children.includes(block), 'block hierarchy/index aliases');
    both('addStyleClass', () => ['hot', 'fill:#00f,color:white,font-weight:bold']);
    both('addStyle2Node', () => ['b', 'stroke:red,stroke-width:3']);
    both('setCssClass', () => ['a,b', 'hot']);
    compareRender('styles and hierarchy', {maxWidth: 160});
    const classes = block.classes;
    mutate(value => {Object.defineProperty(value.byId.get('a').classes, 'length', {writable: false});});
    const failed = both('setCssClass', () => ['a', 'hot'], 'block.setCssClass.readonly-length');
    check(failed.error?.name === 'TypeError', 'readonly array length error');
    check(block.classes === classes && Object.getOwnPropertyDescriptor(classes, 'length').writable === false, 'readonly descriptor and array identity');
    // Array length is nonconfigurable; replace it to recover, preserving node identity.
    mutate(value => {value.byId.get('a').classes = [...value.byId.get('a').classes];});
    both('setCssClass', () => ['a', 'hot']);
    both('generateId');
    mutate(value => {value.byId = new Map([...value.byId].reverse()); value.byId.get('a').label = 'Changed **block**';});
    check(model.byId.get('a') === block, 'replacement lookup preserves hierarchy record');
  } else {
    const original = model.rules[0];
    check(model.getRule('r') === original, 'rule/index return alias');
    reference.getRule('r');
    const added = () => [{name: 'new', definition: {type: 'sequence', elements: [
      {type: 'terminal', value: 'n'},
      {type: 'repetition', element: {type: 'nonterminal', name: 'r'}, min: 1, max: Infinity},
    ]}}];
    both('addRule', added);
    const found = both('getRule', () => ['new']);
    check(found.value === model.rules.at(-1), 'new rule shared return');
    both('setTitle', () => ['Native grammar']);
    compareRender('rules and repetition', {maxWidth: 180});
    const lookup = [model.byName, reference.byName];
    mutate(value => {value.byName = null;});
    const failed = both('addRule', () => [{name: 'partial', definition: {type: 'terminal', value: 'published'}}], 'railroad.addRule.lookup-null');
    check(failed.error?.name === 'TypeError' && model.rules.at(-1).name === 'partial', 'rule push published before lookup failure');
    model.byName = lookup[0]; reference.byName = lookup[1];
    mutate(value => {value.rules = [value.getRule('new')];});
    check(model.getRule('r') === original, 'detached private lookup survives rules replacement');
    reference.getRule('r');
  }

  // Borrowed real prototype method must update the supplied receiver and preserve
  // aliases to its shared collections, without changing the retained model root.
  const setter = family === 'railroad' ? 'setTitle' : family === 'block' ? null : 'setAccTitle';
  if (setter) {
    const borrowed = {...model}, expectedBorrowed = {...reference};
    prototype[setter].call(borrowed, '  Borrowed');
    expectedPrototype[setter].call(expectedBorrowed, '  Borrowed');
    check(canonical([model, borrowed]) === canonical([reference, expectedBorrowed]), 'borrowed receiver graph');
  }
  const extras = [];
  for (const value of [model, reference]) {
    const extra = {raw: '\ud800A\udc00', nan: NaN, zero: -0, infinity: Infinity, shared: value};
    extra.self = extra; value.extra = extra; extras.push(extra);
    let at = extra;
    for (let i = 0; i < 1200; i++) {at.next = {}; at = at.next;}
    at.end = extra;
  }
  compareRender('replacement, deep extras and recovery');
  check(model.extra === extras[0] && model.extra.shared === model && model.extra.self === model.extra && model.extra.raw.charCodeAt(0) === 0xd800 && Object.is(model.extra.zero, -0), 'extra identity UTF16 and IEEE values');
  const sentinel = new Error(`${family} host callback identity`); let caught;
  try {engine.render(model, {now, measurer: {width() {throw sentinel;}}});} catch (error) {caught = error;}
  check(caught === sentinel, 'original host exception identity');
  // Oracle performs the same failed scheduling/render, so complete post-error
  // graphs compare as well as the successful recovery SVG.
  try {oracle.render(reference, {now, measurer: {width() {throw sentinel;}}});} catch {}
  compareGraph('published failed render');
  compareOutputFailure('retained edited model');
  compareRender('host failure recovery', {maxWidth: 240});
  const method = Reflect.ownKeys(expectedPrototype).find(key => key !== 'constructor' && typeof reference[key] === 'function');
  const retainedMethod = model[method];
  engine.dispose(second); engine.dispose(model);
  check(outcome(() => engine.render(model, {now})).error?.name === 'TypeError', 'disposed render rejected');
  check(outcome(() => retainedMethod.call(model)).error?.name === 'TypeError', 'disposed native method rejected');
  return {failures, intrinsicErrors};
}

for (const family of ['gantt', 'sequence', 'requirement', 'block', 'railroad']) {
  test(`remaining mutable ${family}: native methods, complete aliases, pinned SVG and recovery`, async ({page}, testInfo) => {
    await page.goto('/examples/web/');
    const report = await page.evaluate(exercise, {family});
    await testInfo.attach('intrinsic-vm-diagnostics', {body:Buffer.from(JSON.stringify({browser:testInfo.project.name, family, observations:report.intrinsicErrors}, null, 2)), contentType:'application/json'});
    expect(report.intrinsicErrors.length).toBe(1);
    expect(report.failures).toEqual([]);
  });
}
