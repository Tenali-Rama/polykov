import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import { addLink, findByTitle, getLinks, MATCHES, removeLink } from './links';
import type { Obj } from './objects';

function obj(id: number, title: string): Obj {
  return {
    id,
    type: 'note',
    title,
    created: '2026-10-29T09:15:00Z',
    updated: '2026-10-29T09:15:00Z',
    archived: false,
    pinned: false,
    tags: [],
    properties: {},
  };
}

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('link calls', () => {
  it('getLinks GETs both sides', async () => {
    const body = { links: [{ id: 2, type: 'task', title: 'Write' }], backlinks: [{ id: 3, type: 'note', title: 'Ideas' }] };
    const fetch = stubFetch(() => Response.json(body));
    expect(await getLinks(1)).toEqual(body);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects/1/links');
    expect(fetch.mock.calls[0][1]?.method).toBe('GET');
  });

  it('addLink PUTs and accepts an empty 204', async () => {
    const fetch = stubFetch(() => new Response(null, { status: 204 }));
    await addLink(1, 2);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects/1/links/2');
    expect(fetch.mock.calls[0][1]?.method).toBe('PUT');
  });

  it('addLink throws the server text for a self-link', async () => {
    stubFetch(() => Response.json({ error: 'cannot link an object to itself' }, { status: 400 }));
    const err = await addLink(1, 1).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).message).toBe('cannot link an object to itself');
  });

  it('removeLink DELETEs the link', async () => {
    const fetch = stubFetch(() => new Response(null, { status: 204 }));
    await removeLink(1, 2);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects/1/links/2');
    expect(fetch.mock.calls[0][1]?.method).toBe('DELETE');
  });
});

describe('findByTitle', () => {
  it('filters titles ignoring case, skips self and asks for archived too', async () => {
    const objects = [obj(1, 'Groceries'), obj(2, 'grocery run'), obj(3, 'Taxes'), obj(4, 'GROCER')];
    const fetch = stubFetch(() => Response.json({ objects, total: 4 }));
    const found = await findByTitle(' Grocer ', 4);
    expect(found.map((o) => o.id)).toEqual([1, 2]);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects?archived=any&limit=1000');
  });

  it('returns at most the first MATCHES matches in list order', async () => {
    const objects = Array.from({ length: 15 }, (_, i) => obj(i + 10, `note ${i}`));
    stubFetch(() => Response.json({ objects, total: 15 }));
    const found = await findByTitle('note', 1);
    expect(found).toHaveLength(MATCHES);
    expect(found[0].id).toBe(10);
    expect(found[MATCHES - 1].id).toBe(10 + MATCHES - 1);
  });
});
