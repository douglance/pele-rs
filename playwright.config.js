import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/web', workers: 1, timeout: 30000,
  use: { baseURL: 'http://127.0.0.1:5191', viewport: { width: 1440, height: 1000 } },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium', ...(process.env.CI ? {} : { channel: 'chrome' }) } },
    { name: 'firefox', use: { browserName: 'firefox' } },
    { name: 'webkit', use: { browserName: 'webkit' } }
  ],
  webServer: { command: 'node scripts/serve.mjs', url: 'http://127.0.0.1:5191/examples/web/', reuseExistingServer: true },
  reporter: 'list'
});
