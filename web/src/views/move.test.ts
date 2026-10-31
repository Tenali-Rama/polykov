import { afterEach, describe, expect, it, vi } from 'vitest';
import { makeObj, stubApi } from '../objects/testing';
import { makeRow } from '../tasks/testing';
import type { Group, View } from '../lib/views';
import { boardGroup, columns, movable, moveTask, planMove } from './move';

const view = (group: View['properties']['group']): View => ({
  ...makeObj(4, { type: 'view' }),
  properties: { query: 'type:task', layout: 'kanban', group, sort: 'smart' },
});
const done = makeRow(7, { status: 'done' });
const body = (init?: RequestInit) => JSON.parse(String(init?.body));

afterEach(() => vi.unstubAllGlobals());

describe('planMove', () => {
  it('status: done completes, open reopens, none has no call', () => {
    expect(planMove('status', 'done')).toEqual({ call: 'complete' });
    expect(planMove('status', 'open')).toEqual({ call: 'reopen' });
    expect(planMove('status', null)).toBeNull();
  });

  it('priority: sets it, and none sets the no-priority 4', () => {
    expect(planMove('priority', 2)).toEqual({ call: 'patch', task: { priority: 2 } });
    expect(planMove('priority', null)).toEqual({ call: 'patch', task: { priority: 4 } });
  });

  it('project, section and area set that id, and none clears it', () => {
    expect(planMove('project', 3)).toEqual({ call: 'patch', task: { project_id: 3 } });
    expect(planMove('project', null)).toEqual({ call: 'patch', task: { project_id: null } });
    expect(planMove('section', 8)).toEqual({ call: 'patch', task: { section_id: 8 } });
    expect(planMove('section', null)).toEqual({ call: 'patch', task: { section_id: null } });
    expect(planMove('area', 5)).toEqual({ call: 'patch', task: { area_id: 5 } });
    expect(planMove('area', null)).toEqual({ call: 'patch', task: { area_id: null } });
  });

  it('tag and type are read-only', () => {
    expect(planMove('tag', 'work')).toBeNull();
    expect(planMove('type', 'task')).toBeNull();
    expect(movable('tag')).toBe(false);
    expect(movable('type')).toBe(false);
    expect(movable('priority')).toBe(true);
  });
});

describe('moveTask', () => {
  it('a move to done completes the task through POST', async () => {
    const fetch = stubApi({ 'POST /api/tasks/7/complete': () => Response.json({ task: done, next: null }) });
    expect(await moveTask('status', 7, 'done')).toEqual(done);
    expect(fetch.mock.calls[0][0]).toMatch(/^\/api\/tasks\/7\/complete\?today=\d{4}-\d{2}-\d{2}$/);
  });

  it('a move to open reopens it', async () => {
    const fetch = stubApi({ 'POST /api/tasks/7/reopen': () => Response.json(makeRow(7)) });
    await moveTask('status', 7, 'open');
    expect(fetch).toHaveBeenCalledOnce();
  });

  it('a field move sends one PATCH with the task change', async () => {
    const fetch = stubApi({ 'PATCH /api/tasks/7': () => Response.json(makeRow(7, { project_id: null })) });
    await moveTask('project', 7, null);
    expect(body(fetch.mock.calls[0][1])).toEqual({ task: { project_id: null } });
  });

  it('a read-only grouping throws without a request', async () => {
    const fetch = stubApi({});
    await expect(moveTask('tag', 7, 'work')).rejects.toThrow('read-only grouping');
    await expect(moveTask('status', 7, null)).rejects.toThrow('cannot move to none');
    expect(fetch).not.toHaveBeenCalled();
  });
});

describe('columns', () => {
  const note = makeObj(9);
  const groups: Group[] = [
    { key: 2, label: 'P2', items: [makeRow(1, { priority: 2 })] },
    { key: null, label: 'none', items: [note] },
  ];

  it('adds the empty fixed columns in order and keeps none last', () => {
    expect(columns('priority', groups).map((g) => [g.label, g.items.length])).toEqual([
      ['P1', 0],
      ['P2', 1],
      ['P3', 0],
      ['P4', 0],
      ['none', 1],
    ]);
    expect(columns('status', []).map((g) => g.key)).toEqual(['open', 'done']);
  });

  it('keeps the server groups as they are for other groupings', () => {
    expect(columns('project', groups)).toBe(groups);
  });

  it('a kanban view without grouping is grouped by status', () => {
    expect(boardGroup(view('none'))).toBe('status');
    expect(boardGroup(view('area'))).toBe('area');
  });
});
