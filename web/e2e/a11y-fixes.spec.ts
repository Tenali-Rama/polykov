import { expect, test } from '@playwright/test';
import { api, expectPhoneFit, expectShell, uid } from './helpers';

/** `n` active project JSONs, as `GET /api/projects` returns them. */
function projects(n: number, id: string) {
  const now = new Date().toISOString();
  return Array.from({ length: n }, (_, i) => ({
    id: 900_000 + i,
    type: 'project',
    title: `project ${String(i).padStart(2, '0')} ${id}`,
    tags: [],
    properties: {},
    pinned: false,
    archived: false,
    created: now,
    updated: now,
    project: { status: 'active', area_id: null, start: null, deadline: null, open_tasks: 0, done_tasks: 0 },
  }));
}

test('the skip link reaches the command line in 2 key presses past 30 projects', async ({ page }) => {
  const id = uid();
  // The 30 projects go to this page's nav only: real ones would lengthen every parallel test's
  // nav on the shared server and push their Tab flows past `tabTo`'s cap.
  await page.route('**/api/projects', (route) => route.fulfill({ json: { projects: projects(30, id) } }));
  // A row on the list, so its Enter key (open the selected row) would fire if the link let it through.
  await api(page.request).create(`note ${id}`);
  await page.goto('/#/');
  await expectShell(page);
  const nav = page.getByRole('navigation', { name: 'lists' });
  await expect(nav.getByRole('link', { name: `project 29 ${id}`, includeHidden: true })).toBeAttached();
  await expect(page.getByRole('link', { name: new RegExp(`note ${id}`) })).toBeVisible();

  await page.keyboard.press('Tab');
  const skip = page.getByRole('link', { name: 'skip to command line' });
  await expect(skip).toBeFocused();
  await expect(skip).toBeInViewport();
  await page.keyboard.press('Enter');
  await expect(page.locator('#command')).toBeFocused();
  expect(new URL(page.url()).hash).toBe('#/');
});

test('every checkbox on the object list is at least 44 × 44 px on phone', async ({ page }) => {
  test.skip(test.info().project.name !== 'phone', 'phone width only');
  await api(page.request).create(`note ${uid()}`);
  await page.goto('/#/');
  await expectShell(page);
  const boxes = page.locator('input[type=checkbox]');
  await expect(boxes.first()).toBeVisible();
  for (const box of await boxes.all()) {
    const r = await box.boundingBox();
    expect(r?.width).toBeGreaterThanOrEqual(44);
    expect(r?.height).toBeGreaterThanOrEqual(44);
  }
  await expectPhoneFit(page);
});
