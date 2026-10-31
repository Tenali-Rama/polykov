import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { makeObj, stubApi, type Route } from '../objects/testing';
import { makeRow } from '../tasks/testing';
import type { Group, View } from '../lib/views';
import Kanban from './Kanban.svelte';
import ViewPage from './ViewPage.svelte';
import { viewSource } from './source';

const view = (group: View['properties']['group']): View => ({
  ...makeObj(4, { type: 'view', title: 'Board' }),
  properties: { query: 'type:task', layout: 'kanban', group, sort: 'smart' },
});
// Items are the task JSON: no sub-task counts.
const task = (id: number, fields: Parameters<typeof makeRow>[1] = {}) => {
  const { title, task } = makeRow(id, fields);
  return { ...makeObj(id, { type: 'task', title }), task };
};
const PRIORITY: Group[] = [
  { key: 1, label: 'P1', items: [task(1, { priority: 1, scheduled: '2026-11-02', deadline: '2026-11-05' })] },
  { key: 3, label: 'P3', items: [task(2, { priority: 3 }), task(3, { priority: 3 })] },
  { key: null, label: 'none', items: [makeObj(5, { title: 'Idea' })] },
];
const P4 = 'P4 · none';
const body = (init?: RequestInit) => JSON.parse(String(init?.body));

function api(group: View['properties']['group'], groups: Group[], more: Record<string, Route> = {}) {
  return stubApi({
    'GET /api/views/4/items': () =>
      Response.json({ view: view(group), groups, total: groups.flatMap((g) => g.items).length, truncated: false }),
    ...more,
  });
}

const loadsOf = (fetch: ReturnType<typeof stubApi>) =>
  fetch.mock.calls.filter(([url]) => url.startsWith('/api/views/4/items')).length;
const column = (name: string) => within(screen.getByRole('list', { name }));
const card = (name: RegExp) => screen.getByRole('link', { name });

afterEach(() => vi.unstubAllGlobals());

