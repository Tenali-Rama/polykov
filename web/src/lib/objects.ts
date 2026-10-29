// Object and tag calls (server API "Objects", "GET /api/tags"), all built on `send`.
import { send } from './api';

/** Object types the server accepts (spec §3). */
export const TYPES = [
  'task',
  'project',
  'area',
  'goal',
  'milestone',
  'habit',
  'note',
  'notebook',
  'journal_entry',
  'daily_note',
  'review',
  'event',
  'time_entry',
  'view',
  'forecast_record',
] as const;

export type Obj = {
  id: number;
  type: string;
  title: string;
  created: string;
  updated: string;
  archived: boolean;
  pinned: boolean;
  tags: string[];
  properties: Record<string, unknown>;
};

export type Archived = 'false' | 'true' | 'any';

/** Query for `GET /api/objects`; unset fields use the server defaults. */
export type Filter = {
  type?: string;
  tag?: string;
  pinned?: boolean;
  archived?: Archived;
  limit?: number;
  offset?: number;
};

export type Page = { objects: Obj[]; total: number };
export type TagCount = { tag: string; count: number };

export type NewObj = {
  type: string;
  title: string;
  properties?: Record<string, unknown>;
  pinned?: boolean;
  archived?: boolean;
  tags?: string[];
};
export type Patch = Partial<Pick<Obj, 'title' | 'properties' | 'pinned' | 'archived' | 'tags'>>;

/** `?type=...&tag=...` for the set fields of `filter` (empty strings are skipped), or `''`. */
export function filterQuery(filter: Filter): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filter)) {
    if (value !== undefined && value !== '') params.set(key, String(value));
  }
  const query = params.toString();
  return query === '' ? '' : `?${query}`;
}

export async function listObjects(filter: Filter = {}): Promise<Page> {
  return (await send('GET', `/api/objects${filterQuery(filter)}`)).json();
}

/** Throws `ApiError` 404 (`object not found`) for an unknown or trashed id. */
export async function getObject(id: number): Promise<Obj> {
  return (await send('GET', `/api/objects/${id}`)).json();
}

export async function createObject(input: NewObj): Promise<Obj> {
  return (await send('POST', '/api/objects', input)).json();
}

export async function updateObject(id: number, patch: Patch): Promise<Obj> {
  return (await send('PATCH', `/api/objects/${id}`, patch)).json();
}

/** Moves the object to the trash. */
export async function deleteObject(id: number): Promise<void> {
  await send('DELETE', `/api/objects/${id}`);
}

export async function listTags(): Promise<TagCount[]> {
  const { tags } = (await (await send('GET', '/api/tags')).json()) as { tags: TagCount[] };
  return tags;
}
