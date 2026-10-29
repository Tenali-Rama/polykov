import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import { addComment, deleteComment, editComment, listComments, type Comment } from './comments';

const comment: Comment = {
  id: 1,
  object_id: 2,
  body: 'looks good',
  created: '2026-10-29T09:15:00Z',
  updated: '2026-10-29T09:15:00Z',
};

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('comment calls', () => {
  it('listComments GETs and unwraps the comments', async () => {
    const fetch = stubFetch(() => Response.json({ comments: [comment] }));
    expect(await listComments(2)).toEqual([comment]);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects/2/comments');
    expect(fetch.mock.calls[0][1]?.method).toBe('GET');
  });

  it('addComment POSTs the body and returns the comment', async () => {
    const fetch = stubFetch(() => Response.json(comment, { status: 201 }));
    expect(await addComment(2, 'looks good')).toEqual(comment);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects/2/comments');
    expect(fetch.mock.calls[0][1]?.method).toBe('POST');
    expect(JSON.parse(String(fetch.mock.calls[0][1]?.body))).toEqual({ body: 'looks good' });
  });

  it('addComment throws the server 400 text', async () => {
    stubFetch(() => Response.json({ error: 'body must be 1 to 10000 characters' }, { status: 400 }));
    const err = await addComment(2, ' ').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(400);
    expect((err as ApiError).message).toBe('body must be 1 to 10000 characters');
  });

  it('editComment PATCHes the comment', async () => {
    const fetch = stubFetch(() => Response.json({ ...comment, body: 'better' }));
    expect((await editComment(1, 'better')).body).toBe('better');
    expect(fetch.mock.calls[0][0]).toBe('/api/comments/1');
    expect(fetch.mock.calls[0][1]?.method).toBe('PATCH');
    expect(JSON.parse(String(fetch.mock.calls[0][1]?.body))).toEqual({ body: 'better' });
  });

  it('deleteComment DELETEs and accepts an empty 204', async () => {
    const fetch = stubFetch(() => new Response(null, { status: 204 }));
    await deleteComment(1);
    expect(fetch.mock.calls[0][0]).toBe('/api/comments/1');
    expect(fetch.mock.calls[0][1]?.method).toBe('DELETE');
  });
});
