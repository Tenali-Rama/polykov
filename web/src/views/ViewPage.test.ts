import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { makeObj, stubApi, urls, type Route } from '../objects/testing';
import { makeRow } from '../tasks/testing';
import type { Group, View } from '../lib/views';
import ViewPage from './ViewPage.svelte';

const view = (over: Partial<View> = {}): View => ({
  ...makeObj(4, { type: 'view', title: 'Urgent' }),
  properties: { query: 'type:task priority:<=2', layout: 'list', group: 'priority', sort: 'smart' },
  ...over,
});
// Items are the task JSON: no sub-task counts.
const task = (id: number, priority: number) => ({
  ...makeObj(id, { type: 'task', title: `task ${id}` }),
  task: makeRow(id, { priority }).task,
});
const GROUPS: Group[] = [
  { key: 1, label: 'P1', items: [task(1, 1), task(2, 1)] },
  { key: 2, label: 'P2', items: [task(3, 2)] },
  { key: null, label: 'none', items: [makeObj(5, { title: 'Idea' })] },
];
const body = (init?: RequestInit) => JSON.parse(String(init?.body));

function api(groups: Group[] = GROUPS, more: Record<string, Route> = {}, extra = {}) {
  return stubApi({
    'GET /api/objects/4': () => Response.json(view()),
    'GET /api/views/4/items': () =>
      Response.json({ view: view(), groups, total: groups.flatMap((g) => g.items).length, truncated: false, ...extra }),
    ...more,
  });
}

async function page() {
  render(ViewPage, { id: 4 });
  await screen.findByRole('heading', { name: 'Urgent' });
}

const group = (name: string) => within(screen.getByRole('list', { name }));

afterEach(() => {
  vi.unstubAllGlobals();
  location.hash = '';
});

describe('ViewPage', () => {
  it('shows the title, query and one section per group with its label and count', async () => {
    const fetch = api();
    await page();
    expect(screen.getByText('> type:task priority:<=2')).toBeInTheDocument();
    await screen.findByRole('list', { name: 'P1' });
    expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toEqual(['P1 2', 'P2 1', 'none 1']);
    expect(group('P1').getAllByRole('button').map((b) => b.getAttribute('aria-label'))).toEqual([
      'complete task 1',
      'complete task 2',
    ]);
    expect(group('P2').getByRole('link', { name: 'p2 task 3' })).toHaveAttribute('href', '#/o/3');
    expect(group('none').getByText('note')).toBeInTheDocument();
    expect(group('none').getByRole('link', { name: 'Idea' })).toHaveAttribute('href', '#/o/5');
    expect(urls(fetch, '/api/views/4/items')[0]).toMatch(/\?today=\d{4}-\d{2}-\d{2}$/);
    expect(screen.queryByText(/showing/)).toBeNull();
  });

  it('shows the empty state when nothing matches', async () => {
    api([]);
    await page();
    expect(await screen.findByText('nothing matches')).toBeInTheDocument();
    expect(screen.queryByRole('heading', { level: 2 })).toBeNull();
  });

  it('says how many are shown when the results are truncated', async () => {
    api(GROUPS, {}, { total: 812, truncated: true });
    await page();
    expect(await screen.findByText('showing 4 of 812')).toBeInTheDocument();
  });

  it('pin toggles through PATCH and tells the favourites', async () => {
    let pinned = false;
    const fetch = api(GROUPS, {
      'PATCH /api/views/4': (_, init) => {
        pinned = body(init).pinned;
        return Response.json(view({ pinned }));
      },
    });
    const changed = vi.fn();
    window.addEventListener('polykov:objects-changed', changed);
    await page();
    await fireEvent.click(screen.getByRole('button', { name: '[ pin ]' }));
    expect(await screen.findByRole('button', { name: '[ unpin ]' })).toBeInTheDocument();
    expect(pinned).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: '[ unpin ]' }));
    expect(await screen.findByRole('button', { name: '[ pin ]' })).toBeInTheDocument();
    window.removeEventListener('polykov:objects-changed', changed);
    expect(urls(fetch, '/api/views/4').filter((u) => !u.includes('items'))).toHaveLength(2);
    expect(changed).toHaveBeenCalledTimes(2);
  });

  it('shows a pin failure inline', async () => {
    api(GROUPS, { 'PATCH /api/views/4': () => Response.json({ error: 'view not found' }, { status: 404 }) });
    await page();
    await fireEvent.click(screen.getByRole('button', { name: '[ pin ]' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('error: view not found');
  });

  it('edit reveals the editor; a save hides it and reloads the results', async () => {
    const fetch = api(GROUPS, { 'PATCH /api/views/4': () => Response.json(view({ title: 'Now' })) });
    await page();
    await screen.findByRole('list', { name: 'P1' });
    const edit = screen.getByRole('button', { name: '[ edit ]' });
    expect(screen.queryByRole('form')).toBeNull();
    await fireEvent.click(edit);
    expect(edit).toHaveAttribute('aria-expanded', 'true');
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    expect(await screen.findByRole('heading', { name: 'Now' })).toBeInTheDocument();
    expect(screen.queryByRole('form')).toBeNull();
    expect(edit).toHaveFocus();
    await waitFor(() => expect(urls(fetch, '/api/views/4/items')).toHaveLength(2));
  });

  it('reloads on polykov:tasks-changed', async () => {
    const fetch = api();
    await page();
    await screen.findByRole('list', { name: 'P1' });
    window.dispatchEvent(new Event('polykov:tasks-changed'));
    await waitFor(() => expect(urls(fetch, '/api/views/4/items')).toHaveLength(2));
  });

  it('j selects the first item and x completes it, then reloads', async () => {
    const fetch = api(GROUPS, {
      'POST /api/tasks/1/complete': () => Response.json({ task: task(1, 1), next: null }),
    });
    await page();
    await screen.findByRole('list', { name: 'P1' });
    await fireEvent.keyDown(window, { key: 'j' });
    expect(screen.getByRole('link', { name: 'p1 task 1' })).toHaveFocus();
    await fireEvent.keyDown(window, { key: 'x' });
    await waitFor(() => expect(urls(fetch, '/api/tasks/1/complete')).toHaveLength(1));
    await waitFor(() => expect(urls(fetch, '/api/views/4/items')).toHaveLength(2));
  });

  it('says view not found for a missing id or another type', async () => {
    stubApi({ 'GET /api/objects/4': () => Response.json(makeObj(4)) });
    render(ViewPage, { id: 4 });
    expect(await screen.findByRole('alert')).toHaveTextContent('view not found');
  });
});
