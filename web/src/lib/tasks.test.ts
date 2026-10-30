import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import {
  buildRule,
  completeTask,
  createTask,
  describeRecurrence,
  formatEstimate,
  getTask,
  joinDate,
  listSubtasks,
  listTasks,
  localToday,
  parseEstimate,
  parseRule,
  reopenTask,
  splitDate,
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

describe('estimate', () => {
  it('parses minutes, m and h forms', () => {
    expect(parseEstimate('90')).toBe(90);
    expect(parseEstimate('90m')).toBe(90);
    expect(parseEstimate('1h30m')).toBe(90);
    expect(parseEstimate(' 1H 30M ')).toBe(90);
    expect(parseEstimate('2h')).toBe(120);
  });

  it('rejects anything else', () => {
    for (const bad of ['', 'h', 'm', '1.5h', '30m1h', 'ninety', '-5', '1d']) expect(parseEstimate(bad)).toBeNull();
  });

  it('formats minutes back', () => {
    expect([45, 60, 90, 600].map(formatEstimate)).toEqual(['45m', '1h', '1h30m', '10h']);
  });
});

describe('task dates', () => {
  it('splits and joins a date and a time', () => {
    expect(splitDate('2026-11-03T17:30')).toEqual({ date: '2026-11-03', time: '17:30' });
    expect(splitDate('2026-11-03')).toEqual({ date: '2026-11-03', time: '' });
    expect(splitDate(null)).toEqual({ date: '', time: '' });
    expect(joinDate('2026-11-03', '17:30')).toBe('2026-11-03T17:30');
    expect(joinDate('2026-11-03', '')).toBe('2026-11-03');
    expect(joinDate('', '17:30')).toBeNull();
  });
});

describe('recurrence rules', () => {
  it('builds the canonical rule', () => {
    expect(buildRule({ freq: 'day', interval: 1, byDay: [], monthDay: null })).toBe('FREQ=DAILY');
    expect(buildRule({ freq: 'week', interval: 2, byDay: ['thu', 'mon'], monthDay: null })).toBe(
      'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH',
    );
    expect(buildRule({ freq: 'month', interval: 1, byDay: [], monthDay: 15 })).toBe('FREQ=MONTHLY;BYMONTHDAY=15');
  });

  it('drops parts the frequency does not take', () => {
    expect(buildRule({ freq: 'year', interval: 1, byDay: ['mon'], monthDay: 3 })).toBe('FREQ=YEARLY');
    expect(buildRule({ freq: 'month', interval: 3, byDay: ['mon'], monthDay: null })).toBe('FREQ=MONTHLY;INTERVAL=3');
  });

  it('parses a rule in any order and case', () => {
    expect(parseRule('byday=th,mo;FREQ=WEEKLY;INTERVAL=2')).toEqual({
      freq: 'week',
      interval: 2,
      byDay: ['mon', 'thu'],
      monthDay: null,
    });
    expect(parseRule('FREQ=MONTHLY;BYMONTHDAY=31')).toMatchObject({ freq: 'month', monthDay: 31 });
  });

  it('describes a rule', () => {
    expect(describeRecurrence({ rule: 'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH', from: 'completion' })).toBe(
      'every 2 weeks on mon, thu · from completion',
    );
    expect(describeRecurrence({ rule: 'FREQ=DAILY', from: 'scheduled' })).toBe('every day · from scheduled');
    expect(describeRecurrence({ rule: 'FREQ=MONTHLY;BYMONTHDAY=15', from: 'scheduled' })).toBe(
      'every month on day 15 · from scheduled',
    );
  });
});
