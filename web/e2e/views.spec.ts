import { expect, test, type APIRequestContext } from '@playwright/test';
import { expectPhoneFit, tabTo, uid } from './helpers';

type Made = { id: number; title: string };

/** Task and view REST helpers for test data. */
function rest(request: APIRequestContext) {
  async function ok<T>(res: ReturnType<APIRequestContext['post']>): Promise<T> {
    const r = await res;
    if (!r.ok()) throw new Error(`${r.url()}: HTTP ${r.status()} ${await r.text()}`);
    return (await r.json()) as T;
  }
  return {
    task: (title: string, task: Record<string, unknown> = {}, tags: string[] = []) =>
      ok<Made>(request.post('/api/tasks', { data: { title, tags, task } })),
    view: (title: string, query: string, over: Record<string, unknown> = {}) =>
      ok<Made>(request.post('/api/views', { data: { title, query, ...over } })),
  };
}

test('keyboard: create a view grouped by priority, then complete a task in it', async ({ page }) => {
  const api = rest(page.request);
  const id = uid();
  // The oldest deadline puts it first in its group under the smart sort.
  await api.task(`alpha ${id}`, { priority: 1, deadline: '1990-01-01' });
  await api.task(`beta ${id}`, { priority: 2 });
  await api.task(`gamma ${id}`, { priority: 3 });

  await page.goto('/#/views');
  await tabTo(page, '[ new view ]');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('textbox', { name: 'title' })).toBeFocused();
  await page.keyboard.type(`urgent ${id}`);
  await page.keyboard.press('Tab');
  await expect(page.getByRole('textbox', { name: 'query' })).toBeFocused();
  await page.keyboard.type('type:task priority:<=2 -status:done');
  await page.keyboard.press('Tab');
  await page.keyboard.press('Tab');
  const group = page.getByRole('combobox', { name: 'group' });
  await expect(group).toBeFocused();
  // none → status → priority.
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await expect(group).toHaveValue('priority');
  await tabTo(page, '[ save ]');
  await page.keyboard.press('Enter');

  await expect(page).toHaveURL(/#\/v\/\d+$/);
  await expect(page.getByRole('heading', { name: `urgent ${id}` })).toBeVisible();
  const alpha = page.getByRole('list', { name: 'P1' }).getByRole('link', { name: `p1 alpha ${id}` });
  await expect(alpha).toBeVisible();
  await expect(page.getByRole('list', { name: 'P2' }).getByRole('link', { name: `p2 beta ${id}` })).toBeVisible();
  await expect(page.getByRole('link', { name: `p3 gamma ${id}` })).toHaveCount(0);

  // Parallel tests add tasks to these groups too: j until ours is selected, then x.
  for (let i = 0; i < 500 && !(await alpha.evaluate((el) => el === document.activeElement)); i++) {
    await page.keyboard.press('j');
  }
  await expect(alpha).toBeFocused();
  await page.keyboard.press('x');
  await expect(alpha).toHaveCount(0);
  await expect(page.getByRole('link', { name: `p2 beta ${id}` })).toBeVisible();
});

test('a bad query shows the server error with its offset and keeps the input', async ({ page }) => {
  await page.goto('/#/views');
  await page.getByRole('button', { name: '[ new view ]' }).click();
  await page.getByRole('textbox', { name: 'title' }).fill(`bad ${uid()}`);
  await page.getByRole('textbox', { name: 'query' }).fill('priority:<=');
  await page.getByRole('button', { name: '[ save ]' }).click();
  await expect(page.getByRole('alert')).toContainText('error: query:');
  await expect(page.getByText(/^\s*\^$/)).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'query' })).toHaveValue('priority:<=');
});

test.describe('phone fit', () => {
  test.beforeEach(() => {
    test.skip(test.info().project.name !== 'phone', 'phone width only');
  });

  test('views index', async ({ page }) => {
    const title = `pinned ${uid()} with a fairly long view title that wraps`;
    await rest(page.request).view(title, 'type:task tag:projects/client-alpha/very-long-tag-name', { pinned: true });
    await page.goto('/#/views');
    await expect(page.getByRole('list', { name: 'saved views' }).getByRole('link', { name: `* ${title}` })).toBeVisible();
    await expectPhoneFit(page);
  });

  test('the editor with an error', async ({ page }) => {
    await page.goto('/#/views');
    await page.getByRole('button', { name: '[ new view ]' }).click();
    await page.getByRole('textbox', { name: 'query' }).fill('type:task priority:<=');
    await page.getByRole('button', { name: '[ save ]' }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    await expectPhoneFit(page);
  });

  test('a view with 3 groups', async ({ page }) => {
    const api = rest(page.request);
    const tag = `v${uid()}`;
    await api.task(`one ${tag}`, { priority: 1, deadline: '1990-01-01' }, [tag]);
    await api.task(`two ${tag} with a fairly long title that wraps on a phone`, { priority: 2, scheduled: '2030-01-01' }, [tag]);
    await api.task(`three ${tag}`, { priority: 3 }, [tag]);
    const view = await api.view(`groups ${tag}`, `type:task tag:${tag}`, { group: 'priority' });
    await page.goto(`/#/v/${view.id}`);
    await expect(page.getByRole('heading', { level: 2 })).toHaveText(['P1 1', 'P2 1', 'P3 1']);
    await expectPhoneFit(page);
  });
});
