import { defineConfig, devices } from '@playwright/test';

// File mode also exercises the production bundle in environments without TCP listeners.
const filePreview = process.env.PUDDING_E2E_FILE;

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  timeout: 30_000,
  use: {
    baseURL: 'http://127.0.0.1:1420',
    viewport: { width: 1280, height: 820 },
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'], channel: process.env.CI ? undefined : 'chrome', launchOptions: { args: filePreview ? ['--allow-file-access-from-files'] : [] }, viewport: { width: 1280, height: 820 } } }],
  webServer: filePreview ? undefined : { command: 'npm run dev', url: 'http://127.0.0.1:1420', reuseExistingServer: !process.env.CI },
});
