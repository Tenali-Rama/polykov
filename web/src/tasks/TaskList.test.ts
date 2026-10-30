import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { stubApi, urls, type Route } from '../objects/testing';
import type { Row } from '../lib/tasks';
import TaskList, { TASKS_CHANGED } from './TaskList.svelte';
import { makeRow } from './testing';

const TODAY = '2026-11-03';

function api(tasks: Row[], more: Record<string, Route> = {}) {
  return stubApi({
    'GET /api/lists/today': () => Response.json({ tasks, total: tasks.length }),
    'GET /api/lists/upcoming': () => Response.json({ tasks, total: tasks.length }),
    'GET /api/lists/inbox': () => Response.json({ tasks, total: tasks.length }),
    'GET /api/lists/all': () => Response.json({ tasks, total: tasks.length }),
    ...more,
  });
}

const rowItems = () => screen.queryAllByRole('listitem');
const titles = () => rowItems().map((li) => within(li).getByRole('link'));

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date(2026, 10, 3, 12, 0));
  location.hash = '#/today';
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  location.hash = '';
});

describe('rendering', () => {
  it('shows a full row: check, priority, title link, dates, overdue, sub-tasks, tags', async () => {
    const fetch = api([
      makeRow(
        5,
        { priority: 1, scheduled: '2026-11-01', deadline: '2026-11-02T17:00' },
        { title: 'Pay rent', tags: ['home', 'money'], subtasks: { open: 1, total: 3 } },
      ),
      makeRow(6, { deadline: TODAY }),
    ]);
    render(TaskList, { name: 'today' });
    await waitFor(() => expect(rowItems()).toHaveLength(2));
    const [first, second] = rowItems();
    expect(first).toHaveTextContent('[ ] p1 Pay rent 2026-11-01 due 2026-11-02T17:00 [1/3] #home #money');
    expect(within(first).getByRole('link')).toHaveAttribute('href', '#/o/5');
    expect(within(first).getByText('due 2026-11-02T17:00')).toHaveClass('err');
    // P4 is hidden; due today is not overdue.
    expect(second).toHaveTextContent('[ ] task 6 due 2026-11-03');
    expect(second).not.toHaveTextContent('p4');
    expect(within(second).getByText(`due ${TODAY}`)).not.toHaveClass('err');
    expect(within(second).queryByText(/\[\d+\/\d+\]/)).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'today' })).toBeInTheDocument();
    expect(urls(fetch, '/api/lists')).toEqual([`/api/lists/today?today=${TODAY}&limit=100&offset=0`]);
  });

  it('groups upcoming under date headers (scheduled, else deadline)', async () => {
    api([
      makeRow(1, { scheduled: '2026-11-04T09:00' }),
      makeRow(2, { scheduled: '2026-11-04' }),
      makeRow(3, { deadline: '2026-11-06' }),
    ]);
    render(TaskList, { name: 'upcoming' });
    const dates = await screen.findAllByRole('heading', { level: 2 });
    expect(dates.map((h) => h.textContent)).toEqual(['2026-11-04', '2026-11-06']);
    expect(within(screen.getByRole('list', { name: '2026-11-04' })).getAllByRole('listitem')).toHaveLength(2);
    expect(within(screen.getByRole('list', { name: '2026-11-06' })).getByRole('listitem')).toHaveTextContent('task 3');
  });

  it.each(['inbox', 'today', 'upcoming'] as const)('shows nothing in %s when empty', async (name) => {
    api([]);
    render(TaskList, { name });
    expect(await screen.findByText(`nothing in ${name}`)).toBeInTheDocument();
  });

  it('passes the filter and shows no heading when embedded', async () => {
    const fetch = api([makeRow(1)]);
    render(TaskList, { name: 'all', filter: { area: 9 } });
    await waitFor(() => expect(rowItems()).toHaveLength(1));
    expect(screen.queryByRole('heading')).not.toBeInTheDocument();
    expect(urls(fetch, '/api/lists')).toEqual([`/api/lists/all?today=${TODAY}&area=9&limit=100&offset=0`]);
  });

  it('shows a load error', async () => {
    api([], { 'GET /api/lists/today': () => Response.json({ error: 'today must be a date YYYY-MM-DD' }, { status: 400 }) });
    render(TaskList, { name: 'today' });
    expect(await screen.findByRole('alert')).toHaveTextContent('error: today must be a date YYYY-MM-DD');
    expect(screen.queryByText('nothing in today')).not.toBeInTheDocument();
  });

  it('pages with [ more ]', async () => {
    let call = 0;
    const fetch = stubApi({
      'GET /api/lists/today': () =>
        Response.json(call++ === 0 ? { tasks: [makeRow(1)], total: 2 } : { tasks: [makeRow(2)], total: 2 }),
    });
    render(TaskList, { name: 'today' });
    await fireEvent.click(await screen.findByRole('button', { name: '[ more ]' }));
    await waitFor(() => expect(rowItems()).toHaveLength(2));
    expect(urls(fetch, '/api/lists')[1]).toBe(`/api/lists/today?today=${TODAY}&limit=100&offset=1`);
    expect(screen.queryByRole('button', { name: '[ more ]' })).not.toBeInTheDocument();
  });

  it(`reloads on the window event ${TASKS_CHANGED}`, async () => {
    let tasks = [makeRow(1)];
    const fetch = stubApi({ 'GET /api/lists/today': () => Response.json({ tasks, total: tasks.length }) });
    render(TaskList, { name: 'today' });
    await waitFor(() => expect(rowItems()).toHaveLength(1));
    tasks = [makeRow(2), makeRow(1)];
    window.dispatchEvent(new Event('polykov:tasks-changed'));
    await waitFor(() => expect(rowItems()).toHaveLength(2));
    expect(urls(fetch, '/api/lists')).toHaveLength(2);
  });
});

