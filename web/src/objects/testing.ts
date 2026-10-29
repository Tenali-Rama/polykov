// Test helpers: a fake `fetch` routed by `METHOD path` (query ignored) and object fixtures.
import { vi } from 'vitest';
import type { Obj } from '../lib/objects';

export type Route = (url: string, init?: RequestInit) => Response;

/** Stub `fetch`; unknown routes answer 599 so a test sees them. Returns the mock to inspect calls. */
export function stubApi(routes: Record<string, Route>) {
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    const route = routes[`${init?.method ?? 'GET'} ${url.split('?')[0]}`];
    return route ? route(url, init) : new Response(`no route ${url}`, { status: 599 });
  });
  vi.stubGlobal('fetch', fn);
  return fn;
}

/** URLs requested so far whose path starts with `prefix`. */
export function urls(fn: ReturnType<typeof stubApi>, prefix: string): string[] {
  return fn.mock.calls.map(([url]) => url).filter((url) => url.startsWith(prefix));
}

export function makeObj(id: number, over: Partial<Obj> = {}): Obj {
  return {
    id,
    type: 'note',
    title: `object ${id}`,
    created: '2026-10-29T09:15:00Z',
    updated: '2026-10-29T09:15:00Z',
    archived: false,
    pinned: false,
    tags: [],
    properties: {},
    ...over,
  };
}
