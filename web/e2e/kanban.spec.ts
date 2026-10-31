import { expect, test, type APIRequestContext } from '@playwright/test';
import { expectPhoneFit, tabTo, uid } from './helpers';

type Made = { id: number; title: string };
type TaskJson = Made & { task: { status: string; priority: number } };

/** Task and view REST helpers for test data. */
function rest(request: APIRequestContext) {
  async function ok<T>(res: ReturnType<APIRequestContext['post']>): Promise<T> {
    const r = await res;
    if (!r.ok()) throw new Error(`${r.url()}: HTTP ${r.status()} ${await r.text()}`);
    return (await r.json()) as T;
  }
  return {
    task: (title: string, task: Record<string, unknown>, tags: string[]) =>
      ok<Made>(request.post('/api/tasks', { data: { title, tags, task } })),
    get: (id: number) => ok<TaskJson>(request.get(`/api/tasks/${id}`)),
    view: (title: string, query: string, group: string) =>
      ok<Made>(request.post('/api/views', { data: { title, query, layout: 'kanban', group } })),
  };
}

test('keyboard only: Shift+ArrowLeft moves a card from P3 to P2', async ({ page }) => {
  const api = rest(page.request);
  const tag = `k${uid()}`;
  const card = await api.task(`move me ${tag}`, { priority: 3 }, [tag]);
  await api.task(`stay ${tag}`, { priority: 1 }, [tag]);
  const view = await api.view(`board ${tag}`, `type:task tag:${tag}`, 'priority');

  await page.goto(`/#/v/${view.id}`);
  await expect(page.getByRole('heading', { level: 2 })).toHaveText(['P1 1', 'P2 0', 'P3 1', 'P4 0']);
  await tabTo(page, `p3 move me ${tag}`);
  await page.keyboard.press('Shift+ArrowLeft');
  const moved = page.getByRole('list', { name: 'P2' }).getByRole('link', { name: `p2 move me ${tag}` });
  await expect(moved).toBeFocused();
  await expect.poll(async () => (await api.get(card.id)).task.priority).toBe(2);

  await page.reload();
  await expect(page.getByRole('list', { name: 'P2' }).getByRole('link', { name: `p2 move me ${tag}` })).toBeVisible();
  await expect(page.getByRole('heading', { level: 2 })).toHaveText(['P1 1', 'P2 1', 'P3 0', 'P4 0']);
  // Enter opens the focused card.
  await tabTo(page, `p2 move me ${tag}`);
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(new RegExp(`#/o/${card.id}$`));
});

test('dragging a card to the done column completes the task', async ({ page }) => {
  test.fixme(
    test.info().project.name === 'phone',
    'touch drag: HTML5 drag-and-drop does not start from touch input; touch moves come with M14 swipe actions',
  );
  const api = rest(page.request);
  const tag = `k${uid()}`;
  const card = await api.task(`drag me ${tag}`, {}, [tag]);
  // No grouping: a kanban view groups by status.
  const view = await api.view(`status ${tag}`, `type:task tag:${tag}`, 'none');

  await page.goto(`/#/v/${view.id}`);
  const link = page.getByRole('list', { name: 'open' }).getByRole('link', { name: `drag me ${tag}` });
  await link.dragTo(page.getByRole('list', { name: 'done' }));
  await expect(page.getByRole('list', { name: 'done' }).getByRole('link', { name: `drag me ${tag}` })).toBeVisible();
  await expect.poll(async () => (await api.get(card.id)).task.status).toBe('done');
});

test('phone fit: the board scrolls inside its box', async ({ page }) => {
  test.skip(test.info().project.name !== 'phone', 'phone width only');
  const api = rest(page.request);
  const tag = `k${uid()}`;
  await api.task(`one ${tag} with a fairly long title that wraps on a phone`, { priority: 1, deadline: '2030-01-01' }, [tag]);
  await api.task(`two ${tag}`, { priority: 3, scheduled: '2030-01-02' }, [tag]);
  const view = await api.view(`phone ${tag}`, `type:task tag:${tag}`, 'priority');

  await page.goto(`/#/v/${view.id}`);
  await expect(page.getByRole('heading', { level: 2 })).toHaveText(['P1 1', 'P2 0', 'P3 1', 'P4 0']);
  await expectPhoneFit(page);
  const sizes = await page.locator('.card').evaluateAll((els) => els.map((el) => el.getBoundingClientRect().height));
  expect(sizes.length).toBe(2);
  for (const h of sizes) expect(h).toBeGreaterThanOrEqual(44);
  const widths = await page.locator('.col').evaluateAll((els) => els.map((el) => el.getBoundingClientRect().width));
  for (const w of widths) expect(w).toBeGreaterThanOrEqual(260);
  const board = await page.locator('.board').evaluate((el) => ({
    scrolls: el.scrollWidth > el.clientWidth,
    snap: getComputedStyle(el).scrollSnapType,
  }));
  expect(board).toEqual({ scrolls: true, snap: 'x mandatory' });
});
