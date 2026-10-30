import { expect, test, type APIRequestContext, type Page } from '@playwright/test';
import { expectPhoneFit, tabTo, uid } from './helpers';

/** The local date `days` from now as `YYYY-MM-DD` (the browser sends its local date too). */
function day(days = 0): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

type Made = { id: number; title: string };

/** Task, project and section REST helpers for test data. */
function rest(request: APIRequestContext) {
  async function ok<T>(res: ReturnType<APIRequestContext['post']>): Promise<T> {
    const r = await res;
    if (!r.ok()) throw new Error(`${r.url()}: HTTP ${r.status()} ${await r.text()}`);
    return (await r.json()) as T;
  }
  return {
    task: (title: string, task: Record<string, unknown> = {}, tags: string[] = []) =>
      ok<Made>(request.post('/api/tasks', { data: { title, tags, task } })),
    project: (title: string) => ok<Made>(request.post('/api/objects', { data: { type: 'project', title } })),
    section: (project: number, title: string) =>
      ok<Made>(request.post(`/api/projects/${project}/sections`, { data: { title } })),
  };
}

/** Today with 3 tasks: one overdue, one with sub-tasks, one with long tags. */
async function todayTasks(page: Page) {
  const api = rest(page.request);
  const id = uid();
  await api.task(`overdue ${id}`, { priority: 2, deadline: day(-2) });
  const parent = await api.task(`parent ${id}`, { priority: 3, scheduled: day() });
  await api.task(`child a ${id}`, { parent_id: parent.id });
  await api.task(`child b ${id}`, { parent_id: parent.id });
  await api.task(`tagged ${id} with a fairly long title that wraps`, { scheduled: day(), deadline: day(3) }, [
    'projects/client-alpha/very-long-tag-name',
    'waiting-for-someone-else-entirely',
  ]);
  return id;
}

async function projectWithSections(page: Page) {
  const api = rest(page.request);
  const id = uid();
  const project = await api.project(`launch ${id}`);
  const plan = await api.section(project.id, `plan ${id}`);
  const build = await api.section(project.id, `build ${id}`);
  await api.task(`loose ${id}`, { project_id: project.id, priority: 1, deadline: day(-1) });
  await api.task(`research ${id}`, { project_id: project.id, section_id: plan.id, scheduled: day(1) }, ['research']);
  await api.task(`code ${id}`, { project_id: project.id, section_id: build.id });
  return { project, id };
}

test.describe('phone fit', () => {
  test.beforeEach(() => {
    test.skip(test.info().project.name !== 'phone', 'phone width only');
  });

  test('today with overdue, sub-tasks and long tags', async ({ page }) => {
    const id = await todayTasks(page);
    await page.goto('/#/today');
    await expect(page.getByRole('link', { name: `parent ${id}` })).toBeVisible();
    await expect(page.getByText('[2/2]').first()).toBeVisible();
    await expect(page.getByText(`due ${day(-2)}`).first()).toBeVisible();
    await expectPhoneFit(page);
  });

  test('upcoming', async ({ page }) => {
    const api = rest(page.request);
    const id = uid();
    await api.task(`soon ${id}`, { scheduled: day(2) }, ['next']);
    await api.task(`later ${id}`, { deadline: day(9) });
    await page.goto('/#/upcoming');
    await expect(page.getByRole('heading', { name: day(2) })).toBeVisible();
    await expect(page.getByRole('link', { name: `later ${id}` })).toBeVisible();
    await expectPhoneFit(page);
  });

  test('project with two sections', async ({ page }) => {
    const { project, id } = await projectWithSections(page);
    await page.goto(`/#/p/${project.id}`);
    await expect(page.getByRole('heading', { name: `build ${id}` })).toBeVisible();
    await expect(page.getByRole('link', { name: /loose/ })).toBeVisible();
    await expectPhoneFit(page);
  });

  test('nav menu open', async ({ page }) => {
    await rest(page.request).project(`nav ${uid()} with a long project name to wrap`);
    await page.goto('/#/today');
    await page.getByRole('button', { name: 'lists' }).click();
    await expect(page.getByRole('link', { name: 'anytime', exact: true })).toBeVisible();
    await expectPhoneFit(page);
  });

  test('keyboard: open the menu, go to today, j then x completes the task', async ({ page }) => {
    // P1 with the oldest deadline sorts first in today.
    const title = `kbd ${uid()}`;
    await rest(page.request).task(title, { priority: 1, deadline: '1990-01-01' });
    await page.goto('/#/inbox');
    await tabTo(page, 'lists');
    await page.keyboard.press('Enter');
    // Favourites pinned by parallel tests come first in the menu; arrow down past them.
    const inbox = page.getByRole('link', { name: 'inbox', exact: true });
    for (let i = 0; i < 200 && !(await inbox.evaluate((el) => el === document.activeElement)); i++) {
      await page.keyboard.press('ArrowDown');
    }
    await expect(inbox).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(page.getByRole('link', { name: 'today', exact: true })).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(/#\/today$/);
    await expect(page.getByRole('heading', { name: 'today' })).toBeVisible();
    await expect(page.getByRole('link', { name: title })).toBeVisible();
    await page.keyboard.press('j');
    await expect(page.getByRole('link', { name: title })).toBeFocused();
    await page.keyboard.press('x');
    await expect(page.getByRole('button', { name: `reopen ${title}` })).toHaveText('[x]');
  });
});

test.describe('desktop', () => {
  test.beforeEach(() => {
    test.skip(test.info().project.name !== 'desktop', 'desktop width only');
  });

  test('the nav is visible without the toggle', async ({ page }) => {
    await page.goto('/#/upcoming');
    await expect(page.getByRole('button', { name: 'lists' })).toBeHidden();
    const nav = page.getByRole('navigation', { name: 'lists' });
    for (const name of ['inbox', 'today', 'upcoming', 'someday', 'anytime', 'objects', 'trash']) {
      await expect(nav.getByRole('link', { name, exact: true })).toBeVisible();
    }
    await expect(nav.getByRole('link', { name: 'upcoming', exact: true })).toHaveAttribute('aria-current', 'page');
  });

  test('a project: add a section and complete a task', async ({ page }) => {
    const { project, id } = await projectWithSections(page);
    await page.goto(`/#/p/${project.id}`);
    await page.getByRole('button', { name: '[ add section ]' }).click();
    await page.getByRole('textbox', { name: 'section title' }).fill(`ship ${id}`);
    await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { name: `ship ${id}` })).toBeVisible();
    await page.getByRole('button', { name: `complete code ${id}` }).click();
    await expect(page.getByRole('button', { name: `reopen code ${id}` })).toHaveText('[x]');
  });
});
