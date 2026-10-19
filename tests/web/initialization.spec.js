import {test, expect} from '@playwright/test';

test('concurrent web entry initialization retains independent native owners', async ({page}) => {
  await page.route('**/initialization-blank.html', route => route.fulfill({contentType:'text/html',body:'<!doctype html><title>Initialization</title>'}));
  let loads = 0;
  page.on('request', request => { if (new URL(request.url()).pathname.endsWith('/pkg/pele_bg.wasm')) loads++; });
  await page.goto('/initialization-blank.html');
  const actual = await page.evaluate(async () => {
    const [full, registry, core, entries] = await Promise.all([
      import('/dist/package-web/package/index.js'),
      import('/dist/package-web/package/registry.js'),
      import('/dist/package-web/package/core.js'),
      import('/dist/package-web/package/diagram-entries.js'),
    ]);
    const [engine, empty, inspection] = await Promise.all([
      full.loadRenderer(), registry.loadRegistry(), core.loadCore(), entries.loadDiagramEntries(),
    ]);
    const source = 'gantt\ndateFormat YYYY-MM-DD\ntodayMarker off\nA:a,2024-01-01,1d';
    const model = engine.parse(source);
    try {
      const drawing = engine.render(model);
      const gantt = (await import('/dist/package-web/package/diagrams/gantt.js')).default;
      const raw = gantt.parse(source, {}, undefined, 0, {maxEdges:undefined});
      try {
        return {all:engine.registered('gantt'),empty:empty.registered('gantt'),
          core:inspection.detectType(source),svg:drawing.svg,raw:gantt.render(raw,{},{}).svg};
      } finally { entries.disposeDiagramModel(raw); }
    } finally { engine.dispose(model); empty.close(); engine.close(); }
  });
  expect(loads).toBe(1);
  expect(actual.all).toBe(true);
  expect(actual.empty).toBe(false);
  expect(actual.core).toBe('gantt');
  expect(actual.svg).toContain('pele-gantt');
  expect(actual.raw).toBe(actual.svg);
});
