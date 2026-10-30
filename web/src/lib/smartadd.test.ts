import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import { add, landing, localToday, parse, type AddedTask } from './smartadd';

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

const body = (fn: ReturnType<typeof stubFetch>) => JSON.parse(String(fn.mock.calls[0][1]?.body));

const fields: AddedTask['task'] = {
  scheduled: null,
  deadline: null,
  project_id: null,
  area_id: null,
  parent_id: null,
  someday: false,
};

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe('smart add calls', () => {
  it('localToday is the local calendar date, zero-padded', () => {
    expect(localToday(new Date(2026, 0, 5, 23, 59))).toBe('2026-01-05');
  });

  it('parse POSTs the text with the local today and passes the signal', async () => {
    vi.useFakeTimers({ now: new Date(2026, 9, 30, 23, 30), toFake: ['Date'] });
    const fetch = stubFetch(() => Response.json({ title: 'x', tokens: [], tags: [], task: {} }));
    const ctrl = new AbortController();
    expect((await parse('x p2', ctrl.signal)).title).toBe('x');
    expect(fetch.mock.calls[0][0]).toBe('/api/parse');
    expect(fetch.mock.calls[0][1]?.method).toBe('POST');
    expect(fetch.mock.calls[0][1]?.signal).toBe(ctrl.signal);
    expect(body(fetch)).toEqual({ text: 'x p2', today: '2026-10-30' });
  });

  it('add POSTs to /api/tasks/add and returns the task', async () => {
    const fetch = stubFetch(() => Response.json({ id: 3, title: 'x', task: fields }, { status: 201 }));
    expect((await add('x')).id).toBe(3);
    expect(fetch.mock.calls[0][0]).toBe('/api/tasks/add');
    expect(body(fetch)).toEqual({ text: 'x', today: localToday() });
  });

  it('add throws the server text on 400', async () => {
    stubFetch(() => Response.json({ error: 'unknown project: nope' }, { status: 400 }));
    await expect(add('x +nope')).rejects.toEqual(new ApiError(400, 'unknown project: nope'));
  });
});

describe('landing', () => {
  const today = '2026-10-30';
  it('names the list a new task lands in', () => {
    expect(landing(fields, today)).toBe('inbox');
    expect(landing({ ...fields, scheduled: '2026-10-31T17:00' }, today)).toBe('upcoming');
    expect(landing({ ...fields, scheduled: '2026-10-30' }, today)).toBe('today');
    expect(landing({ ...fields, deadline: '2026-11-02' }, today)).toBe('upcoming');
    expect(landing({ ...fields, scheduled: '2026-11-05', deadline: '2026-10-29' }, today)).toBe('today');
    expect(landing({ ...fields, project_id: 4 }, today)).toBe('anytime');
  });
});
