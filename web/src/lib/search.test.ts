import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import { hitHref, isDone, pieces, search, type Found } from './search';
import { makeObj } from '../objects/testing';

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('search', () => {
  it('GETs /api/search with the encoded query, the limit and the signal', async () => {
    const body = { results: [{ object: makeObj(1), snippet: '[obj]ect 1', score: 12 }], total: 1 };
    const fetch = stubFetch(() => Response.json(body));
    const ctrl = new AbortController();
    expect(await search('a&b #c', { signal: ctrl.signal, limit: 10 })).toEqual(body);
    expect(fetch.mock.calls[0][0]).toBe('/api/search?q=a%26b%20%23c&limit=10');
    expect(fetch.mock.calls[0][1]?.method).toBe('GET');
    expect(fetch.mock.calls[0][1]?.signal).toBe(ctrl.signal);
  });

  it('leaves the limit to the server when not given', async () => {
    const fetch = stubFetch(() => Response.json({ results: [], total: 0 }));
    await search('x');
    expect(fetch.mock.calls[0][0]).toBe('/api/search?q=x');
  });

  it('throws the server error text', async () => {
    stubFetch(() => Response.json({ error: 'q must be 1 to 200 characters' }, { status: 400 }));
    const err = await search('').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).message).toBe('q must be 1 to 200 characters');
  });
});

describe('hit helpers', () => {
  it('isDone: a done task or project only', () => {
    const task = (status: string): Found => ({ ...makeObj(1, { type: 'task' }), task: { status } });
    const project = (status: string): Found => ({ ...makeObj(2, { type: 'project' }), project: { status } });
    expect(isDone(task('done'))).toBe(true);
    expect(isDone(task('open'))).toBe(false);
    expect(isDone(project('done'))).toBe(true);
    expect(isDone(project('active'))).toBe(false);
    expect(isDone(makeObj(3))).toBe(false);
  });

  it('hitHref: views to #/v/<id>, everything else to the object detail', () => {
    expect(hitHref(makeObj(4, { type: 'view' }))).toBe('#/v/4');
    expect(hitHref(makeObj(5, { type: 'project' }))).toBe('#/o/5');
    expect(hitHref(makeObj(6))).toBe('#/o/6');
  });

  it('pieces splits on the marks and drops them', () => {
    expect(pieces('[Meet]ing notes about [team]')).toEqual([
      { text: 'Meet', hit: true },
      { text: 'ing notes about ', hit: false },
      { text: 'team', hit: true },
    ]);
    expect(pieces('no marks')).toEqual([{ text: 'no marks', hit: false }]);
    expect(pieces('…a [b')).toEqual([{ text: '…a [b', hit: false }]);
    expect(pieces('')).toEqual([]);
  });
});
