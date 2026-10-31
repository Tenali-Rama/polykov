import { afterEach, describe, expect, it, vi } from 'vitest';
import { makeObj, stubApi, urls } from '../objects/testing';
import { makeRow } from '../tasks/testing';
import { TASKS_CHANGED } from '../tasks/TaskList.svelte';
import type { Task } from '../lib/tasks';
import type { Group, Item, Layout, View } from '../lib/views';
import { indexed, loader, rowKey, taskRow, viewSource, type Loaded, type RowRefs, type Source } from './source';

const view = (layout: Layout, group: View['properties']['group']): View => ({
  ...makeObj(4, { type: 'view' }),
  properties: { query: 'type:task', layout, group, sort: 'smart' },
});
const task = (id: number): Item => {
  const { title, task } = makeRow(id);
  return { ...makeObj(id, { type: 'task', title }), task };
};
const GROUPS: Group[] = [
  { key: 'a', label: 'a', items: [task(1), makeObj(2)] },
  { key: 'b', label: 'b', items: [] },
  { key: 'c', label: 'c', items: [task(3)] },
];

/** A source whose loads resolve or reject when the test says so, in any order. */
function manual() {
  const calls: { resolve: (d: Loaded) => void; reject: (e: Error) => void }[] = [];
  const source: Source = {
    group: 'none',
    load: () => new Promise((resolve, reject) => calls.push({ resolve, reject })),
  };
  return { source, calls };
}

const loaded = (total: number): Loaded => ({
  groups: [],
  total,
  truncated: false,
});
const flush = () => new Promise((r) => setTimeout(r));

afterEach(() => {
  vi.unstubAllGlobals();
  location.hash = '';
});

describe('viewSource', () => {
  it('loads the view items, without the view', async () => {
    const fetch = stubApi({
      'GET /api/views/4/items': () =>
        Response.json({
          view: view('list', 'none'),
          groups: GROUPS,
          total: 9,
          truncated: true,
        }),
    });
    expect(await viewSource(view('list', 'none')).load()).toEqual({
      groups: GROUPS,
      total: 9,
      truncated: true,
    });
    expect(urls(fetch, '/api/views/4/items')).toHaveLength(1);
  });

  it('takes the view grouping; a kanban view without one is grouped by status, as the server does', () => {
    expect(viewSource(view('list', 'none')).group).toBe('none');
    expect(viewSource(view('list', 'area')).group).toBe('area');
    expect(viewSource(view('kanban', 'none')).group).toBe('status');
    expect(viewSource(view('kanban', 'priority')).group).toBe('priority');
  });
});

describe('loader', () => {
  it('applies only the latest response, so a stale one never overwrites it', async () => {
    const { source, calls } = manual();
    const data = vi.fn();
    const { load } = loader(source, data, () => {});
    const first = load();
    const second = load();
    calls[1].resolve(loaded(2));
    calls[0].resolve(loaded(1));
    await Promise.all([first, second]);
    expect(data.mock.calls).toEqual([[loaded(2)]]);
  });

  it('clears the error on each load and reports a failure as `error: <cause>`, unless stale', async () => {
    const { source, calls } = manual();
    const error = vi.fn();
    const { load } = loader(source, () => {}, error);
    const first = load();
    calls[0].reject(new Error('boom'));
    await first;
    expect(error.mock.calls).toEqual([[''], ['error: boom']]);
    const stale = load();
    const latest = load();
    calls[1].reject(new Error('stale'));
    calls[2].resolve(loaded(0));
    await Promise.all([stale, latest]);
    expect(error.mock.calls.slice(2)).toEqual([[''], ['']]);
  });

  it('watch loads now and on every TASKS_CHANGED until its cleanup runs', async () => {
    const load = vi.fn(async () => loaded(0));
    const stop = loader(
      { group: 'none', load },
      () => {},
      () => {},
    ).watch();
    expect(load).toHaveBeenCalledTimes(1);
    window.dispatchEvent(new Event(TASKS_CHANGED));
    expect(load).toHaveBeenCalledTimes(2);
    stop();
    window.dispatchEvent(new Event(TASKS_CHANGED));
    expect(load).toHaveBeenCalledTimes(2);
  });
});

