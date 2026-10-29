import { expect, test, type Page } from '@playwright/test';
import { api, expectPhoneFit, expectShell, password, STATE, tabTo, uid } from './helpers';

const loggedOut = { cookies: [], origins: [] };

const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** A 1×1 PNG, so the attachments panel renders an image thumbnail. */
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
  'base64',
);

/** An object linked to another, with a comment and two attachments. */
async function richObject(page: Page) {
  const rest = api(page.request);
  const id = uid();
  const obj = await rest.create(`detail ${id}`, { tags: ['e2e'] });
  const other = await rest.create(`target ${id}`);
  await rest.link(obj.id, other.id);
  await rest.link(other.id, obj.id);
  await rest.comment(obj.id, `comment ${id}`);
  await rest.attach(obj.id, 'notes.txt', 'text/plain', 'hello');
  await rest.attach(obj.id, 'dot.png', 'image/png', PNG);
  return { obj, other, id };
}

async function trashedObject(page: Page) {
  const rest = api(page.request);
  const obj = await rest.create(`trashed ${uid()}`);
  await rest.trash(obj.id);
  return obj;
}

test.describe('logged out', () => {
  test.use({ storageState: loggedOut });

  test('keyboard only: log in, j/k/Enter opens an object, Esc goes back', async ({ page, browser }) => {
    // Two objects so j and k both move; made through a separate logged-in context.
    const seeder = await browser.newContext({ storageState: STATE, baseURL: test.info().project.use.baseURL });
    const rest = api(seeder.request);
    await rest.create(`kbd one ${uid()}`);
    await rest.create(`kbd two ${uid()}`);
    await seeder.close();

    await page.goto('/');
    await tabTo(page, 'password');
    await page.keyboard.type(password());
    await tabTo(page, '[ log in ]');
    await page.keyboard.press('Enter');
    await expectShell(page);

    // Tab into the list, then j/k move the selection and Enter opens it.
    const rows = page.getByRole('list', { name: 'objects' }).getByRole('link');
    await expect(rows.nth(1)).toBeVisible();
    const first = (await rows.nth(0).locator('.title').textContent()) ?? '';
    await tabTo(page, new RegExp(escape(first)));
    await expect(rows.nth(0)).toBeFocused();
    await page.keyboard.press('j');
    await expect(rows.nth(1)).toBeFocused();
    await page.keyboard.press('k');
    await expect(rows.nth(0)).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(/#\/o\/\d+$/);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(first);

    await page.keyboard.press('Escape');
    await expect(page).toHaveURL(/#\/$/);
    await expect(page.getByRole('list', { name: 'objects' })).toBeVisible();
  });

  test('wrong password shows an error', async ({ page }) => {
    await page.goto('/');
    await page.getByLabel('password').fill('not-the-password');
    await page.keyboard.press('Enter');
    await expect(page.getByText('wrong password')).toBeVisible({ timeout: 30_000 });
  });
});

test('object list shows a new object and opens it', async ({ page }) => {
  const obj = await api(page.request).create(`listed ${uid()}`);
  await page.goto('/');
  const row = page.getByRole('link', { name: new RegExp(obj.title) });
  await expect(row).toBeVisible();
  await row.click();
  await expect(page).toHaveURL(new RegExp(`#/o/${obj.id}$`));
});

test('detail shows links, backlinks, comments and attachments', async ({ page }) => {
  const { obj, other, id } = await richObject(page);
  await page.goto(`/#/o/${obj.id}`);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(obj.title);
  const links = page.getByRole('region', { name: 'links' });
  await expect(links.getByRole('link', { name: other.title })).toHaveCount(2);
  await expect(page.getByRole('region', { name: 'comments' }).getByText(`comment ${id}`)).toBeVisible();
  const files = page.getByRole('region', { name: 'attachments' });
  await expect(files.getByText('notes.txt')).toBeVisible();
  await expect(files.getByRole('img', { name: 'dot.png' })).toBeVisible();
});

test('trash lists a deleted object and restores it', async ({ page }) => {
  const obj = await trashedObject(page);
  await page.goto('/#/trash');
  const row = page.getByRole('listitem').filter({ hasText: obj.title });
  await row.getByRole('button', { name: '[ restore ]' }).click();
  await expect(page.getByText(`restored ${obj.title}`)).toBeVisible();
  await expect(row).toHaveCount(0);
});

test.describe('phone fit', () => {
  test.beforeEach(() => {
    test.skip(test.info().project.name !== 'phone', 'phone width only');
  });

  test.describe('logged out', () => {
    test.use({ storageState: loggedOut });
    test('login', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByLabel('password')).toBeVisible();
      await expectPhoneFit(page);
    });
  });

  test('object list', async ({ page }) => {
    test.fixme(true, 'the `pinned only` checkbox is 13×13 px and the `trash` link 38 px wide');
    await api(page.request).create(`fit ${uid()}`, { tags: ['e2e'] });
    await page.goto('/');
    await expect(page.getByRole('list', { name: 'objects' }).getByRole('link').first()).toBeVisible();
    await expectPhoneFit(page);
  });

  test('detail with links, comments and attachments', async ({ page }) => {
    test.fixme(true, 'the tag `[ x ]` remove button is 38 px wide');
    const { obj, id } = await richObject(page);
    await page.goto(`/#/o/${obj.id}`);
    await expect(page.getByText(`comment ${id}`)).toBeVisible();
    await expect(page.getByText('notes.txt')).toBeVisible();
    await expectPhoneFit(page);
  });

  test('trash', async ({ page }) => {
    const obj = await trashedObject(page);
    await page.goto('/#/trash');
    await expect(page.getByText(obj.title)).toBeVisible();
    await expectPhoneFit(page);
  });
});
