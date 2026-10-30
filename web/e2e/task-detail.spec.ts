import { expect, test, type Page } from '@playwright/test';
import { api, expectPhoneFit, tabTo, uid } from './helpers';

/** A 1×1 PNG, so the attachments panel renders an image thumbnail. */
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
  'base64',
);

/** The local date `days` from now as `YYYY-MM-DD` (the browser sends its local date too). */
function day(days = 0): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

type Made = { id: number; title: string };

async function task(page: Page, title: string, fields: Record<string, unknown> = {}): Promise<Made> {
  const r = await page.request.post('/api/tasks', { data: { title, task: fields } });
  if (!r.ok()) throw new Error(`POST /api/tasks: HTTP ${r.status()} ${await r.text()}`);
  return (await r.json()) as Made;
}

async function section(page: Page, project: number, title: string): Promise<number> {
  const r = await page.request.post(`/api/projects/${project}/sections`, { data: { title } });
  if (!r.ok()) throw new Error(`POST sections: HTTP ${r.status()} ${await r.text()}`);
  return ((await r.json()) as { id: number }).id;
}

/** A task with every field set, its project, area and parent, plus `checklist`. */
async function fullTask(page: Page, checklist: { text: string; done?: boolean }[]) {
  const rest = api(page.request);
  const id = uid();
  const project = await rest.create(`launch ${id} with a long project title`, { type: 'project' });
  const sid = await section(page, project.id, `plan ${id}`);
  const area = await rest.create(`health ${id}`, { type: 'area' });
  const parent = await task(page, `parent ${id}`);
  const t = await task(page, `full ${id}`, {
    priority: 2,
    scheduled: `${day(1)}T09:30`,
    deadline: `${day(5)}T17:00`,
    estimate: 90,
    someday: true,
    project_id: project.id,
    section_id: sid,
    area_id: area.id,
    parent_id: parent.id,
    recurrence: { rule: 'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH', from: 'completion' },
    checklist,
  });
  return { t, parent, id };
}

/** The focused element's accessible name, roughly: its `aria-label`, else its text. */
const focusedName = (page: Page) =>
  page.evaluate(() => {
    const el = document.activeElement;
    return el ? (el.getAttribute('aria-label') ?? (el.textContent ?? '').trim()) : '';
  });

test('phone: a task with every field, checklist, sub-tasks, a comment and an image fits', async ({ page }) => {
  test.skip(test.info().project.name !== 'phone', 'phone width only');
  const long = `${'a fairly long checklist item that keeps going '.repeat(6)}${'x'.repeat(24)}`.slice(0, 300);
  expect(long).toHaveLength(300);
  const { t, id } = await fullTask(page, [{ text: 'one' }, { text: 'two', done: true }, { text: long }, { text: 'four' }, { text: 'five' }]);
  await task(page, `child a ${id}`, { parent_id: t.id });
  await task(page, `child b ${id}`, { parent_id: t.id, priority: 1 });
  const rest = api(page.request);
  await rest.comment(t.id, `comment ${id}`);
  await rest.attach(t.id, 'dot.png', 'image/png', PNG);

  await page.goto(`/#/o/${t.id}`);
  await expect(page.getByText('every 2 weeks on mon, thu · from completion')).toBeVisible();
  await expect(page.getByRole('link', { name: `parent ${id}` })).toBeVisible();
  await expect(page.getByRole('button', { name: /^\[ \] a fairly long/ })).toBeVisible();
  await expect(page.getByRole('link', { name: `child b ${id}` })).toBeVisible();
  await expect(page.getByText(`comment ${id}`)).toBeVisible();
  await expect(page.getByRole('region', { name: 'attachments' }).getByRole('img', { name: 'dot.png' })).toBeVisible();
  await expect(page.getByLabel('section')).toHaveValue(/\d+/);
  await expectPhoneFit(page);
});

