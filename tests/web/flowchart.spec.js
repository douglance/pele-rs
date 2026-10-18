import { test, expect } from '@playwright/test';
import { readFile, mkdir, writeFile } from 'node:fs/promises';

test('real WASM corpus matches native Rust, paints SVG, rejects input and recovers', async ({ page, browserName, browser }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/examples/web/');
  const traversal = await page.request.get('/bindings/web/%2e%2e%2f%2e%2e%2f.git/config');
  expect(traversal.status()).toBe(404);
  await expect(page.locator('#status')).toContainText('5 nodes');
  await expect(page.locator('#preview svg')).toHaveCount(1);
  const fixtures = JSON.parse(await readFile('fixtures/manifest.json', 'utf8'));
  const sources = await Promise.all(fixtures.map(async fixture => ({ ...fixture, source: fixture.source ?? await readFile(`fixtures/${fixture.file}`, 'utf8') })));
  const output = await page.evaluate(async sources => {
    const engine = await import('/dist/wasm/pele.js'); await engine.default();
    return sources.map(f => ({ name: f.name, envelope: JSON.parse(engine.render(f.source, '')) }));
  }, sources);
  for (const fixture of fixtures) {
    const actual = output.find(f => f.name === fixture.name).envelope;
    const native = JSON.parse(await readFile(`dist/evidence/rust/${fixture.name}.json`, 'utf8'));
    expect(actual).toEqual(native);
    if (fixture.error) expect(actual.error.code).toBe(fixture.error);
    else {
      expect(actual.result.geometry.nodes).toHaveLength(fixture.nodes);
      expect(actual.result.geometry.edges).toHaveLength(fixture.edges);
    }
  }
  // Exercise the packed npm artifact, not only workspace JavaScript.
  const packed = await page.evaluate(async sources => {
    const { loadPele, PeleError } = await import('/dist/package-web/package/index.js');
    const sdk = await loadPele();
    return sources.map(f => {
      try { return { name: f.name, result: sdk.render(f.source) }; }
      catch (error) { return { name: f.name, code: error.code, typed: error instanceof PeleError }; }
    });
  }, sources);
  for (const fixture of fixtures) {
    const actual = packed.find(f => f.name === fixture.name);
    const raw = output.find(f => f.name === fixture.name).envelope;
    if (fixture.error) { expect(actual.code).toBe(fixture.error); expect(actual.typed).toBe(true); }
    else expect(actual.result).toEqual(raw.result);
  }
  const pixels = await page.locator('#preview svg').evaluate(async svg => {
    const image = new Image();
    image.src = URL.createObjectURL(new Blob([svg.outerHTML], { type: 'image/svg+xml' }));
    await image.decode();
    const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
    const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
    const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let painted = 0; for (let i=0; i<data.length; i+=4) if (data[i]<180 && data[i+3]>0) painted++;
    URL.revokeObjectURL(image.src); return painted;
  });
  expect(pixels).toBeGreaterThan(100);
  await mkdir('dist/evidence', { recursive: true });
  await page.screenshot({ path: `dist/evidence/browser-flowchart-${browserName}.png`, fullPage: true });
  await page.locator('#source').fill('flowchart TB\nsubgraph S\nA-->B\nend');
  await page.getByRole('button', { name: 'Render flowchart' }).click();
  await expect(page.getByRole('alert')).toContainText('unsupported_syntax');
  await expect(page.locator('#preview svg')).toHaveCount(0);
  await page.locator('#source').fill(sources.find(f => f.name === 'injection').source);
  await page.getByRole('button', { name: 'Render flowchart' }).click();
  await expect(page.locator('#preview svg')).toHaveCount(1);
  await expect(page.locator('#preview script, #preview img, #preview foreignObject')).toHaveCount(0);
  await page.locator('#example').selectOption('branch');
  await expect(page.locator('#status')).toContainText('5 nodes');
  await page.locator('#dark').check();
  await expect(page.locator('#preview svg > rect').first()).toHaveAttribute('fill', '#191b20');
  expect(errors).toEqual([]);
  await writeFile(`dist/evidence/browser-results-${browserName}.json`, JSON.stringify({ browser: browserName, version: browser.version(), fixtures: output.length, packedNpmVerified: true, paintedPixels: pixels, exactNativeParity: true, errors }, null, 2));
});
