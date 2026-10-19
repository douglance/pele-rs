// Run only after the final matching release gates pass:
// npm run serve
// node scripts/record-full-demo.mjs --release-verified
// Captures the real packed-library UI; no generated or reconstructed frames.
import assert from 'node:assert/strict';
import {chromium} from '@playwright/test';
import {createHash} from 'node:crypto';
import {constants} from 'node:fs';
import {copyFile, mkdir, mkdtemp, readFile, stat, writeFile} from 'node:fs/promises';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';

const recording = process.argv[2] === '--release-verified';
assert(recording || process.argv[2] === '--verify-only', 'Use --verify-only to check the demo, or --release-verified after release gates pass.');
assert.equal(process.argv.length, 3, 'Usage: node scripts/record-full-demo.mjs --verify-only | --release-verified');
const execute = promisify(execFile);
const root = fileURLToPath(new URL('..', import.meta.url));
const url = new URL(process.env.PELE_DEMO_URL ?? 'http://127.0.0.1:5191/examples/web/full.html');
assert(['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname), 'Use the local repository server.');
assert.equal(url.pathname, '/examples/web/full.html');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const manifest = JSON.parse(await readFile(join(root, 'dist/package-web/package/package.json'), 'utf8'));
assert.equal(manifest.version, '0.3.0', 'Record the verified 0.3 package.');
const archive = join(root, 'dist/release', `unlocalhosted-pele-rs-${manifest.version}.tgz`);
const output = join(root, 'docs/images/full-demo.gif');
if (recording) {
  try { await stat(output); throw new Error('full-demo.gif exists; preserve or move it before a new recording.'); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  await execute('ffmpeg', ['-version'], {timeout: 5000, maxBuffer: 64 * 1024});
}

async function artifacts() {
  const values = {
    archive: hash(await readFile(archive)),
    demoHTML: hash(await readFile(join(root, 'examples/web/full.html'))),
    stylesheet: hash(await readFile(join(root, 'examples/web/style.css'))),
    recorder: hash(await readFile(fileURLToPath(import.meta.url))),
  };
  for (const name of ['full.js', 'pkg/pele_bg.wasm']) {
    const bytes = await readFile(join(root, 'dist/package-web/package', name));
    const {stdout} = await execute('tar', ['-xOf', archive, `package/${name}`], {
      encoding: 'buffer', timeout: 10000, maxBuffer: 64 * 1024 * 1024,
    });
    assert(bytes.equals(stdout), `${name} differs from the actual npm archive.`);
    values[name] = hash(bytes);
  }
  return values;
}
const before = await artifacts();
await mkdir(join(root, 'dist'), {recursive: true});
const frames = await mkdtemp(join(root, 'dist/full-demo-frames-'));
const {stdout: revision} = await execute('git', ['-C', root, 'rev-parse', 'HEAD'], {timeout: 5000, maxBuffer: 4096});
const proof = {version: manifest.version, revision: revision.trim(), mode: recording ? 'record' : 'verify-only', url: String(url), artifacts: before, checks: [], frames: 0, succeeded: false};
const deadline = setTimeout(() => {
  console.error('Recording deadline exceeded; stopping owned browser.');
  const stop = setTimeout(() => process.exit(1), 3000);
  stop.unref();
  void browser?.close().finally(() => process.exit(1));
}, 180000);
let browser;
try {
  browser = await chromium.launch({...(!process.env.CI ? {channel: 'chrome'} : {}), timeout: 15000});
  const page = await browser.newPage({viewport: {width: 1280, height: 1120}, deviceScaleFactor: 1, reducedMotion: 'reduce'});
  page.setDefaultTimeout(10000);
  const failures = [], requests = [];
  page.on('pageerror', error => failures.push(error.message));
  page.on('request', request => requests.push(request.url()));
  page.on('requestfailed', request => failures.push(`Request failed: ${request.url()}`));
  page.on('response', response => {
    if (response.status() >= 400 && !response.url().endsWith('/favicon.ico')) failures.push(`HTTP ${response.status()}: ${response.url()}`);
  });
  await page.goto(String(url), {waitUntil: 'networkidle', timeout: 20000});
  await page.waitForFunction(() => !document.querySelector('#render').disabled);
  assert.match(await page.locator('#family-count').innerText(), /^33 registered diagram families/);
  proof.checks.push('33 families discovered from packed runtime');

  const renderNumber = () => page.locator('#preview').getAttribute('data-render');
  const waitForDrawing = async (family, previous = null) => {
    await page.waitForFunction(({family, previous}) => {
      const preview = document.querySelector('#preview');
      return preview.dataset.family === family && preview.dataset.render !== previous && document.querySelector('#error').hidden;
    }, {family, previous});
    const dimensions = await page.locator('#preview svg').evaluate(svg => ({
      width: Number(svg.getAttribute('width')), height: Number(svg.getAttribute('height')),
      viewBox: [svg.viewBox.baseVal.width, svg.viewBox.baseVal.height],
      className: svg.getAttribute('class'), text: svg.textContent,
    }));
    assert(dimensions.width > 0 && dimensions.height > 0 && dimensions.viewBox.every(value => Number.isFinite(value) && value > 0));
    assert(dimensions.className.split(/\s+/).includes(`pele-${family}`));
    return dimensions;
  };
  const hold = async seconds => {
    if (!recording) return;
    assert(proof.frames + seconds * 6 <= 240, 'Frame budget exceeded.');
    for (let index = 0; index < seconds * 6; index++) {
      await page.screenshot({path: join(frames, `${String(proof.frames++).padStart(4, '0')}.png`), timeout: 10000});
      await page.waitForTimeout(1000 / 6);
    }
  };
  const choose = async (family, marker) => {
    const previous = await renderNumber();
    await page.locator('#example').selectOption(family);
    const drawing = await waitForDrawing(family, previous);
    assert(drawing.text.includes(marker), `Missing actual ${family} label.`);
    proof.checks.push(`${family} SVG rendered with finite dimensions and expected content`);
    await hold(2);
  };
  const flow = await waitForDrawing('flowchart');
  assert(flow.text.includes('Capture'));
  proof.checks.push('flowchart SVG rendered with finite dimensions and expected content');
  await page.screenshot({path: join(frames, 'desktop.png'), fullPage: true});
  await page.setViewportSize({width: 390, height: 844});
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Mobile layout overflows horizontally.');
  await page.screenshot({path: join(frames, 'mobile.png'), fullPage: true});
  await page.setViewportSize({width: 1280, height: 1120});
  proof.checks.push('Actual desktop and mobile screenshots captured; mobile has no horizontal overflow');
  await hold(2);
  await choose('class', 'Engine');
  await choose('sequence', 'Parse source');
  await choose('pie', 'Swift');

  const pieSource = await page.locator('#source').inputValue();
  const pieSVG = await page.locator('#preview').innerHTML();
  const pieRender = await renderNumber();
  await page.getByRole('button', {name: 'Add Rust slice', exact: true}).click();
  const mutated = await waitForDrawing('pie', pieRender);
  assert(mutated.text.includes('Rust'));
  assert.notEqual(await page.locator('#preview').innerHTML(), pieSVG);
  assert.equal(await page.locator('#source').inputValue(), pieSource, 'Model edit must not rewrite/reparse editor source.');
  assert.match(await page.locator('#status').innerText(), /changed in owned model/);
  assert.equal(await page.getByRole('button', {name: 'Grow Rust slice', exact: true}).isEnabled(), true);
  proof.checks.push('Pie Map mutation changes actual SVG while source stays unchanged');
  await hold(2);
  await choose('railroad', 'flowchart');

  const lastGood = await page.locator('#preview').innerHTML();
  const lastRender = await renderNumber();
  await page.locator('#source').fill('this is not a diagram');
  await page.getByRole('button', {name: 'Render diagram', exact: true}).click();
  await page.getByRole('alert').waitFor({state: 'visible'});
  assert((await page.getByRole('alert').innerText()).length > 8);
  assert.match(await page.locator('#status').innerText(), /Last successful drawing retained/);
  assert.equal(await page.locator('#preview').innerHTML(), lastGood);
  assert.equal(await renderNumber(), lastRender);
  proof.checks.push('Real parse error retains the exact last-good SVG');
  await hold(2);
  await choose('flowchart', 'Capture');
  assert.equal(await page.getByRole('alert').isVisible(), false);
  proof.checks.push('Corrected source recovers through the real renderer');

  const themeSVG = await page.locator('#preview').innerHTML();
  const themeRender = await renderNumber();
  const foreground = () => page.locator('#preview svg').evaluate(svg => getComputedStyle(svg).color);
  const light = await foreground();
  await page.locator('#dark').check();
  const dark = await foreground();
  assert.notEqual(dark, light, 'Dark CSS palette must affect the actual SVG.');
  assert.equal(await renderNumber(), themeRender);
  assert.equal(await page.locator('#preview').innerHTML(), themeSVG, 'Palette change should not reconstruct diagram SVG.');
  proof.checks.push('Dark CSS palette changes rendered colors without rerendering');
  await hold(2);
  await page.locator('#dark').uncheck();
  assert.equal(await foreground(), light);
  proof.checks.push('Light CSS palette restored');
  await page.locator('#source').blur();
  await hold(1);
  assert(requests.some(value => new URL(value).pathname === '/dist/package-web/package/full.js'));
  assert(requests.some(value => new URL(value).pathname.endsWith('/pkg/pele_bg.wasm')));
  assert(!requests.some(value => new URL(value).pathname.startsWith('/bindings/web/')), 'Demo must consume the packed artifact.');
  assert.deepEqual(failures, []);
  proof.checks.push('One browser, packed JS/WASM requests, no uncaught runtime or resource failures');
} finally {
  await browser?.close();
  clearTimeout(deadline);
  await writeFile(join(frames, 'recording.json'), JSON.stringify(proof, null, 2) + '\n');
}
assert.deepEqual(await artifacts(), before, 'Package changed during recording.');
if (!recording) {
  proof.finishedAt = new Date().toISOString();
  proof.succeeded = true;
  await writeFile(join(frames, 'recording.json'), JSON.stringify(proof, null, 2) + '\n');
  console.log(`Demo behavior verified without recording a GIF. Evidence: ${join(frames, 'recording.json')}`);
  process.exit(0);
}
const temporaryGIF = join(frames, 'full-demo.gif');
await execute('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', '6', '-i', join(frames, '%04d.png'),
  '-filter_complex', 'fps=6,scale=1024:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=none',
  '-loop', '0', temporaryGIF], {timeout: 60000, maxBuffer: 1024 * 1024});
assert((await stat(temporaryGIF)).size > 1024, 'GIF output is empty.');
await mkdir(join(root, 'docs/images'), {recursive: true});
await copyFile(temporaryGIF, output, constants.COPYFILE_EXCL);
proof.gifSHA256 = hash(await readFile(output));
proof.finishedAt = new Date().toISOString();
proof.succeeded = true;
await writeFile(join(frames, 'recording.json'), JSON.stringify(proof, null, 2) + '\n');
console.log(`Recorded ${proof.frames} actual browser frames. GIF: ${output}. Evidence: ${join(frames, 'recording.json')}`);
