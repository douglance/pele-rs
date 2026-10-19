import {test, expect} from '@playwright/test';

test('canonical package renders full flowcharts and the playground recovers', async ({page}) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/examples/web/');
  await expect(page.locator('#family-count')).toContainText('33 registered diagram families');
  await expect(page.locator('#preview svg')).toHaveCount(1);
  const result = await page.evaluate(async () => {
    const api = await import('/dist/package-web/package/index.js');
    const engine = await api.loadRenderer();
    const model = engine.parse('flowchart LR\nsubgraph Rust\nA[Parse] --> B[Layout]\nend\nB --> C[SVG]');
    try {
      const drawing = engine.render(model);
      return {oldEntry: 'loadPele' in api, svg: drawing.svg, width: drawing.width, height: drawing.height};
    } finally {engine.dispose(model);}
  });
  expect(result.oldEntry).toBe(false);
  expect(result.svg).toContain('Layout');
  expect(result.width).toBeGreaterThan(0);
  expect(result.height).toBeGreaterThan(0);
  const lastGood = await page.locator('#preview').innerHTML();
  await page.locator('#source').fill('not a diagram');
  await page.getByRole('button', {name: 'Render diagram', exact: true}).click();
  await expect(page.getByRole('alert')).toBeVisible();
  expect(await page.locator('#preview').innerHTML()).toBe(lastGood);
  await page.locator('#source').fill('flowchart LR\nA[Recovered] --> B[Ready]');
  await page.getByRole('button', {name: 'Render diagram', exact: true}).click();
  await expect(page.getByRole('alert')).toBeHidden();
  await expect(page.locator('#preview svg')).toContainText('Recovered');
  expect((await page.request.get('/bindings/web/%2e%2e%2f%2e%2e%2f.git/config')).status()).toBe(404);
  expect(errors).toEqual([]);
});
