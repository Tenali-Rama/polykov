import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import { createView, isTask, listViews, updateView, viewItems } from './views';
import { makeObj } from '../objects/testing';
import { makeRow } from '../tasks/testing';

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

const calls = (fn: ReturnType<typeof stubFetch>) => fn.mock.calls.map(([url, init]) => [init?.method, url, init?.body]);

afterEach(() => vi.unstubAllGlobals());

describe('view calls', () => {
  it('listViews unwraps the views', async () => {
    const fetch = stubFetch(() => Response.json({ views: [{ id: 4 }] }));
    expect(await listViews()).toEqual([{ id: 4 }]);
    expect(calls(fetch)).toEqual([['GET', '/api/views', undefined]]);
  });

  it('createView and updateView send their body as JSON', async () => {
    const fetch = stubFetch(() => Response.json({ id: 4 }));
    await createView({ title: 'Work', query: 'tag:work', group: 'priority' });
    await updateView(4, { pinned: true });
    expect(calls(fetch)).toEqual([
      ['POST', '/api/views', '{"title":"Work","query":"tag:work","group":"priority"}'],
      ['PATCH', '/api/views/4', '{"pinned":true}'],
    ]);
  });

  it('a query parse error carries its offset', async () => {
    stubFetch(() => Response.json({ error: 'query: expected a value', at: 9 }, { status: 400 }));
    const err = await createView({ title: 'x', query: 'priority:' }).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(400);
    expect((err as ApiError).at).toBe(9);
  });

  it('viewItems sends the local today, or the given one', async () => {
    const fetch = stubFetch(() => Response.json({ view: {}, groups: [], total: 0, truncated: false }));
    await viewItems(4, '2026-10-31');
    await viewItems(5);
    expect(fetch.mock.calls[0][0]).toBe('/api/views/4/items?today=2026-10-31');
    expect(fetch.mock.calls[1][0]).toMatch(/^\/api\/views\/5\/items\?today=\d{4}-\d{2}-\d{2}$/);
  });

  it('isTask tells task JSON from object JSON', () => {
    expect(isTask(makeRow(1))).toBe(true);
    expect(isTask(makeObj(2))).toBe(false);
  });
});
