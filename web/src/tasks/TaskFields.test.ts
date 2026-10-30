import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import Detail from '../objects/Detail.svelte';
import { makeObj, stubApi, type Route } from '../objects/testing';
import type { Task, TaskFields as Fields } from '../lib/tasks';
import TaskFields from './TaskFields.svelte';
import { makeRow } from './testing';

function makeTask(task: Partial<Fields> = {}): Task {
  // A Row is a Task plus `subtasks`; the task API serves the Task alone.
  const t: Partial<ReturnType<typeof makeRow>> = makeRow(7, task);
  delete t.subtasks;
  return t as Task;
}

/** Serves task 7, projects 2 and 3 (2 has sections 20, 21), area 5, and merges each PATCH into the task. */
function api(task: Task, more: Record<string, Route> = {}) {
  let current = task;
  const patches: unknown[] = [];
  const fetch = stubApi({
    'GET /api/tasks/7': () => Response.json(current),
    'GET /api/projects': () =>
      Response.json({ projects: [makeObj(2, { type: 'project', title: 'Launch' }), makeObj(3, { type: 'project', title: 'Move' })] }),
    'GET /api/objects': () => Response.json({ objects: [makeObj(5, { type: 'area', title: 'Health' })], total: 1 }),
    'GET /api/projects/2/sections': () =>
      Response.json({
        sections: [
          { id: 20, project_id: 2, title: 'Plan', position: 0 },
          { id: 21, project_id: 2, title: 'Build', position: 1 },
        ],
      }),
    'GET /api/projects/3/sections': () => Response.json({ sections: [] }),
    'PATCH /api/tasks/7': (_url, init) => {
      const body = JSON.parse(String(init?.body)) as { task: Partial<Fields> };
      patches.push(body);
      current = { ...current, task: { ...current.task, ...body.task } };
      return Response.json(current);
    },
    ...more,
  });
  return { fetch, patches };
}

async function open(task = makeTask(), more: Record<string, Route> = {}) {
  const res = api(task, more);
  render(TaskFields, { props: { id: 7 } });
  await screen.findByRole('combobox', { name: 'priority' });
  return res;
}

const field = (name: string) => screen.getByLabelText(name) as HTMLInputElement;

afterEach(() => vi.unstubAllGlobals());

