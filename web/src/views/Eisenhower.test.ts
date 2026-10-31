import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { makeObj, stubApi, urls, type Route } from '../objects/testing';
import { makeRow } from '../tasks/testing';
import type { Group, View } from '../lib/views';
import ViewPage from './ViewPage.svelte';

const view: View = {
  ...makeObj(4, { type: 'view', title: 'Matrix' }),
  properties: { query: 'type:task', layout: 'eisenhower', group: 'none', sort: 'smart' },
};
// Items are the task JSON: no sub-task counts.
const task = (id: number, priority: number) => ({
  ...makeObj(id, { type: 'task', title: `task ${id}` }),
  task: makeRow(id, { priority }).task,
});
const QUADRANTS: Group[] = [
  { key: 'urgent_important', label: 'urgent & important', items: [task(1, 1), task(2, 2)] },
  { key: 'important', label: 'important', items: [task(3, 1)] },
  { key: 'urgent', label: 'urgent', items: [] },
  { key: 'neither', label: 'neither', items: [task(4, 4), makeObj(5, { title: 'Idea' })] },
];

function api(more: Record<string, Route> = {}) {
  return stubApi({
    'GET /api/objects/4': () => Response.json(view),
    'GET /api/views/4/items': () => Response.json({ view, groups: QUADRANTS, total: 5, truncated: false }),
    ...more,
  });
}

async function page() {
  render(ViewPage, { id: 4 });
  await screen.findByRole('region', { name: 'urgent & important' });
}

const quadrant = (name: string) => within(screen.getByRole('region', { name }));

afterEach(() => {
  vi.unstubAllGlobals();
  location.hash = '';
});

describe('Eisenhower', () => {
  it('renders the four quadrants in the server order, each a region with its label and count', async () => {
    api();
    await page();
    const results = within(screen.getByRole('region', { name: 'results' }));
    expect(results.getAllByRole('region').map((r) => r.getAttribute('aria-label'))).toEqual([
      'urgent & important',
      'important',
      'urgent',
      'neither',
    ]);
    expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toEqual([
      'urgent & important 2',
      'important 1',
      'urgent 0',
      'neither 2',
    ]);
    expect(quadrant('urgent & important').getAllByRole('link').map((a) => a.textContent?.trim())).toEqual([
      'p1 task 1',
      'p2 task 2',
    ]);
    expect(quadrant('important').getByRole('link', { name: 'p1 task 3' })).toHaveAttribute('href', '#/o/3');
    expect(quadrant('neither').getByRole('button', { name: 'complete task 4' })).toBeInTheDocument();
    expect(quadrant('neither').getByRole('link', { name: 'Idea' })).toHaveAttribute('href', '#/o/5');
  });

  it('an empty quadrant still renders and says empty', async () => {
    api();
    await page();
    const urgent = quadrant('urgent');
    expect(urgent.getByText('empty')).toBeInTheDocument();
    expect(urgent.queryByRole('listitem')).toBeNull();
    expect(quadrant('important').queryByText('empty')).toBeNull();
  });

  it('completing a task posts it and reloads the quadrants', async () => {
    const fetch = api({ 'POST /api/tasks/3/complete': () => Response.json({ task: task(3, 1), next: null }) });
    await page();
    await fireEvent.click(quadrant('important').getByRole('button', { name: 'complete task 3' }));
    await waitFor(() => expect(urls(fetch, '/api/tasks/3/complete')).toHaveLength(1));
    expect(await quadrant('important').findByRole('button', { name: 'reopen task 3' })).toBeInTheDocument();
  });

  it('j walks quadrant by quadrant; x completes the selection, then reloads', async () => {
    const fetch = api({ 'POST /api/tasks/3/complete': () => Response.json({ task: task(3, 1), next: null }) });
    await page();
    for (let n = 0; n < 3; n++) await fireEvent.keyDown(window, { key: 'j' });
    expect(screen.getByRole('link', { name: 'p1 task 3' })).toHaveFocus();
    await fireEvent.keyDown(window, { key: 'j' });
    expect(screen.getByRole('link', { name: 'task 4' })).toHaveFocus();
    await fireEvent.keyDown(window, { key: 'k' });
    await fireEvent.keyDown(window, { key: 'x' });
    await waitFor(() => expect(urls(fetch, '/api/tasks/3/complete')).toHaveLength(1));
    await waitFor(() => expect(urls(fetch, '/api/views/4/items')).toHaveLength(2));
  });

  it('Enter opens the selected item', async () => {
    api();
    await page();
    for (let n = 0; n < 5; n++) await fireEvent.keyDown(window, { key: 'j' });
    expect(screen.getByRole('link', { name: 'Idea' })).toHaveFocus();
    await fireEvent.keyDown(document.body, { key: 'Enter' });
    expect(location.hash).toBe('#/o/5');
  });

  it('shows a load failure', async () => {
    stubApi({
      'GET /api/objects/4': () => Response.json(view),
      'GET /api/views/4/items': () => Response.json({ error: 'boom' }, { status: 500 }),
    });
    render(ViewPage, { id: 4 });
    expect(await screen.findByRole('alert')).toHaveTextContent('error: boom');
  });
});
