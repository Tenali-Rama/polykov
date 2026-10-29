import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import type { Obj } from './objects';
import { emptyTrash, listTrash, purgeObject, restoreObject, type Trashed } from './trash';

const obj: Obj = {
  id: 1,
  type: 'note',
  title: 'Groceries',
  created: '2026-10-29T09:15:00Z',
  updated: '2026-10-29T09:15:00Z',
  archived: false,
  pinned: false,
  tags: [],
  properties: {},
};
const trashed: Trashed = { ...obj, deleted: '2026-10-29T10:00:00Z', purge_at: '2026-11-28T10:00:00Z' };

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('trash calls', () => {
  it('listTrash GETs and unwraps the objects', async () => {
    const fetch = stubFetch(() => Response.json({ objects: [trashed] }));
    expect(await listTrash()).toEqual([trashed]);
    expect(fetch.mock.calls[0][0]).toBe('/api/trash');
    expect(fetch.mock.calls[0][1]?.method).toBe('GET');
  });

  it('restoreObject POSTs the restore and returns the object', async () => {
    const fetch = stubFetch(() => Response.json(obj));
    expect(await restoreObject(1)).toEqual(obj);
    expect(fetch.mock.calls[0][0]).toBe('/api/trash/1/restore');
    expect(fetch.mock.calls[0][1]?.method).toBe('POST');
  });

  it('restoreObject throws ApiError 404 for an id not in the trash', async () => {
    stubFetch(() => Response.json({ error: 'not in trash' }, { status: 404 }));
    const err = await restoreObject(9).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
    expect((err as ApiError).message).toBe('not in trash');
  });

  it('purgeObject DELETEs one and accepts an empty 204', async () => {
    const fetch = stubFetch(() => new Response(null, { status: 204 }));
    await purgeObject(1);
    expect(fetch.mock.calls[0][0]).toBe('/api/trash/1');
    expect(fetch.mock.calls[0][1]?.method).toBe('DELETE');
  });

  it('emptyTrash DELETEs the trash and returns the purged count', async () => {
    const fetch = stubFetch(() => Response.json({ purged: 3 }));
    expect(await emptyTrash()).toBe(3);
    expect(fetch.mock.calls[0][0]).toBe('/api/trash');
    expect(fetch.mock.calls[0][1]?.method).toBe('DELETE');
  });
});
