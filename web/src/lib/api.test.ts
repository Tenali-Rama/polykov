import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, getJson, serverVersion } from './api';

afterEach(() => vi.unstubAllGlobals());

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
