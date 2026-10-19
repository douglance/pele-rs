import {test, expect} from '@playwright/test';

test('failed shared initialization retries without poisoning other facades', async ({page}) => {
  await page.route('**/initialization-recovery.html', route => route.fulfill({contentType:'text/html',body:'<!doctype html><title>Recovery</title>'}));
  let loads = 0;
  await page.route('**/pkg/pele_bg.wasm', async route => {
    loads++;
    if (loads === 1) await route.fulfill({status:503,contentType:'application/wasm',body:'temporary transport failure'});
    else await route.continue();
  });
  await page.goto('/initialization-recovery.html');
  const actual = await page.evaluate(async () => {
    const [full, registry, entries] = await Promise.all([
      import('/dist/package-web/package/index.js'),
      import('/dist/package-web/package/registry.js'),
      import('/dist/package-web/package/diagram-entries.js'),
    ]);
    const failed = await Promise.allSettled([full.loadRenderer(),registry.loadRegistry(),entries.loadDiagramEntries()]);
    const rejected = failed.every(value => value.status === 'rejected');
    const sameError = rejected && failed.every(value => value.reason === failed[0].reason);
    const [engine, empty] = await Promise.all([full.loadRenderer(),registry.loadRegistry(),entries.loadDiagramEntries()]);
    const model = engine.parse('pie\n"Recovered":1');
    try { return {rejected,sameError,svg:engine.render(model).svg,all:engine.registered('pie'),empty:empty.registered('pie')}; }
    finally {engine.dispose(model);empty.close();engine.close();}
  });
  expect(loads).toBe(2);
  expect(actual.rejected).toBe(true);
  expect(actual.sameError).toBe(true);
  expect(actual.all).toBe(true);
  expect(actual.empty).toBe(false);
  expect(actual.svg).toContain('Recovered');
});