describe('task fields', () => {
  it('priority, project, section and area each send one PATCH', async () => {
    const { patches } = await open();
    await fireEvent.change(field('priority'), { target: { value: '1' } });
    await fireEvent.change(field('project'), { target: { value: '2' } });
    const section = await screen.findByRole('combobox', { name: 'section' });
    await waitFor(() => expect(section).toHaveTextContent('Build'));
    await fireEvent.change(section, { target: { value: '21' } });
    await fireEvent.change(field('area'), { target: { value: '5' } });
    await fireEvent.change(field('area'), { target: { value: '' } });
    await waitFor(() => expect(patches).toHaveLength(5));
    expect(patches).toEqual([
      { task: { priority: 1 } },
      { task: { project_id: 2 } },
      { task: { section_id: 21 } },
      { task: { area_id: 5 } },
      { task: { area_id: null } },
    ]);
  });

  it('scheduled and deadline take a date, an optional time and a clear', async () => {
    const { patches } = await open(makeTask({ deadline: '2026-11-09T17:30' }));
    expect(screen.queryByLabelText('scheduled time')).toBeNull();
    await fireEvent.change(field('scheduled'), { target: { value: '2026-11-05' } });
    await fireEvent.change(await screen.findByLabelText('scheduled time'), { target: { value: '09:30' } });
    expect(field('deadline').value).toBe('2026-11-09');
    expect(field('deadline time').value).toBe('17:30');
    await fireEvent.change(field('deadline time'), { target: { value: '' } });
    await fireEvent.click(screen.getByRole('button', { name: 'clear scheduled' }));
    await waitFor(() => expect(patches).toHaveLength(4));
    expect(patches).toEqual([
      { task: { scheduled: '2026-11-05' } },
      { task: { scheduled: '2026-11-05T09:30' } },
      { task: { deadline: '2026-11-09' } },
      { task: { scheduled: null } },
    ]);
  });

  it('emptying a date that has a time clears both', async () => {
    const { patches } = await open(makeTask({ scheduled: '2026-11-05T09:30' }));
    await fireEvent.change(field('scheduled'), { target: { value: '' } });
    await waitFor(() => expect(screen.queryByLabelText('scheduled time')).toBeNull());
    expect(patches).toEqual([{ task: { scheduled: null } }]);
  });

  it('estimate accepts 90, 90m and 1h30m, and shows an inline error otherwise', async () => {
    const { patches } = await open();
    const input = field('estimate');
    await fireEvent.input(input, { target: { value: 'soon' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(screen.getByRole('alert')).toHaveTextContent('estimate must look like 90, 90m or 1h30m');
    expect(patches).toEqual([]);
    await fireEvent.input(input, { target: { value: '1h30m' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    // A blur right after Enter, and one after the reply, send nothing more.
    await fireEvent.blur(input);
    await waitFor(() => expect(patches).toHaveLength(1));
    await fireEvent.blur(input);
    await fireEvent.input(input, { target: { value: '45' } });
    await fireEvent.blur(input);
    await fireEvent.input(input, { target: { value: '' } });
    await fireEvent.blur(input);
    await waitFor(() => expect(patches).toHaveLength(3));
    expect(patches).toEqual([{ task: { estimate: 90 } }, { task: { estimate: 45 } }, { task: { estimate: null } }]);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('someday toggles', async () => {
    const { patches } = await open();
    const someday = screen.getByRole('button', { name: 'someday' });
    expect(someday).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(someday);
    await waitFor(() => expect(someday).toHaveAttribute('aria-pressed', 'true'));
    expect(patches).toEqual([{ task: { someday: true } }]);
  });

  it('parent: an id, a title match (tasks only), and clear', async () => {
    const { patches } = await open(makeTask(), {
      'GET /api/objects/9': () => Response.json(makeObj(9, { type: 'task', title: 'Big one' })),
      'GET /api/objects/8': () => Response.json(makeObj(8, { type: 'task', title: 'Write report' })),
    });
    // The areas route also answers findByTitle's listing, so swap in a mixed listing.
    stubAgain({
      'GET /api/objects': () =>
        Response.json({
          objects: [makeObj(8, { type: 'task', title: 'Write report' }), makeObj(4, { type: 'note', title: 'report notes' })],
          total: 2,
        }),
    });
    await fireEvent.click(screen.getByRole('button', { name: 'set parent' }));
    const prompt = screen.getByRole('textbox', { name: 'parent id or title' });
    expect(prompt).toHaveFocus();
    await fireEvent.input(prompt, { target: { value: 'report' } });
    await fireEvent.submit(prompt);
    const matches = await screen.findByRole('list', { name: 'parent matches' });
    expect(matches).toHaveTextContent('Write report');
    expect(matches).not.toHaveTextContent('report notes');
    await fireEvent.click(screen.getByRole('button', { name: 'Write report' }));
    expect(await screen.findByRole('link', { name: 'Write report' })).toHaveAttribute('href', '#/o/8');
    await fireEvent.click(screen.getByRole('button', { name: 'change parent' }));
    const again = screen.getByRole('textbox', { name: 'parent id or title' });
    await fireEvent.input(again, { target: { value: '9' } });
    await fireEvent.submit(again);
    expect(await screen.findByRole('link', { name: 'Big one' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'clear parent' }));
    await waitFor(() => expect(screen.queryByRole('link')).toBeNull());
    expect(patches).toEqual([{ task: { parent_id: 8 } }, { task: { parent_id: 9 } }, { task: { parent_id: null } }]);
  });

  it('Esc closes the parent prompt and is marked handled', async () => {
    await open();
    await fireEvent.click(screen.getByRole('button', { name: 'set parent' }));
    const ev = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    screen.getByRole('textbox', { name: 'parent id or title' }).dispatchEvent(ev);
    expect(ev.defaultPrevented).toBe(true);
    await waitFor(() => expect(screen.queryByRole('textbox', { name: 'parent id or title' })).toBeNull());
  });

  it("shows the server's 400 text inline", async () => {
    await open(makeTask(), {
      'PATCH /api/tasks/7': () => Response.json({ error: 'parent must be a live task' }, { status: 400 }),
    });
    await fireEvent.click(screen.getByRole('button', { name: 'set parent' }));
    const prompt = screen.getByRole('textbox', { name: 'parent id or title' });
    await fireEvent.input(prompt, { target: { value: '99' } });
    await fireEvent.submit(prompt);
    expect(await screen.findByRole('alert')).toHaveTextContent('parent must be a live task');
    // The prompt stays open with the draft.
    expect(screen.getByRole('textbox', { name: 'parent id or title' })).toHaveValue('99');
  });

  it('recurrence: frequency, weekdays, interval, month day and from each send the built rule', async () => {
    const { patches } = await open();
    expect(screen.getByText('does not repeat')).toBeInTheDocument();
    await fireEvent.change(field('repeat'), { target: { value: 'week' } });
    await fireEvent.click(await screen.findByRole('button', { name: 'thu' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'thu' })).toHaveAttribute('aria-pressed', 'true'));
    await fireEvent.click(screen.getByRole('button', { name: 'mon' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'mon' })).toHaveAttribute('aria-pressed', 'true'));
    await fireEvent.change(field('interval'), { target: { value: '2' } });
    await screen.findByText('every 2 weeks on mon, thu · from scheduled');
    await fireEvent.change(field('repeat from'), { target: { value: 'completion' } });
    expect(await screen.findByText('every 2 weeks on mon, thu · from completion')).toBeInTheDocument();
    await fireEvent.change(field('repeat'), { target: { value: 'month' } });
    await fireEvent.change(await screen.findByLabelText('month day'), { target: { value: '15' } });
    await screen.findByText('every 2 months on day 15 · from completion');
    await fireEvent.change(field('repeat'), { target: { value: '' } });
    await waitFor(() => expect(patches).toHaveLength(8));
    const rec = (rule: string, from = 'scheduled') => ({ task: { recurrence: { rule, from } } });
    expect(patches).toEqual([
      rec('FREQ=WEEKLY'),
      rec('FREQ=WEEKLY;BYDAY=TH'),
      rec('FREQ=WEEKLY;BYDAY=MO,TH'),
      rec('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH'),
      rec('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH', 'completion'),
      rec('FREQ=MONTHLY;INTERVAL=2', 'completion'),
      rec('FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=15', 'completion'),
      { task: { recurrence: null } },
    ]);
  });

  it('complete shows the next occurrence of a recurring task, reopen undoes it', async () => {
    const recurring = makeTask({ scheduled: '2026-11-03', recurrence: { rule: 'FREQ=WEEKLY', from: 'scheduled' } });
    const done = makeTask({ status: 'done' });
    const next = { ...makeTask({ scheduled: '2026-11-10' }), id: 12 };
    const { fetch } = await open(recurring, {
      'POST /api/tasks/7/complete': () => Response.json({ task: done, next }),
      'POST /api/tasks/7/reopen': () => Response.json(makeTask()),
    });
    expect(screen.getByText('every week · from scheduled')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: '[ complete ]' }));
    expect(await screen.findByRole('link', { name: 'next 2026-11-10' })).toHaveAttribute('href', '#/o/12');
    await fireEvent.click(screen.getByRole('button', { name: '[ reopen ]' }));
    await screen.findByRole('button', { name: '[ complete ]' });
    expect(screen.queryByRole('link', { name: /next/ })).toBeNull();
    expect(fetch.mock.calls.map(([u, i]) => `${i?.method ?? 'GET'} ${u.split('?')[0]}`)).toContain('POST /api/tasks/7/reopen');
  });
});

describe('detail panels', () => {
  function detail(obj = makeObj(7)) {
    stubApi({
      'GET /api/objects/7': () => Response.json(obj),
      'GET /api/objects/7/attachments': () => Response.json({ attachments: [] }),
      'GET /api/objects/7/links': () => Response.json({ links: [], backlinks: [] }),
      'GET /api/objects/7/comments': () => Response.json({ comments: [] }),
      'GET /api/tasks/7': () => Response.json(makeTask()),
      'GET /api/tasks/7/subtasks': () => Response.json({ tasks: [] }),
      'GET /api/projects': () => Response.json({ projects: [] }),
      'GET /api/objects': () => Response.json({ objects: [], total: 0 }),
    });
    render(Detail, { props: { id: 7, ondeleted: () => {} } });
  }

  it('a non-task object shows no task panels', async () => {
    detail();
    expect(await screen.findByText('no attachments · drop files here')).toBeInTheDocument();
    for (const name of ['task', 'checklist', 'sub-tasks']) expect(screen.queryByRole('region', { name })).toBeNull();
  });

  it('a task shows its fields, checklist and sub-tasks next to the M1 panels', async () => {
    detail(makeObj(7, { type: 'task' }));
    expect(await screen.findByRole('combobox', { name: 'priority' })).toBeInTheDocument();
    for (const name of ['task', 'checklist', 'sub-tasks', 'links', 'comments']) {
      expect(screen.getByRole('region', { name })).toBeInTheDocument();
    }
    expect(await screen.findByText('no sub-tasks')).toBeInTheDocument();
  });
});

/** Replaces routes of the current `fetch` stub, keeping the rest. */
function stubAgain(routes: Record<string, Route>) {
  const old = fetch as unknown as (url: string, init?: RequestInit) => Promise<Response>;
  vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
    const route = routes[`${init?.method ?? 'GET'} ${url.split('?')[0]}`];
    return route ? route(url, init) : old(url, init);
  });
}