describe('indexed and taskRow', () => {
  it('numbers the items across groups in order, empty groups included', () => {
    expect(indexed(GROUPS).map((g) => g.items.map((x) => [x.item.id, x.i]))).toEqual([
      [
        [1, 0],
        [2, 1],
      ],
      [],
      [[3, 2]],
    ]);
  });

  it('gives a task item empty sub-task counts', () => {
    const item = task(1) as Task;
    expect(taskRow(item)).toEqual({ ...item, subtasks: { open: 0, total: 0 } });
  });
});

describe('rowKey', () => {
  const items = indexed(GROUPS).flatMap((g) => g.items.map((x) => x.item));
  function refs(): RowRefs & { toggled: number[]; focused: string[] } {
    const toggled: number[] = [];
    const focused: string[] = [];
    const row = (i: number) => ({
      focus: () => focused.push(`row ${i}`),
      toggle: async () => void toggled.push(i),
    });
    const link = document.createElement('a');
    link.focus = () => focused.push('link 1');
    return {
      rows: [row(0), undefined, row(2)],
      links: [undefined, link, undefined],
      toggled,
      focused,
    };
  }
  const key = (key: string, init: KeyboardEventInit = {}, target: EventTarget = document.body) => {
    const e = new KeyboardEvent('keydown', { key, cancelable: true, ...init });
    Object.defineProperty(e, 'target', { value: target });
    return e;
  };

  it('j/k select and focus the next/previous item, clamped at the ends', () => {
    const r = refs();
    const e = key('j');
    expect(rowKey(e, -1, items, r)).toBe(0);
    expect(e.defaultPrevented).toBe(true);
    expect(rowKey(key('j'), 0, items, r)).toBe(1);
    expect(rowKey(key('j'), 2, items, r)).toBe(2);
    expect(rowKey(key('k'), 0, items, r)).toBe(0);
    expect(r.focused).toEqual(['row 0', 'link 1', 'row 2', 'row 0']);
  });

  it('x toggles a selected task, then dispatches TASKS_CHANGED; not on an object', async () => {
    const r = refs();
    const changed = vi.fn();
    window.addEventListener(TASKS_CHANGED, changed);
    expect(rowKey(key('x'), 1, items, r)).toBeNull();
    expect(rowKey(key('x'), 2, items, r)).toBe(2);
    await flush();
    window.removeEventListener(TASKS_CHANGED, changed);
    expect(r.toggled).toEqual([2]);
    expect(changed).toHaveBeenCalledTimes(1);
  });

  it('Enter opens the selected item, unless nothing is selected or a link has focus', () => {
    expect(rowKey(key('Enter'), -1, items, refs())).toBeNull();
    expect(rowKey(key('Enter', {}, document.createElement('a')), 1, items, refs())).toBeNull();
    expect(location.hash).toBe('');
    expect(rowKey(key('Enter'), 1, items, refs())).toBe(1);
    expect(location.hash).toBe('#/o/2');
  });

  it('leaves alone modified keys, keys a control owns, other keys and an empty list', () => {
    const owner = document.createElement('div');
    owner.dataset.ownsKeys = '';
    const inner = owner.appendChild(document.createElement('span'));
    const r = refs();
    for (const e of [
      key('j', { ctrlKey: true }),
      key('j', {}, document.createElement('input')),
      key('j', {}, inner),
      key('q'),
    ]) {
      expect(rowKey(e, 0, items, r)).toBeNull();
      expect(e.defaultPrevented).toBe(false);
    }
    expect(rowKey(key('j'), -1, [], r)).toBeNull();
    expect(r.focused).toEqual([]);
  });
});
