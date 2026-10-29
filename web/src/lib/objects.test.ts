import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import {
  createObject,
  deleteObject,
  filterQuery,
  getObject,
  listObjects,
  listTags,
  updateObject,
  type Obj,
} from './objects';

const obj: Obj = {
  id: 1,
  type: 'note',
  title: 'Groceries',
  created: '2026-10-29T09:15:00Z',
  updated: '2026-10-29T09:15:00Z',
  archived: false,
  pinned: false,
  tags: ['shop/food'],
  properties: { aisle: 3 },
};

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('filterQuery', () => {
  it('is empty for no filter', () => {
    expect(filterQuery({})).toBe('');
  });

  it('encodes set fields and skips unset and empty ones', () => {
    expect(
      filterQuery({ type: 'task', tag: 'work/client a', pinned: true, archived: 'any', limit: 50, offset: 0 }),
    ).toBe('?type=task&tag=work%2Fclient+a&pinned=true&archived=any&limit=50&offset=0');
    expect(filterQuery({ type: '', tag: undefined, archived: 'false' })).toBe('?archived=false');
  });
});

describe('object calls', () => {
  it('listObjects GETs the filtered list', async () => {
    const fetch = stubFetch(() => Response.json({ objects: [obj], total: 1 }));
    expect(await listObjects({ type: 'note' })).toEqual({ objects: [obj], total: 1 });
    expect(fetch.mock.calls[0][0]).toBe('/api/objects?type=note');
    expect(fetch.mock.calls[0][1]?.method).toBe('GET');
  });

  it('getObject throws ApiError 404 for an unknown id', async () => {
    stubFetch(() => Response.json({ error: 'object not found' }, { status: 404 }));
    const err = await getObject(9).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
  });

  it('createObject POSTs the input and returns the object', async () => {
    const fetch = stubFetch(() => Response.json(obj, { status: 201 }));
    expect(await createObject({ type: 'note', title: 'Groceries' })).toEqual(obj);
    const [path, init] = fetch.mock.calls[0];
    expect(path).toBe('/api/objects');
    expect(init?.method).toBe('POST');
    expect(init?.body).toBe('{"type":"note","title":"Groceries"}');
  });

  it('createObject surfaces the server 400 text', async () => {
    stubFetch(() => Response.json({ error: 'title must be 1 to 1000 characters' }, { status: 400 }));
    await expect(createObject({ type: 'note', title: ' ' })).rejects.toThrow('title must be 1 to 1000 characters');
  });

  it('updateObject PATCHes the object', async () => {
    const fetch = stubFetch(() => Response.json({ ...obj, pinned: true }));
    expect((await updateObject(1, { pinned: true })).pinned).toBe(true);
    const [path, init] = fetch.mock.calls[0];
    expect(path).toBe('/api/objects/1');
    expect(init?.method).toBe('PATCH');
    expect(init?.body).toBe('{"pinned":true}');
  });

  it('deleteObject DELETEs and accepts an empty 204', async () => {
    const fetch = stubFetch(() => new Response(null, { status: 204 }));
    await deleteObject(1);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects/1');
    expect(fetch.mock.calls[0][1]?.method).toBe('DELETE');
  });

  it('listTags unwraps the tags array', async () => {
    stubFetch(() => Response.json({ tags: [{ tag: 'work', count: 2 }] }));
    expect(await listTags()).toEqual([{ tag: 'work', count: 2 }]);
  });
});
