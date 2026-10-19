import {test, expect} from '@playwright/test';

test('owned web registries match pinned discovery, selection and error ordering', async ({page}) => {
  await page.goto('/examples/web/');
  const failures = await page.evaluate(async () => {
    const {loadRegistry} = await import('/dist/package-web/package/registry.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const fixture = await (await fetch('/fixtures/pele/registry.json')).json();
    const empty = await loadRegistry(), other = await loadRegistry(), full = await loadRegistry(undefined, {allBuiltins: true});
    const failures = [], options = {now: 1704067200000, measurer: {width: (text, size, style) => text.length * size * (style & 1 ? .71 : .6)}};
    const projectError = error => ({name: error.name, code: error.code, message: error.message, type: error.type, line: error.line, column: error.column, snippet: error.snippet});
    const outcome = operation => {try {return {value: operation()};} catch (error) {return {error: projectError(error)};}};
    const check = (condition, stage) => {if (!condition) failures.push(stage);};
    check(empty.capabilities().length === 33 && empty.capabilities().every(c => c.known && !c.registered), 'empty native membership');
    check(full.capabilities().every(c => c.registered), 'all-builtins constructor');
    const expected = fixture.registries.find(registry => registry.id === 'empty');
    for (const [type, registered] of Object.entries(expected.registered)) check(empty.registered(type) === registered, 'exact identifier '+type);
    for (const entry of expected.cases) {
      check(JSON.stringify(outcome(() => empty.detectType(entry.source))) === JSON.stringify(entry.expected.detection), 'pinned detection '+entry.id);
      check(empty.supports(entry.source) === entry.expected.supports, 'pinned supports '+entry.id);
      if (entry.expected.parse?.error) check(JSON.stringify(outcome(() => empty.parse(entry.source))) === JSON.stringify(entry.expected.parse), 'pinned parse order '+entry.id);
    }
    for (const entry of expected.cases.filter(entry => entry.id.startsWith('canonical/'))) {
      const type = entry.expected.detection.value;
      empty.registerBuiltin(type);
      check(empty.registered(type) && !other.registered(type), 'independent selection '+type);
      // Canonical headings establish discovery; some intentionally lack valid
      // family grammar. Compare those parser/source failures before retained use.
      const parsed = outcome(() => empty.parse(entry.source));
      const pinned = outcome(() => oracle.parse(entry.source));
      check(JSON.stringify(parsed.error) === JSON.stringify(pinned.error), 'selected grammar outcome '+type);
      if (parsed.error || pinned.error) {
        check(JSON.stringify(outcome(() => empty.render(entry.source, options))) === JSON.stringify(outcome(() => oracle.render(entry.source, options))), 'whole selected source error '+type);
        if (parsed.value) empty.dispose(parsed.value);
        continue;
      }
      const model = parsed.value;
      const compare = outcome(() => empty.render(model, options));
      check(JSON.stringify(compare) === JSON.stringify(outcome(() => oracle.render(entry.source, options))), 'whole retained result '+type);
      check(JSON.stringify(outcome(() => empty.render(entry.source, options))) === JSON.stringify(compare), 'whole source result '+type);
      empty.registerBuiltin(type);
      check(JSON.stringify(outcome(() => empty.render(model, options))) === JSON.stringify(compare), 'retained registration after replacement '+type);
      check(outcome(() => other.render(model, options)).error?.name === 'TypeError', 'foreign owner rejected '+type);
      empty.dispose(model);
    }
    check(empty.capabilities().filter(c => c.registered).length === 33, 'all33 selected');
    const retained = empty.parse('pie\n"Retained":2');
    retained.sections.set('Changed', 3);
    const before = empty.render(retained, options);
    empty.close(); empty.close();
    check(JSON.stringify(empty.render(retained, options)) === JSON.stringify(before), 'model survives registry close');
    check(outcome(() => empty.parse('pie\n"A":1')).error?.message === 'Registry is closed', 'closed registry rejects parse');
    empty.dispose(retained); other.close(); full.close();
    return failures;
  });
  expect(failures).toEqual([]);
});

test('lazy built-in selection preserves pending promises, load-before-parse and recovery', async ({page}) => {
  await page.goto('/examples/web/');
  const failures = await page.evaluate(async () => {
    const {loadLazy, withLazy, loadRegistry} = await import('/dist/package-web/package/lazy.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const failures = [], lazy = await loadLazy(), options = {measurer: {width: (text, size, style) => text.length * size * (style & 1 ? .71 : .6)}};
    const check = (condition, stage) => {if (!condition) failures.push(stage);};
    check(await lazy.load(null) === false && await lazy.load('unknown') === false && await lazy.load('PIE') === false, 'unknown/null exact loading');
    const first = lazy.load('pie'), repeated = lazy.load('pie');
    check(first === repeated && !lazy.registered('pie'), 'concurrent load shares pending promise');
    check(await first && await repeated && lazy.registered('pie'), 'native selection resolves true');
    const source = 'pie\n"A":1\n"B":2';
    check(JSON.stringify(await lazy.renderAsync(source, options)) === JSON.stringify(oracle.render(source, options)), 'async render whole SVG');
    const long = await loadLazy();
    let failure;
    try {await long.renderAsync(source + ' '.repeat(50001), options);} catch (error) {failure = error;}
    check(failure?.code === 'limit' && long.registered('pie'), 'load occurs before semantic source limit');
    const malformed = await loadLazy();
    try {await malformed.renderAsync('---\nconfig: [\n---\npie\n"A":1', options);} catch (error) {failure = error;}
    check(failure?.code === 'syntax' && malformed.capabilities().every(c => !c.registered), 'malformed preprocessor selects no family');
    const registry = await loadRegistry(), original = registry.registerBuiltin.bind(registry), sentinel = new Error('loader rejected once');
    let attempts = 0;
    registry.registerBuiltin = type => {if (++attempts === 1) throw sentinel;original(type);};
    const recovering = withLazy(registry);
    try {await recovering.load('info');} catch (error) {failure = error;}
    check(failure === sentinel && !registry.registered('info'), 'loader rejection identity');
    check(await recovering.load('info') && attempts === 2, 'failed pending entry deleted for retry');
    const host = document.createElement('div');host.style.width = '400px';document.body.append(host);
    const mounted = await lazy.mountAsync(host, 'flowchart LR\nA-->B', {...options, zoom: false});
    check(lazy.registered('flowchart') && mounted.result.type === 'flowchart', 'mountAsync selects requested family');
    mounted.update('sequenceDiagram\nA->>B: Hello');
    check(mounted.error?.code === 'unsupported-diagram' && mounted.result.type === 'flowchart', 'later update does not silently load another family');
    await lazy.load('sequence');mounted.update('sequenceDiagram\nA->>B: Hello');
    check(mounted.error === undefined && mounted.result.type === 'sequence', 'explicit later load recovers mount');
    mounted.destroy();host.remove();
    for (const owner of [lazy, long, malformed, registry]) owner.close();
    return failures;
  });
  expect(failures).toEqual([]);
});
