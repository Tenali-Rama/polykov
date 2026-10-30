import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import {
  createSection,
  deleteSection,
  getProject,
  listProjects,
  listSections,
  updateProject,
  updateSection,
} from './projects';

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

const calls = (fn: ReturnType<typeof stubFetch>) => fn.mock.calls.map(([url, init]) => [init?.method, url, init?.body]);

afterEach(() => vi.unstubAllGlobals());

describe('project calls', () => {
  it('listProjects unwraps and passes only the set filters', async () => {
    const fetch = stubFetch(() => Response.json({ projects: [{ id: 2 }] }));
    expect(await listProjects()).toEqual([{ id: 2 }]);
    await listProjects({ status: 'any', area: 3 });
    expect(calls(fetch)).toEqual([
      ['GET', '/api/projects', undefined],
      ['GET', '/api/projects?status=any&area=3', undefined],
    ]);
  });

  it('getProject throws ApiError 404', async () => {
    stubFetch(() => Response.json({ error: 'project not found' }, { status: 404 }));
    const err = await getProject(9).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(404);
  });

  it('updateProject PATCHes the changes', async () => {
    const fetch = stubFetch(() => Response.json({ id: 2 }));
    await updateProject(2, { status: 'someday', area_id: null });
    expect(calls(fetch)).toEqual([['PATCH', '/api/projects/2', '{"status":"someday","area_id":null}']]);
  });
});

describe('section calls', () => {
  it('list, create (default last), update and delete', async () => {
    const fetch = stubFetch(() => Response.json({ sections: [{ id: 1 }] }));
    expect(await listSections(2)).toEqual([{ id: 1 }]);
    await createSection(2, 'Plan');
    await createSection(2, 'First', 0);
    await updateSection(1, { position: 2 });
    stubFetch(() => new Response(null, { status: 204 }));
    await deleteSection(1);
    expect(calls(fetch)).toEqual([
      ['GET', '/api/projects/2/sections', undefined],
      ['POST', '/api/projects/2/sections', '{"title":"Plan"}'],
      ['POST', '/api/projects/2/sections', '{"title":"First","position":0}'],
      ['PATCH', '/api/sections/1', '{"position":2}'],
    ]);
  });

  it('deleteSection DELETEs and surfaces a 404', async () => {
    const fetch = stubFetch(() => Response.json({ error: 'section not found' }, { status: 404 }));
    const err = await deleteSection(7).catch((e: unknown) => e);
    expect((err as ApiError).message).toBe('section not found');
    expect(calls(fetch)).toEqual([['DELETE', '/api/sections/7', undefined]]);
  });
});
