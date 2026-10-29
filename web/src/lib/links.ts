// Link calls (server API "Links"), all built on `send`.
import { send } from './api';
import { listObjects, type Obj } from './objects';

/** A live object as a link end. */
export type Summary = Pick<Obj, 'id' | 'type' | 'title'>;
/** Outgoing links and backlinks, each ordered by title. */
export type Links = { links: Summary[]; backlinks: Summary[] };

/** How many title matches `findByTitle` returns. */
export const MATCHES = 10;

export async function getLinks(id: number): Promise<Links> {
  return (await send('GET', `/api/objects/${id}/links`)).json();
}

/** Links `id` to `to`; linking again is a no-op. Throws `ApiError` 400 for a self-link, 404 for an unknown end. */
export async function addLink(id: number, to: number): Promise<void> {
  await send('PUT', `/api/objects/${id}/links/${to}`);
}

/** Throws `ApiError` 404 (`link not found`). */
export async function removeLink(id: number, to: number): Promise<void> {
  await send('DELETE', `/api/objects/${id}/links/${to}`);
}

/**
 * Objects (archived too) whose title contains `query`, ignoring case, except `self`: the first
 * `MATCHES` in list order. The server has no title search, so this filters one page of up to 1000.
 */
export async function findByTitle(query: string, self: number): Promise<Obj[]> {
  const needle = query.trim().toLowerCase();
  const { objects } = await listObjects({ archived: 'any', limit: 1000 });
  return objects.filter((o) => o.id !== self && o.title.toLowerCase().includes(needle)).slice(0, MATCHES);
}