test('keyboard: Tab goes through every field in visual order', async ({ page }) => {
  const { t, id } = await fullTask(page, []);
  await page.goto(`/#/o/${t.id}`);
  await expect(page.getByText('every 2 weeks on mon, thu · from completion')).toBeVisible();
  await tabTo(page, '[ complete ]');
  const seen: string[] = [];
  for (let i = 0; i < 60 && seen.at(-1) !== 'sun'; i++) {
    const name = await focusedName(page);
    if (name !== seen.at(-1)) seen.push(name);
    await page.keyboard.press('Tab');
  }
  expect(seen).toEqual([
    '[ complete ]',
    'priority',
    'scheduled',
    'scheduled time',
    'clear scheduled',
    'deadline',
    'deadline time',
    'clear deadline',
    'estimate',
    'someday',
    'project',
    'section',
    'area',
    `parent ${id}`,
    'change parent',
    'clear parent',
    'repeat',
    'interval',
    'repeat from',
    'mon',
    'tue',
    'wed',
    'thu',
    'fri',
    'sat',
    'sun',
  ]);
  // Tabbing through changed nothing.
  const r = await page.request.get(`/api/tasks/${t.id}`);
  expect(((await r.json()) as { task: { estimate: number } }).task.estimate).toBe(90);
});

test('keyboard: add a checklist item with Enter, toggle it with Space, move it with Alt+arrows', async ({ page }) => {
  const t = await task(page, `list ${uid()}`, { checklist: [{ text: 'first' }] });
  await page.goto(`/#/o/${t.id}`);
  await expect(page.getByRole('button', { name: '[ ] first' })).toBeVisible();
  await tabTo(page, 'add item');
  await page.keyboard.type('second');
  await page.keyboard.press('Enter');
  const item = page.getByRole('button', { name: /^\[[ x]\] second$/ });
  await expect(item).toHaveText('[ ] second');
  await expect(page.getByRole('textbox', { name: 'add item' })).toBeFocused();
  for (let i = 0; i < 6 && !(await item.evaluate((el) => el === document.activeElement)); i++) {
    await page.keyboard.press('Shift+Tab');
  }
  await expect(item).toBeFocused();
  await page.keyboard.press('Space');
  await expect(item).toHaveText('[x] second');
  await expect(item).toBeFocused();
  await page.keyboard.press('Alt+ArrowUp');
  await expect(page.getByRole('list').filter({ has: item }).getByRole('listitem').first()).toContainText('second');
  await expect(item).toBeFocused();
  await page.keyboard.press('Alt+ArrowDown');
  await expect(page.getByRole('list').filter({ has: item }).getByRole('listitem').last()).toContainText('second');
  await expect(item).toBeFocused();
  const r = await page.request.get(`/api/tasks/${t.id}`);
  expect(((await r.json()) as { task: { checklist: unknown } }).task.checklist).toEqual([
    { text: 'first', done: false },
    { text: 'second', done: true },
  ]);
});

test('completing a recurring task from the panel shows the next date', async ({ page }) => {
  const t = await task(page, `weekly ${uid()}`, {
    scheduled: day(),
    recurrence: { rule: 'FREQ=WEEKLY', from: 'scheduled' },
  });
  await page.goto(`/#/o/${t.id}`);
  await expect(page.getByText('every week · from scheduled')).toBeVisible();
  await page.getByRole('button', { name: '[ complete ]' }).click();
  await expect(page.getByRole('link', { name: `next ${day(7)}` })).toBeVisible();
  await expect(page.getByRole('button', { name: '[ reopen ]' })).toBeVisible();
});

test('keyboard: a rejected interval shows the error and the saved interval again', async ({ page }) => {
  const t = await task(page, `bad interval ${uid()}`, { recurrence: { rule: 'FREQ=WEEKLY;INTERVAL=2', from: 'scheduled' } });
  await page.goto(`/#/o/${t.id}`);
  await expect(page.getByText('every 2 weeks · from scheduled')).toBeVisible();
  await tabTo(page, 'interval');
  await page.keyboard.press('ControlOrMeta+a');
  await page.keyboard.type('0');
  await page.keyboard.press('Tab');
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByRole('spinbutton', { name: 'interval' })).toHaveValue('2');
  // Focus moved on with Tab and stays there: the revert doesn't re-render the editor.
  await expect(page.getByRole('combobox', { name: 'repeat from' })).toBeFocused();
  const r = await page.request.get(`/api/tasks/${t.id}`);
  expect(((await r.json()) as { task: { recurrence: unknown } }).task.recurrence).toEqual({
    rule: 'FREQ=WEEKLY;INTERVAL=2',
    from: 'scheduled',
  });
});