describe('Kanban', () => {
  it('shows a box per column in order, with label, count and cards', async () => {
    api('priority', PRIORITY);
    render(Kanban, { source: viewSource(view('priority')) });
    await screen.findByRole('list', { name: 'P1' });
    expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toEqual([
      'P1 1',
      'P2 0',
      'P3 2',
      'P4 · none 1',
    ]);
    const first = column('P1').getByRole('link');
    expect(first).toHaveTextContent('p1 task 1 2026-11-02 due 2026-11-05');
    expect(first).toHaveAttribute('href', '#/o/1');
    expect(first).toHaveAttribute('draggable', 'true');
    expect(column(P4).getByRole('link', { name: /Idea/ })).toHaveAttribute('draggable', 'false');
    expect(screen.queryByText('read-only grouping')).toBeNull();
  });

  it('ViewPage shows a kanban view as the board', async () => {
    api('priority', PRIORITY, { 'GET /api/objects/4': () => Response.json(view('priority')) });
    render(ViewPage, { id: 4 });
    expect(await screen.findByRole('region', { name: 'P2' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /complete/ })).toBeNull();
  });

  it('arrows move focus between cards and columns, skipping empty ones', async () => {
    api('priority', PRIORITY);
    render(Kanban, { source: viewSource(view('priority')) });
    const one = await screen.findByRole('link', { name: /task 1/ });
    one.focus();
    await fireEvent.keyDown(one, { key: 'ArrowRight' });
    expect(card(/task 2/)).toHaveFocus();
    await fireEvent.keyDown(card(/task 2/), { key: 'ArrowDown' });
    expect(card(/task 3/)).toHaveFocus();
    await fireEvent.keyDown(card(/task 3/), { key: 'ArrowRight' });
    expect(card(/Idea/)).toHaveFocus();
    await fireEvent.keyDown(card(/Idea/), { key: 'ArrowLeft' });
    expect(card(/task 2/)).toHaveFocus();
    await fireEvent.keyDown(card(/task 2/), { key: 'ArrowUp' });
    expect(card(/task 2/)).toHaveFocus();
  });

  it('Shift+ArrowLeft moves the card one column with one PATCH and keeps focus on it', async () => {
    const fetch = api('priority', PRIORITY, {
      'PATCH /api/tasks/2': () => Response.json(task(2, { priority: 2 })),
    });
    render(Kanban, { source: viewSource(view('priority')) });
    const two = await screen.findByRole('link', { name: /task 2/ });
    two.focus();
    await fireEvent.keyDown(two, { key: 'ArrowLeft', shiftKey: true });
    await waitFor(() => expect(column('P2').getByRole('link', { name: 'p2 task 2' })).toHaveFocus());
    const patches = fetch.mock.calls.filter(([, init]) => init?.method === 'PATCH');
    expect(patches.map(([url, init]) => [url, body(init)])).toEqual([['/api/tasks/2', { task: { priority: 2 } }]]);
    expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toContain('P3 1');
  });

  it('a keyboard move to done completes the task', async () => {
    const fetch = api('none', [{ key: 'open', label: 'open', items: [task(1)] }], {
      'POST /api/tasks/1/complete': () => Response.json({ task: task(1, { status: 'done' }), next: null }),
    });
    render(Kanban, { source: viewSource(view('none')) });
    const one = await screen.findByRole('link', { name: /task 1/ });
    one.focus();
    await fireEvent.keyDown(one, { key: 'ArrowRight', shiftKey: true });
    await waitFor(() => expect(column('done').getByRole('link', { name: /task 1/ })).toHaveFocus());
    expect(fetch.mock.calls.some(([url, init]) => url.startsWith('/api/tasks/1/complete') && init?.method === 'POST')).toBe(
      true,
    );
  });

  it('completing a recurring task reloads the board, so its next occurrence shows in open', async () => {
    let loads = 0;
    const fetch = stubApi({
      'GET /api/views/4/items': () => {
        loads++;
        const groups: Group[] =
          loads === 1
            ? [{ key: 'open', label: 'open', items: [task(1)] }]
            : [
                { key: 'open', label: 'open', items: [task(8)] },
                { key: 'done', label: 'done', items: [task(1, { status: 'done' })] },
              ];
        return Response.json({ view: view('status'), groups, total: groups.length, truncated: false });
      },
      'POST /api/tasks/1/complete': () => Response.json({ task: task(1, { status: 'done' }), next: task(8) }),
    });
    render(Kanban, { source: viewSource(view('status')) });
    const one = await screen.findByRole('link', { name: /task 1/ });
    one.focus();
    await fireEvent.keyDown(one, { key: 'ArrowRight', shiftKey: true });
    expect(await column('open').findByRole('link', { name: /task 8/ })).toHaveAttribute('href', '#/o/8');
    await waitFor(() => expect(column('done').getByRole('link', { name: /task 1/ })).toHaveFocus());
    expect(loadsOf(fetch)).toBe(2);
  });

  it('a move without a next occurrence does not reload', async () => {
    const fetch = api('none', [{ key: 'open', label: 'open', items: [task(1)] }], {
      'POST /api/tasks/1/complete': () => Response.json({ task: task(1, { status: 'done' }), next: null }),
    });
    render(Kanban, { source: viewSource(view('none')) });
    const one = await screen.findByRole('link', { name: /task 1/ });
    one.focus();
    await fireEvent.keyDown(one, { key: 'ArrowRight', shiftKey: true });
    await waitFor(() => expect(column('done').getByRole('link', { name: /task 1/ })).toHaveFocus());
    expect(loadsOf(fetch)).toBe(1);
  });

  it('on a tag board, arrows follow the column of the focused card, not its task id', async () => {
    api('tag', [
      { key: 'home', label: 'home', items: [task(1), task(2)] },
      { key: 'work', label: 'work', items: [task(3), task(1), task(4)] },
    ]);
    render(Kanban, { source: viewSource(view('tag')) });
    await screen.findByRole('list', { name: 'work' });
    const inWork = column('work').getByRole('link', { name: /task 1/ });
    inWork.focus();
    await fireEvent.keyDown(inWork, { key: 'ArrowDown' });
    expect(column('work').getByRole('link', { name: /task 4/ })).toHaveFocus();
    await fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    expect(inWork).toHaveFocus();
    await fireEvent.keyDown(inWork, { key: 'ArrowLeft' });
    expect(column('home').getByRole('link', { name: /task 2/ })).toHaveFocus();
  });

  it('a failed drop shows the server error and puts the card back', async () => {
    api('priority', PRIORITY, {
      'PATCH /api/tasks/1': () => Response.json({ error: 'priority must be 1 to 4' }, { status: 400 }),
    });
    render(Kanban, { source: viewSource(view('priority')) });
    const one = await screen.findByRole('link', { name: /task 1/ });
    await fireEvent.dragStart(one);
    await fireEvent.dragOver(screen.getByRole('region', { name: P4 }));
    await fireEvent.drop(screen.getByRole('region', { name: P4 }));
    expect(await screen.findByRole('alert')).toHaveTextContent('error: priority must be 1 to 4');
    expect(column('P1').getByRole('link', { name: /task 1/ })).toBeInTheDocument();
    expect(column(P4).queryAllByRole('link', { name: /task 1/ })).toHaveLength(0);
  });

  it('a tag board is read-only: it says so, nothing drags and a move sends nothing', async () => {
    const fetch = api('tag', [
      { key: 'home', label: 'home', items: [task(1)] },
      { key: 'work', label: 'work', items: [task(2)] },
    ]);
    render(Kanban, { source: viewSource(view('tag')) });
    expect(await screen.findByText('read-only grouping')).toBeInTheDocument();
    const one = card(/task 1/);
    expect(one).toHaveAttribute('draggable', 'false');
    one.focus();
    await fireEvent.keyDown(one, { key: 'ArrowRight', shiftKey: true });
    expect(column('home').getByRole('link', { name: /task 1/ })).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledOnce();
  });
});
