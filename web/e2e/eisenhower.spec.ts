import { expect, test, type APIRequestContext, type Page } from '@playwright/test';
import { expectPhoneFit, tabTo, uid } from './helpers';

type Made = { id: number; title: string };

/** Local date `days` from today, as the web sends it in `?today=`. */
function day(days: number): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

async function ok(res: ReturnType<APIRequestContext['post']>): Promise<Made> {
  const r = await res;
  if (!r.ok()) throw new Error(`${r.url()}: HTTP ${r.status()} ${await r.text()}`);
  return (await r.json()) as Made;
}

/** Tagged tasks `[title, priority, deadline]` and an Eisenhower view of them, opened. */
async function seed(page: Page, tasks: [string, number, string][]) {
  const tag = `e${uid()}`;
  for (const [title, priority, deadline] of tasks) {
    await ok(
      page.request.post('/api/tasks', { data: { title: `${title} ${tag}`, tags: [tag], task: { priority, deadline } } }),
    );
  }
  const view = await ok(
    page.request.post('/api/views', {
      data: { title: `matrix ${tag}`, query: `type:task tag:${tag}`, layout: 'eisenhower' },
    }),
  );
  await page.goto(`/#/v/${view.id}`);
  return tag;
}

/** One task per quadrant. Urgent: a deadline within 2 days; important: P1–P2. */
async function seedAll(page: Page) {
  const tag = await seed(page, [
    ['do', 1, day(0)],
    ['schedule', 2, day(30)],
    ['delegate', 3, day(1)],
    ['drop', 4, day(30)],
  ]);
  await expect(page.getByRole('heading', { level: 2 })).toHaveText([
    'urgent & important 1',
    'important 1',
    'urgent 1',
    'neither 1',
  ]);
  return tag;
}

const QUADRANTS = [
  ['urgent & important', 'do'],
  ['important', 'schedule'],
  ['urgent', 'delegate'],
  ['neither', 'drop'],
] as const;

const region = (page: Page, name: string) => page.getByRole('region', { name, exact: true });

test('each task lands in its quadrant; Tab goes quadrant by quadrant and Enter completes', async ({ page }) => {
  const tag = await seedAll(page);
  for (const [quadrant, title] of QUADRANTS) {
    await expect(region(page, quadrant).getByRole('link', { name: new RegExp(`${title} ${tag}$`) })).toBeVisible();
  }

  await tabTo(page, `complete do ${tag}`);
  const focused = page.locator(':focus');
  for (const [, title] of QUADRANTS) {
    await expect(focused).toHaveAccessibleName(`complete ${title} ${tag}`);
    await page.keyboard.press('Tab');
    await expect(focused).toHaveAccessibleName(new RegExp(`${title} ${tag}$`));
    await page.keyboard.press('Tab');
  }

  await tabTo(page, `complete drop ${tag}`);
  await page.keyboard.press('Enter');
  await expect(region(page, 'neither').getByRole('button', { name: `reopen drop ${tag}` })).toBeVisible();
});

test('2×2 on wide screens, one column on a phone', async ({ page }) => {
  await seedAll(page);
  const boxes = await Promise.all(QUADRANTS.map(([name]) => region(page, name).boundingBox()));
  const [ui, i, u, n] = boxes.map((b) => b!);
  if (test.info().project.name === 'phone') {
    expect([i.x, u.x, n.x]).toEqual([ui.x, ui.x, ui.x]);
    expect(ui.y < i.y && i.y < u.y && u.y < n.y).toBe(true);
    await expectPhoneFit(page);
  } else {
    expect([ui.y, u.x]).toEqual([i.y, ui.x]);
    expect([n.y, n.x]).toEqual([u.y, i.x]);
    expect(i.x).toBeGreaterThan(ui.x);
  }
});

test('an empty quadrant says empty and keeps its size', async ({ page }) => {
  await seed(page, [['only', 1, day(0)]]);
  await expect(region(page, 'urgent').getByText('empty')).toBeVisible();
  const [full, empty] = await Promise.all(['urgent & important', 'urgent'].map((name) => region(page, name).boundingBox()));
  const rem = await page.evaluate(() => parseFloat(getComputedStyle(document.documentElement).fontSize));
  // At least 8rem tall, and as wide as a full quadrant.
  expect(empty!.height).toBeGreaterThanOrEqual(8 * rem);
  expect(empty!.width).toBe(full!.width);
});
