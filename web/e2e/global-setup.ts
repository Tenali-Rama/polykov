import { chromium, type FullConfig } from '@playwright/test';
import { login, STATE } from './helpers';

// Logs in once and saves the session cookie, so every test starts logged in.
export default async function globalSetup(config: FullConfig) {
  const browser = await chromium.launch();
  const page = await browser.newPage({ baseURL: config.projects[0].use.baseURL });
  await login(page);
  await page.context().storageState({ path: STATE });
  await browser.close();
}
