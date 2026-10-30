// Task hash routes: `#/<list>` for the five GTD lists, `#/p/<id>` (project) and `#/a/<id>` (area).
import { LISTS } from '../lib/tasks';

export type ListView = (typeof LISTS)[number];
export type TaskRoute = { view: 'list'; list: ListView } | { view: 'project'; id: number } | { view: 'area'; id: number };

export function taskListHref(list: ListView): string {
  return `#/${list}`;
}

export function projectHref(id: number): string {
  return `#/p/${id}`;
}

export function areaHref(id: number): string {
  return `#/a/${id}`;
}

/** The task route of `hash`, or `null` when it is not one (the object routes then apply). */
export function parseTaskHash(hash: string): TaskRoute | null {
  const list = LISTS.find((l) => hash === taskListHref(l));
  if (list) return { view: 'list', list };
  const m = /^#\/([pa])\/([1-9]\d*)$/.exec(hash);
  if (!m) return null;
  return { view: m[1] === 'p' ? 'project' : 'area', id: Number(m[2]) };
}
