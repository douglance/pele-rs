import {test, expect} from '@playwright/test';

test('ordinary custom Diagram registration matches pinned preparation, live methods and render policy', async ({page}) => {
  await page.goto('/examples/web/');
  const results = await page.evaluate(async () => {
    const {loadRegistry} = await import('/dist/package-web/package/registry.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const native = await loadRegistry();
    function exercise(api) {
      const trace = [], failures = [], documents = [];
      const check = (condition, name) => { if (!condition) failures.push(name); };
      const scalar = value => typeof value === 'number' ? Number.isNaN(value) ? 'NaN' : Object.is(value, -0) ? '-0' : String(value) : value instanceof Date ? ['Date', value.getTime()] : value;
      const failure = error => ({name: error.name, code: error.code, message: error.message, type: error.type, line: error.line, column: error.column, snippet: error.snippet});
      class Model { constructor(config) {this.width = 40;this.renders = 0;this.config = config;this.items = new Map([['one', 1]]);} increment() {this.width++;} }
      let parseCount = 0, lastLinks, lastModel;
      const a = {
        type: 'pie', section: 'pie', keepComments: true,
        parse(source, config, title, offset, limits) {
          check(this === a, 'parse receiver');parseCount++;
          trace.push(['parse', source, title, offset, scalar(limits.maxEdges), config]);
          config.pie = {...config.pie, useMaxWidth: false};
          lastModel = new Model(config);return lastModel;
        },
        render(model, config, options) {
          check(this === a && config === model.config, 'retained provider/config identity');
          model.renders++;
          trace.push(['render', options.responsive, scalar(options.fontSize), scalar(options.maxWidth), scalar(options.directionBreakpoint), scalar(options.padding), scalar(options.now), scalar(options.outputLimit), options.limit, options.maxEdges, options.config]);
          if (options.measurer) trace.push(['width', options.measurer.width('custom', options.fontSize ?? 13, 1)]);
          if (options.icons) {trace.push(['icon', options.icons('ﬂ°°65¶ß')]);trace.push(['icon', options.icons('ﬂ°°65¶ß')]);}
          lastLinks = [{id: 'ﬂ°°65¶ß', href: 'https://example.com', internal: false}];
          return {svg: `<svg xmlns="http://www.w3.org/2000/svg"><rect width="${model.width}" height="20"/></svg>`, width: model.width, height: 20, links: lastLinks};
        }
      };
      api.register();api.register(a);
      check(api.registered('pie') && api.supports('pie bad grammar'), 'selection is independent of grammar');
      const source = '---\ntitle: Provider\nconfig:\n  pie:\n    useMaxWidth: true\n---\npie\n%% keep comment\n"#65;":1';
      const model = api.parse(source, {maxEdges: NaN, config: {extra: 'host'}});documents.push(model);
      check(model === lastModel && model instanceof Model && model.items instanceof Map, 'original model identity');
      model.width = 72;model.increment();model.type = 'foreign-type';
      const options = {fontSize: NaN, maxWidth: -Infinity, directionBreakpoint: -0, padding: Infinity, now: new Date(1704067200000),
        limit: -1, maxEdges: -2, config: {passive: true}, outputLimit: Infinity,
        measurer: {width: (text, _size, style) => text.length + style}, icons: name => {trace.push(['resolve', name]);return `<g data-icon="${name}"/>`;}};
      const first = api.render(model, options);
      check(first.type === 'pie' && first.width === 73 && first.links === lastLinks && lastLinks[0].id === 'A', 'output dimensions, link identity and decoding');
      trace.push(['first', first]);
      a.keepComments = false;const stripped = api.parse(source);documents.push(stripped);
      const b = {type: 'pie', parse: (...args) => a.parse.apply(a, args), render: (model, _config, _options) => ({svg: '<svg data-provider="B"/>', width: model.width, height: 2, links: []})};
      api.register(b);
      const fresh = api.parse('pie\n"B":2');documents.push(fresh);
      a.section = 'other';a.type = 'flowchart';
      const original = a.render;
      a.render = function(model, config, options) {model.width++;return original.call(this, model, config, options);};
      trace.push(['retained', api.render(model, {outputLimit: Infinity})]);trace.push(['replacement', api.render(fresh)]);
      const marker = {message: 'original callback'};
      a.render = function(model) {model.renders++;throw marker;};
      try {api.render(model, {outputLimit: 0});failures.push('missing callback throw');} catch (error) {check(error === marker, 'callback Error identity before output limit');}
      a.render = () => {throw 'literal thrown string';};
      try {api.render(model);} catch (error) {check(error === 'literal thrown string', 'literal thrown string identity');}
      a.render = original;
      try {api.render(model, {outputLimit: -1});failures.push('missing output limit');} catch (error) {trace.push(['output error', failure(error)]);}
      const before = parseCount;
      try {api.render('pie', {limit: -1});failures.push('missing source limit');} catch (error) {trace.push(['source error', failure(error)]);}
      check(parseCount === before, 'source limit precedes custom parse');
      a.render = function(model, config, options) {
        try {options.icons('ﬂ°°65¶ß');failures.push('missing resource error');} catch (error) {check(error === marker, 'caught resource Error identity');}
        return original.call(this, model, config, {...options, icons: undefined});
      };
      trace.push(['caught', api.render(model, {icons: () => {throw marker;}})]);
      a.render = original;
      try {api.render(model, {icons: () => {throw marker;}, outputLimit: 0});failures.push('missing resource throw');}
      catch (error) {check(error === marker, 'uncaught resource Error identity before output limit');}
      trace.push(['mutation', model.renders, model.width]);
      b.parse = () => 17;b.render = value => ({svg: '<svg/>', width: value, height: 1, links: []});
      trace.push(['primitive source model', api.render('pie')]);check(api.parse('pie') === 17, 'primitive parse result');
      for (const document of documents) api.dispose?.(document);
      api.close?.();return {trace, failures};
    }
    return {native: exercise(native), oracle: exercise(oracle)};
  });
  expect(results.native.failures).toEqual([]);expect(results.oracle.failures).toEqual([]);
  expect(results.native.trace).toEqual(results.oracle.trace);
});

test('custom native registry ownership, guarded disposal, finite Config and built-in restoration', async ({page}) => {
  await page.goto('/examples/web/');
  const failures = await page.evaluate(async () => {
    const {loadRegistry} = await import('/dist/package-web/package/registry.js');
    const owner = await loadRegistry(), other = await loadRegistry();
    const failures = [], check = (condition, name) => {if (!condition) failures.push(name);};
    other.registerBuiltin('pie');const builtinCapability = other.capabilities().find(c => c.type === 'pie');
    const throws = operation => {try {operation();return undefined;} catch (error) {return error;}};
    const marker = new Error('parse failed');let model, liveOptions;
    const provider = {type: 'pie', parse: (_source, config) => ({width: 31, config}),
      render(value, config, options) {
        liveOptions = options;check(value.config === config, 'same Config identity');
        check(throws(() => owner.dispose(model)) instanceof TypeError, 'active disposal blocked');
        value.width++;return {svg: '<svg/>', width: value.width, height: 10, links: []};
      }};
    owner.register(provider);check(owner.capabilities().find(c => c.type === 'pie').registered, 'capability membership');
    check(!owner.capabilities().find(c => c.type === 'pie').renderParity, 'foreign parity not inherited');
    model = owner.parse('pie');check(throws(() => other.render(model)) instanceof TypeError, 'foreign registry model rejected');
    owner.render(model, {limit: -1, maxEdges: -1, config: {passive: true}});
    check(liveOptions.limit === -1 && liveOptions.maxEdges === -1 && liveOptions.config.passive === true, 'passive parse options reach callback');
    provider.parse = () => {throw marker;};check(throws(() => owner.parse('pie')) === marker, 'parse error identity');
    provider.parse = (_source, config) => ({width: 1, config});
    check(throws(() => owner.parse('pie', {config: {bad: NaN}})) instanceof TypeError, 'nonfinite Config rejects explicitly');
    owner.registerBuiltin('pie');check(JSON.stringify(owner.capabilities().find(c => c.type === 'pie')) === JSON.stringify(builtinCapability), 'built-in restored');
    const built = owner.parse('pie\n"A":1');check(built.sections instanceof Map, 'restored built-in parser');
    check(owner.render(built).svg.includes('<svg'), 'restored built-in renderer');
    owner.close();check(owner.render(model).width === 33, 'retained provider survives replacement and close');
    owner.dispose(model);check(throws(() => owner.render(model)) instanceof TypeError, 'disposed model rejected');
    owner.dispose(built);other.close();
    const reentrant = await loadRegistry();
    const replacement = {type: 'pie', parse() {reentrant.registerBuiltin('pie');return {width: 7};},
      render(value) {return {svg: '<svg/>', width: value.width, height: 1, links: []};}};
    reentrant.register(replacement);const retained = reentrant.parse('pie');
    const selected = reentrant.parse('pie\n"Selected":1');check(selected.sections instanceof Map, 'registration inside parse takes effect for future sources');reentrant.dispose(selected);
    check(reentrant.render(retained).width === 7, 'in-flight selected provider survives parse-time replacement');
    reentrant.register({...replacement, parse() {reentrant.close();return {width: 9};}});
    const afterClose = reentrant.parse('pie');
    check(reentrant.render(afterClose).width === 9, 'parse-time close retains selected provider');
    reentrant.dispose(retained);reentrant.dispose(afterClose);return failures;
  });
  expect(failures).toEqual([]);
});