describe('complete and reopen', () => {
  const done = (id: number, over = {}) => makeRow(id, { status: 'done', completed: '2026-11-03T12:00:00Z', ...over });

  it('[ ] completes with today, [x] reopens', async () => {
    const fetch = api([makeRow(5)], {
      'POST /api/tasks/5/complete': () => Response.json({ task: done(5), next: null }),
      'POST /api/tasks/5/reopen': () => Response.json(makeRow(5)),
    });
    render(TaskList, { name: 'today' });
    await fireEvent.click(await screen.findByRole('button', { name: 'complete task 5' }));
    const reopen = await screen.findByRole('button', { name: 'reopen task 5' });
    expect(reopen).toHaveTextContent('[x]');
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(urls(fetch, '/api/tasks')).toEqual([`/api/tasks/5/complete?today=${TODAY}`]);
    await fireEvent.click(reopen);
    expect(await screen.findByRole('button', { name: 'complete task 5' })).toHaveTextContent('[ ]');
    expect(urls(fetch, '/api/tasks')[1]).toBe('/api/tasks/5/reopen');
  });

  it('a recurring task shows done · next <date>, cleared on reopen', async () => {
    const recurrence = { rule: 'FREQ=WEEKLY', from: 'scheduled' as const };
    api([makeRow(5, { scheduled: TODAY, recurrence })], {
      'POST /api/tasks/5/complete': () =>
        Response.json({ task: done(5), next: makeRow(6, { scheduled: '2026-11-10T08:00', recurrence }) }),
      'POST /api/tasks/5/reopen': () => Response.json(makeRow(5)),
    });
    render(TaskList, { name: 'today' });
    await fireEvent.click(await screen.findByRole('button', { name: 'complete task 5' }));
    expect(await screen.findByRole('status')).toHaveTextContent('done · next 2026-11-10');
    await fireEvent.click(screen.getByRole('button', { name: 'reopen task 5' }));
    await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
  });

  it('shows a failed complete inline and stays open', async () => {
    api([makeRow(5)], { 'POST /api/tasks/5/complete': () => Response.json({ error: 'task not found' }, { status: 404 }) });
    render(TaskList, { name: 'today' });
    await fireEvent.click(await screen.findByRole('button', { name: 'complete task 5' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('error: task not found');
    expect(screen.getByRole('button', { name: 'complete task 5' })).toBeInTheDocument();
  });
});

describe('keys', () => {
  async function setup() {
    const fetch = api([makeRow(1), makeRow(2), makeRow(3)], {
      'POST /api/tasks/2/complete': () => Response.json({ task: makeRow(2, { status: 'done' }), next: null }),
    });
    render(TaskList, { name: 'today' });
    await waitFor(() => expect(rowItems()).toHaveLength(3));
    return fetch;
  }

  it('j selects the first row, then j/k move and focus; clamped at both ends', async () => {
    await setup();
    expect(titles().some((a) => a.hasAttribute('aria-current'))).toBe(false);
    await fireEvent.keyDown(document.body, { key: 'j' });
    expect(titles()[0]).toHaveFocus();
    expect(titles()[0]).toHaveAttribute('aria-current', 'true');
    await fireEvent.keyDown(document.body, { key: 'j' });
    await fireEvent.keyDown(document.body, { key: 'j' });
    await fireEvent.keyDown(document.body, { key: 'j' });
    expect(titles()[2]).toHaveFocus();
    await fireEvent.keyDown(document.body, { key: 'k' });
    expect(titles()[1]).toHaveFocus();
  });

  it('x toggles the selected task; Enter opens it', async () => {
    const fetch = await setup();
    await fireEvent.keyDown(document.body, { key: 'x' });
    expect(urls(fetch, '/api/tasks')).toEqual([]);
    await fireEvent.keyDown(document.body, { key: 'j' });
    await fireEvent.keyDown(document.body, { key: 'j' });
    await fireEvent.keyDown(document.body, { key: 'x' });
    expect(await screen.findByRole('button', { name: 'reopen task 2' })).toBeInTheDocument();
    expect(urls(fetch, '/api/tasks')).toEqual([`/api/tasks/2/complete?today=${TODAY}`]);
    await fireEvent.keyDown(document.body, { key: 'Enter' });
    expect(location.hash).toBe('#/o/2');
  });

  it('ignores the keys while a control owns them', async () => {
    const fetch = await setup();
    const input = document.createElement('input');
    document.body.append(input);
    for (const key of ['j', 'x', 'Enter']) await fireEvent.keyDown(input, { key });
    await fireEvent.keyDown(screen.getByRole('button', { name: 'complete task 1' }), { key: 'j' });
    input.remove();
    expect(titles().some((a) => a.hasAttribute('aria-current'))).toBe(false);
    expect(urls(fetch, '/api/tasks')).toEqual([]);
    expect(location.hash).toBe('#/today');
  });
});
