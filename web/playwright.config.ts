import { defineConfig } from '@playwright/test';
import { STATE } from './e2e/helpers';

// Run through `scripts/e2e.sh`, which starts a fresh server and sets BASE_URL and E2E_PASSWORD.
export default defineConfig({
  testDir: 'e2e',
  globalSetup: './e2e/global-setup.ts',
  fullyParallel: true,
  forbidOnly: true,
  reporter: 'line',
  use: {
    baseURL: process.env.BASE_URL,
    storageState: STATE,
    browserName: 'chromium',
    trace: 'retain-on-failure',
  },
  projects: [
    { name: 'phone', use: { viewport: { width: 360, height: 740 }, isMobile: true, hasTouch: true } },
    { name: 'desktop', use: { viewport: { width: 1280, height: 800 } } },
  ],
});
