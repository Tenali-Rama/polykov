import { expect, test, type Page } from '@playwright/test';
import { api, expectPhoneFit, expectShell, tabTo, uid } from './helpers';

const LINE = 'tmrw 5pm 30m p2 #family every sunday';

const localToday = (page: Page) =>
  page.evaluate(() => {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  });

async function upcomingTitles(page: Page): Promise<string[]> {
  const res = await page.request.get(`/api/lists/upcoming?today=${await localToday(page)}&limit=1000`);
  expect(res.ok()).toBe(true);
  const { tasks } = (await res.json()) as { tasks: { title: string }[] };
  return tasks.map((t) => t.title);
}

const bar = (page: Page) => page.getByRole('dialog', { name: 'capture' });
const input = (page: Page) => page.getByRole('textbox', { name: 'capture' });
const chips = (page: Page) => page.getByRole('list', { name: 'parsed' }).getByRole('listitem');

/** Opens the bar, types the smart-add line, waits for all six chips and saves; the URL stays put. */
async function capture(page: Page) {
  const title = `call mom x${uid()}`;
  const url = page.url();
  await page.keyboard.press('Control+Space');
  await expect(input(page)).toBeFocused();
  await page.keyboard.type(`${title} ${LINE}`);
  await expect(chips(page)).toHaveCount(6);
  await expect(chips(page).first()).toContainText('tomorrow');
  await page.keyboard.press('Enter');
  await expect(bar(page)).toBeHidden();
  await expect(page.getByRole('status').filter({ hasText: `added: ${title} · upcoming` })).toBeVisible();
  expect(page.url()).toBe(url);
  expect(await upcomingTitles(page)).toContain(title);
}

test('from the object list, Ctrl+Space captures a task into Upcoming', async ({ page }) => {
  await page.goto('/');
  await expectShell(page);
  await capture(page);
});

test('from a task detail page, Ctrl+Space captures a task into Upcoming', async ({ page }) => {
  const task = await api(page.request).create(`detail task ${uid()}`, { type: 'task' });
  await page.goto(`/#/o/${task.id}`);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(task.title);
  await capture(page);
});

test('phone: the open bar with all chips fits, and a long line does not scroll the page', async ({ page }, info) => {
  test.skip(info.project.name !== 'phone', 'phone only');
  await page.goto('/');
  await expectShell(page);
  await page.keyboard.press('Control+Space');
  await input(page).fill(`call mom ${LINE}`);
  await expect(chips(page)).toHaveCount(6);
  await expectPhoneFit(page);

  await input(page).fill('x'.repeat(200));
  await expect(input(page)).toHaveValue('x'.repeat(200));
  const scroll = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(scroll).toBeLessThanOrEqual(360);
});

test('keyboard only: open from another input, type, unparse a chip, Esc restores focus', async ({ page }) => {
  await page.goto('/');
  await expectShell(page);
  // Start with focus in another input (the Shell command line); the list above it has many stops.
  const command = page.getByRole('textbox', { name: 'command' });
  await command.focus();
  await page.keyboard.press('Control+Space');
  await expect(input(page)).toBeFocused();
  const title = `kbd capture ${uid()}`;
  await page.keyboard.type(`${title} p2`);
  await tabTo(page, 'unparse p2');
  await page.keyboard.press('Enter');
  await expect(input(page)).toHaveValue(`${title} "p2"`);
  await expect(input(page)).toBeFocused();
  await expect(page.getByRole('button', { name: 'unparse p2' })).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(bar(page)).toBeHidden();
  await expect(command).toBeFocused();
  expect(page.url()).toMatch(/\/(#\/)?$/);
});
