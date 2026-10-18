// Record actual playground states after release verification. No rendered mock frames.
// Start `npm run serve` first. Requires ffmpeg on PATH.
import { chromium } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
const execute = promisify(execFile);
const frames = 'dist/demo-frames';
await mkdir(frames, { recursive: true });
await mkdir('docs/images', { recursive: true });
const browser = await chromium.launch(process.env.CI ? {} : { channel: 'chrome' });
let frame = 0;
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 1024 }, deviceScaleFactor: 1 });
  await page.goto('http://127.0.0.1:5191/examples/web/');
  await page.locator('#render').waitFor({ state: 'visible' });
  await page.waitForFunction(() => document.querySelector('#status').textContent.includes('5 nodes'));
  const hold = async seconds => {
    for (let i=0; i<seconds*6; i++) {
      await page.screenshot({ path: `${frames}/${String(frame++).padStart(4, '0')}.png` });
      await page.waitForTimeout(1000/6);
    }
  };
  await hold(2);
  await page.locator('#source').fill('flowchart LR\n    A[Capture] --> B{Keep?}\n    B -->|Yes| C([Store])\n    C -.->|Retry| A');
  await hold(1);
  await page.getByRole('button', { name: 'Render flowchart' }).click();
  await hold(2);
  await page.locator('#example').selectOption('cycles');
  await page.waitForFunction(() => document.querySelector('#status').textContent.includes('4 nodes'));
  await hold(2);
  await page.locator('#source').fill('flowchart TB\nsubgraph Unsupported\nA --> B\nend');
  await page.getByRole('button', { name: 'Render flowchart' }).click();
  await page.getByRole('alert').waitFor({ state: 'visible' });
  await hold(2);
  await page.locator('#example').selectOption('branch');
  await page.waitForFunction(() => document.querySelector('#status').textContent.includes('5 nodes'));
  await page.locator('#dark').check();
  await hold(2);
  await page.locator('#dark').uncheck();
  await page.locator('#source').blur();
  await hold(2);
} finally { await browser.close(); }
await execute('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', '6', '-i', `${frames}/%04d.png`,
  '-filter_complex', 'fps=6,scale=1024:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=none',
  '-loop', '0', 'docs/images/demo.gif']);
console.log(`Recorded ${frame} real browser frames to docs/images/demo.gif`);
