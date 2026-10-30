import { expect, test, type Page } from '@playwright/test';
import { api, expectPhoneFit, tabTo, uid } from './helpers';

// Pinned objects show in every page's nav, so each test trashes the ones it pinned when done.

/** The favourites list in the nav; on phone it opens the `[ lists ]` menu first. */
async function favourites(page: Page) {
  const toggle = page.getByRole('button', { name: 'lists' });
  if ((await toggle.isVisible()) && (await toggle.getAttribute('aria-expanded')) === 'false') await toggle.click();
  return page.getByRole('navigation', { name: 'lists' }).getByRole('list', { name: 'favourites' });
}

test('keyboard: pin a note in its detail, it shows under favourites; unpin removes it', async ({ page }) => {
  const title = `fav ${uid()}`;
  const note = await api(page.request).create(title);
  await page.goto(`/#/o/${note.id}`);
  await expect(page.getByRole('heading', { name: title })).toBeVisible();
  await tabTo(page, '[ pin ]');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: '[ unpin ]' })).toBeFocused();

  const link = (await favourites(page)).getByRole('link', { name: `note ${title}` });
  await expect(link).toHaveAttribute('href', `#/o/${note.id}`);
  await expect(link).toHaveAttribute('aria-current', 'page');

  if (test.info().project.name === 'phone') await page.keyboard.press('Escape'); // close the menu
  await tabTo(page, '[ unpin ]');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('button', { name: '[ pin ]' })).toBeFocused();
  await expect((await favourites(page)).getByRole('link', { name: `note ${title}` })).toHaveCount(0);
});

test('a pinned view links to its view route', async ({ page }) => {
  const title = `view ${uid()}`;
  const rest = api(page.request);
  const properties = { query: 'type:task', layout: 'list', group: 'none', sort: 'smart' };
  const view = await rest.create(title, { type: 'view', pinned: true, properties });
  await page.goto('/#/inbox');
  const link = (await favourites(page)).getByRole('link', { name: `view ${title}` });
  await expect(link).toHaveAttribute('href', `#/v/${view.id}`);
  await rest.trash(view.id);
});

test('phone: the open menu with 5 favourites fits', async ({ page }) => {
  test.skip(test.info().project.name !== 'phone', 'phone width only');
  const id = uid();
  const rest = api(page.request);
  const made = [];
  for (const [type, title] of [
    ['note', `note ${id}`],
    ['project', `project ${id} with a fairly long name that has to wrap`],
    ['area', `area ${id}`],
    ['task', `task ${id}`],
    ['journal_entry', `journal ${id}`],
  ]) {
    made.push(await rest.create(title, { type, pinned: true }));
  }
  await page.goto('/#/today');
  const favs = await favourites(page);
  await expect(favs.getByRole('link', { name: new RegExp(`journal ${id}`) })).toBeVisible();
  await expect(favs.getByRole('link', { name: new RegExp(id) })).toHaveCount(5);
  await expectPhoneFit(page);
  for (const o of made) await rest.trash(o.id);
});
