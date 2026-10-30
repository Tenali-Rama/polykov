import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { expect, type APIRequestContext, type APIResponse, type Locator, type Page } from '@playwright/test';

/** Saved logged-in storage state, written by `global-setup.ts`. */
export const STATE = join(import.meta.dirname, '.auth', 'state.json');

/** The owner password `scripts/e2e.sh` set up the server with. */
export function password(): string {
  const p = process.env.E2E_PASSWORD;
  if (!p) throw new Error('E2E_PASSWORD is not set; run the tests through scripts/e2e.sh');
  return p;
}

/** A short unique suffix, so titles from parallel tests never collide. */
export function uid(): string {
  return randomUUID().slice(0, 8);
}

/** Logs in through the login screen and waits for the shell. */
export async function login(page: Page) {
  await page.goto('/');
  await page.getByLabel('password').fill(password());
  await page.getByRole('button', { name: '[ log in ]' }).click();
  await expectShell(page);
}

/** Waits for the shell after a login; argon2 in a debug build takes seconds while tests run in parallel. */
export async function expectShell(page: Page) {
  await expect(page.getByRole('button', { name: 'logout' })).toBeVisible({ timeout: 30_000 });
}

type Obj = { id: number; title: string };

/** REST helpers for test data. Pass `page.request`, which shares the page's session cookie. */
export function api(request: APIRequestContext) {
  async function ok<T>(res: Promise<APIResponse>): Promise<T> {
    const r = await res;
    if (!r.ok()) throw new Error(`${r.url()}: HTTP ${r.status()} ${await r.text()}`);
    return (r.status() === 204 ? undefined : await r.json()) as T;
  }
  return {
    create: (title: string, over: Record<string, unknown> = {}) =>
      ok<Obj>(request.post('/api/objects', { data: { type: 'note', title, ...over } })),
    link: (from: number, to: number) => ok<void>(request.put(`/api/objects/${from}/links/${to}`)),
    comment: (id: number, body: string) => ok<{ id: number }>(request.post(`/api/objects/${id}/comments`, { data: { body } })),
    attach: (id: number, name: string, mime: string, body: string | Buffer) =>
      ok<{ id: number }>(
        request.post(`/api/objects/${id}/attachments?name=${encodeURIComponent(name)}`, {
          headers: { 'Content-Type': mime },
          data: body,
        }),
      ),
    trash: (id: number) => ok<void>(request.delete(`/api/objects/${id}`)),
  };
}

/**
 * Fails, naming each offending element, if the page scrolls horizontally or any visible
 * interactive element is smaller than 44×44 px.
 */
export async function expectPhoneFit(page: Page) {
  // Mobile Chromium widens the layout viewport (and `innerWidth`) to fit overflowing content,
  // so compare against the configured viewport width.
  const viewport = page.viewportSize()?.width ?? 0;
  const problems = await page.evaluate((viewport) => {
    const MIN = 44;
    const width = viewport || window.innerWidth;
    const describe = (el: Element) => {
      const name = el.getAttribute('aria-label') ?? (el.textContent ?? '').trim().replace(/\s+/g, ' ');
      const cls = el.classList.length > 0 ? `.${[...el.classList].join('.')}` : '';
      const type = el instanceof HTMLInputElement ? `[type=${el.type}]` : '';
      return `<${el.tagName.toLowerCase()}${type}${cls}> "${name.slice(0, 40)}"`;
    };
    const out: string[] = [];
    const doc = document.documentElement;
    if (doc.scrollWidth > width) {
      const over = (el: Element) => el.getBoundingClientRect().right > width + 0.5;
      // The innermost overflowing element is the one to fix.
      const leaf = [...document.body.querySelectorAll('*')].find((el) => over(el) && ![...el.children].some(over));
      out.push(
        `horizontal scroll: scrollWidth ${doc.scrollWidth} > width ${width}` +
          (leaf ? `, overflowing ${describe(leaf)}` : ''),
      );
    }
    for (const el of document.querySelectorAll('a, button, input, select, textarea, [role=button], [tabindex]')) {
      if (!el.checkVisibility()) continue;
      const r = el.getBoundingClientRect();
      if (r.width < MIN || r.height < MIN) {
        out.push(`${describe(el)} is ${Math.round(r.width)}×${Math.round(r.height)} px`);
      }
    }
    return out;
  }, viewport);
  if (problems.length > 0) throw new Error(`phone fit at ${page.url()}:\n  ${problems.join('\n  ')}`);
}

async function hasName(focused: Locator, name: string | RegExp): Promise<boolean> {
  if ((await focused.count()) !== 1) return false;
  try {
    // One check, no retrying: the caller presses Tab and asks again.
    await expect(focused).toHaveAccessibleName(name, { timeout: 1 });
    return true;
  } catch {
    return false;
  }
}

/** Presses Tab until the focused element's accessible name is (or matches) `name`; fails after 150
 * presses (the nav's favourites add a Tab stop per pinned object, and parallel tests pin some). */
export async function tabTo(page: Page, name: string | RegExp) {
  const focused = page.locator(':focus');
  for (let presses = 0; ; presses++) {
    if (await hasName(focused, name)) return;
    if (presses === 150) break;
    await page.keyboard.press('Tab');
  }
  throw new Error(`tabTo: nothing named ${JSON.stringify(String(name))} got focus after 150 Tab presses`);
}
