import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import {
  completeTask,
  createTask,
  getTask,
  listSubtasks,
  listTasks,
  localToday,
  reopenTask,
  updateTask,
} from './tasks';

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

const call = (fn: ReturnType<typeof stubFetch>) => ({
  url: fn.mock.calls[0][0],
  method: fn.mock.calls[0][1]?.method,
  body: fn.mock.calls[0][1]?.body,
});

beforeEach(() => {
  // 23:30 local on 3 Nov: the UTC date may differ, the local one must be sent.
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date(2026, 10, 3, 23, 30));
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('localToday', () => {
  it('is the local calendar date, zero-padded', () => {
    expect(localToday()).toBe('2026-11-03');
    expect(localToday(new Date(2027, 0, 5, 0, 1))).toBe('2027-01-05');
  });
});

describe('task calls', () => {
  it('listTasks sends today and the set filter fields', async () => {
    const fetch = stubFetch(() => Response.json({ tasks: [], total: 0 }));
    expect(await listTasks('today')).toEqual({ tasks: [], total: 0 });
    expect(call(fetch)).toMatchObject({ url: '/api/lists/today?today=2026-11-03', method: 'GET' });
    await listTasks('all', { project: 2, area: undefined, limit: 50, offset: 0 });
    expect(fetch.mock.calls[1][0]).toBe('/api/lists/all?today=2026-11-03&project=2&limit=50&offset=0');
  });

  it('listTasks throws the server error', async () => {
    stubFetch(() => Response.json({ error: 'unknown list' }, { status: 404 }));
    const err = await listTasks('inbox').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).message).toBe('unknown list');
  });

  it('listSubtasks unwraps the rows', async () => {
    const fetch = stubFetch(() => Response.json({ tasks: [{ id: 3 }] }));
    expect(await listSubtasks(1)).toEqual([{ id: 3 }]);
    expect(call(fetch)).toMatchObject({ url: '/api/tasks/1/subtasks', method: 'GET' });
  });

  it('getTask, createTask and updateTask hit /api/tasks', async () => {
    const fetch = stubFetch(() => Response.json({ id: 4 }));
    await getTask(4);
    await createTask({ title: 'x', task: { priority: 1 } });
    await updateTask(4, { task: { scheduled: null } });
    const calls = fetch.mock.calls.map(([url, init]) => [init?.method, url, init?.body]);
    expect(calls).toEqual([
      ['GET', '/api/tasks/4', undefined],
      ['POST', '/api/tasks', '{"title":"x","task":{"priority":1}}'],
      ['PATCH', '/api/tasks/4', '{"task":{"scheduled":null}}'],
    ]);
  });

  it('completeTask sends today and returns task and next', async () => {
    const fetch = stubFetch(() => Response.json({ task: { id: 4 }, next: { id: 5 } }));
    expect(await completeTask(4)).toEqual({ task: { id: 4 }, next: { id: 5 } });
    expect(call(fetch)).toMatchObject({ url: '/api/tasks/4/complete?today=2026-11-03', method: 'POST' });
  });

  it('reopenTask POSTs the reopen', async () => {
    const fetch = stubFetch(() => Response.json({ id: 4 }));
    await reopenTask(4);
    expect(call(fetch)).toMatchObject({ url: '/api/tasks/4/reopen', method: 'POST' });
  });
});
