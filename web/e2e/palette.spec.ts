import { expect, test, type Page } from '@playwright/test';
import { api, expectPhoneFit, expectShell, uid } from './helpers';

const palette = (page: Page) => page.getByRole('dialog', { name: 'palette' });
const input = (page: Page) => page.getByRole('combobox', { name: 'palette' });
const options = (page: Page) => palette(page).getByRole('option');

test('Ctrl+K finds an archived note and a done task by a misspelled word; Enter opens one', async ({ page }) => {
  // One 6+ character word shared by both titles; the query drops a letter from it (one typo).
  const id = uid();
  const word = `marmalade${id}`;
  const rest = api(page.request);
  const note = await rest.create(`jar ${word}`, { archived: true });
  const task = await rest.create(`buy ${word}`, { type: 'task' });
  const done = await page.request.post(`/api/tasks/${task.id}/complete?today=2026-01-01`);
  expect(done.ok()).toBe(true);

  await page.goto('/');
  await expectShell(page);
  await page.keyboard.press('Control+k');
  await expect(input(page)).toBeFocused();
  await page.keyboard.type(`marmlade${id}`);
  const noteRow = options(page).filter({ hasText: `jar ${word}` });
  const taskRow = options(page).filter({ hasText: `buy ${word}` });
  await expect(noteRow).toContainText('[archived]');
  await expect(taskRow).toContainText('[done]');
  await expect(noteRow.locator('mark')).toHaveText(word);

  // No command matches, so the first row is a result: move to the note and open it.
  const rows = await options(page).allTextContents();
  const target = rows.findIndex((t) => t.includes(`jar ${word}`));
  for (let i = 0; i < target; i++) await page.keyboard.press('ArrowDown');
  await expect(noteRow).toHaveAttribute('aria-selected', 'true');
  await page.keyboard.press('Enter');
  await expect(palette(page)).toBeHidden();
  await expect(page).toHaveURL(new RegExp(`#/o/${note.id}$`));
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(`jar ${word}`);
});

test('keyboard: Ctrl+Space, >, today, Enter lands on #/today; Esc restores focus', async ({ page }) => {
  await page.goto('/');
  await expectShell(page);
  const command = page.getByRole('textbox', { name: 'command' });
  await command.focus();
  await page.keyboard.press('Control+Space');
  await expect(page.getByRole('textbox', { name: 'capture' })).toBeFocused();
  await page.keyboard.type('>today');
  await expect(page.getByRole('dialog', { name: 'capture' })).toBeHidden();
  await expect(input(page)).toBeFocused();
  await expect(input(page)).toHaveValue('today');
  await expect(options(page).first()).toHaveText('go to today');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/#\/today$/);
  await expect(palette(page)).toBeHidden();

  await command.focus();
  await page.keyboard.press('Control+k');
  await expect(input(page)).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(input(page)).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(palette(page)).toBeHidden();
  await expect(command).toBeFocused();
});

test('phone: ten results fit and every row is at least 44 px tall', async ({ page }, info) => {
  test.skip(info.project.name !== 'phone', 'phone only');
  const word = `zebrafish${uid()}`;
  const rest = api(page.request);
  for (let i = 0; i < 12; i++) {
    await rest.create(`${word} with a rather long title that has to wrap on a phone ${i}`);
  }
  await page.goto('/');
  await expectShell(page);
  await page.keyboard.press('Control+k');
  await page.keyboard.type(word);
  await expect(options(page)).toHaveCount(10);
  await expectPhoneFit(page);
  const heights = await options(page).evaluateAll((rows) => rows.map((r) => r.getBoundingClientRect().height));
  expect(Math.min(...heights)).toBeGreaterThanOrEqual(44);
});
