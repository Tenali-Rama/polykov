// Saved view calls (server API "Views", ADR 0016), all built on `send`.
import { send } from './api';
import type { Obj } from './objects';
import { localToday, type Task } from './tasks';

export const LAYOUTS = ['list', 'kanban', 'eisenhower'] as const;
export const GROUPS = ['none', 'status', 'priority', 'project', 'section', 'area', 'tag', 'type'] as const;
export const SORTS = ['smart', 'priority', 'deadline', 'scheduled', 'created', 'updated', 'completed', 'title'] as const;
export type Layout = (typeof LAYOUTS)[number];
export type GroupBy = (typeof GROUPS)[number];
export type Sort = (typeof SORTS)[number];

/** A view's settings, kept in its `properties`. */
export type Settings = { query: string; layout: Layout; group: GroupBy; sort: Sort };
/** The view JSON: the object JSON (type `view`) with its settings as `properties`. */
export type View = Omit<Obj, 'properties'> & { properties: Settings };
export type NewView = { title: string; query: string } & Partial<Omit<Settings, 'query'>> & { pinned?: boolean };
export type ViewPatch = Partial<Settings> & { title?: string; pinned?: boolean };

/** A matched item: the task JSON for a task, else the object JSON. */
export type Item = Task | Obj;
/** One group; `key` is `null` for the `none` group (and for `group: none`). */
export type Group = { key: string | number | null; label: string; items: Item[] };
export type Items = { view: View; groups: Group[]; total: number; truncated: boolean };

export function isTask(item: Item): item is Task {
  return 'task' in item;
}

/** Live, non-archived views: pinned first, then by title. */
export async function listViews(): Promise<View[]> {
  const { views } = (await (await send('GET', '/api/views')).json()) as { views: View[] };
  return views;
}

/** Throws `ApiError` 400 on a bad setting; a query parse error carries `at`. */
export async function createView(input: NewView): Promise<View> {
  return (await send('POST', '/api/views', input)).json();
}

/** Replaces only the given fields; same errors as `createView`, plus 404 `view not found`. */
export async function updateView(id: number, patch: ViewPatch): Promise<View> {
  return (await send('PATCH', `/api/views/${id}`, patch)).json();
}

/** The view and its grouped items, relative dates anchored on the local `today`. */
export async function viewItems(id: number, today = localToday()): Promise<Items> {
  return (await send('GET', `/api/views/${id}/items?today=${today}`)).json();
}
