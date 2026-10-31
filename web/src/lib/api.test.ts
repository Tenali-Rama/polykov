import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, getJson, send, serverVersion } from './api';

afterEach(() => vi.unstubAllGlobals());

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

describe('send', () => {
  it('sends a plain body as JSON with the given method', async () => {
    const fetch = stubFetch(() => Response.json({}));
    await send('PATCH', '/api/objects/1', { title: 'x' });
    const [path, init] = fetch.mock.calls[0];
    expect(path).toBe('/api/objects/1');
    expect(init?.method).toBe('PATCH');
    expect((init?.headers as Record<string, string>)['Content-Type']).toBe('application/json');
    expect(init?.body).toBe('{"title":"x"}');
  });

  it('sends a Blob raw with its own type', async () => {
    const fetch = stubFetch(() => Response.json({}));
    const file = new File(['hi'], 'a.txt', { type: 'text/plain' });
    await send('POST', '/api/objects/1/attachments?name=a.txt', file);
    const init = fetch.mock.calls[0][1];
    expect(init?.body).toBe(file);
    expect((init?.headers as Record<string, string>)['Content-Type']).toBe('text/plain');
  });

  it('sends no body and no content type when body is undefined', async () => {
    const fetch = stubFetch(() => new Response(null, { status: 204 }));
    await send('DELETE', '/api/objects/1');
    const init = fetch.mock.calls[0][1];
    expect(init?.method).toBe('DELETE');
    expect(init?.body).toBeUndefined();
    expect(init?.headers).not.toHaveProperty('Content-Type');
  });

  it('throws ApiError with the server error text', async () => {
    stubFetch(() => Response.json({ error: 'object not found' }, { status: 404 }));
    const err = await send('GET', '/api/objects/9').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
    expect((err as ApiError).message).toBe('object not found');
    expect((err as ApiError).at).toBeUndefined();
  });

  it('carries the error offset `at` when the server gives one', async () => {
    stubFetch(() => Response.json({ error: 'query: expected a value', at: 7 }, { status: 400 }));
    const err = await send('POST', '/api/views', {}).catch((e: unknown) => e);
    expect((err as ApiError).message).toBe('query: expected a value');
    expect((err as ApiError).at).toBe(7);
  });

  it('falls back to path and status when the error body is not JSON', async () => {
    stubFetch(() => new Response('boom', { status: 500 }));
    const err = await send('PUT', '/api/x').catch((e: unknown) => e);
    expect((err as ApiError).message).toBe('/api/x: HTTP 500');
  });
});

describe('getJson', () => {
  it('throws ApiError carrying the status on non-2xx', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('nope', { status: 404 })));
    const err = await getJson('/api/x').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
  });
});

describe('serverVersion', () => {
  it('returns null when a 200 body is not JSON', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('<html>', { status: 200 })));
    expect(await serverVersion()).toBeNull();
  });

  it('returns null when status is not ok', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({ status: 'starting', version: '1' })));
    expect(await serverVersion()).toBeNull();
  });
});
