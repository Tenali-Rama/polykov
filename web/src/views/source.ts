// What a layout shows: a source of grouped items, plus the load and row-key handling the layouts share.
import { hitHref } from '../lib/search';
import type { Row, Task } from '../lib/tasks';
import { viewItems, type Group, type GroupBy, type Item, type View } from '../lib/views';
import { ownsKeys } from '../objects/route';
import { TASKS_CHANGED } from '../tasks/TaskList.svelte';
import { boardGroup } from './move';

export type Loaded = { groups: Group[]; total: number; truncated: boolean };
/** `group`: the grouping the loaded groups come in (what a Kanban move changes). */
export type Source = { group: GroupBy; load(): Promise<Loaded> };

/** A saved view's items. A kanban view without a grouping comes grouped by `status`, as the server does. */
export function viewSource(view: View): Source {
  return {
    group: view.properties.layout === 'kanban' ? boardGroup(view) : view.properties.group,
    async load() {
      const { groups, total, truncated } = await viewItems(view.id);
      return { groups, total, truncated };
    },
  };
}

/**
 * Loads `source` into `ondata`, clearing `onerror` first and passing it `error: <cause>` on a failure.
 * Only the latest request applies, so a slow, stale response never overwrites a newer one.
 * `watch()` (for `onMount`) loads now and on every `TASKS_CHANGED`, and returns the cleanup.
 */
export function loader(source: Source, ondata: (data: Loaded) => void, onerror: (error: string) => void) {
  let seq = 0;
  async function load() {
    const mine = ++seq;
    onerror('');
    try {
      const data = await source.load();
      if (mine === seq) ondata(data);
    } catch (e) {
      if (mine === seq) onerror(`error: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  function watch() {
    void load();
    const reload = () => void load();
    window.addEventListener(TASKS_CHANGED, reload);
    return () => window.removeEventListener(TASKS_CHANGED, reload);
  }
  return { load, watch };
}

/** Each group's items paired with their index across all groups (the j/k order). */
export function indexed(groups: Group[]) {
  let i = 0;
  return groups.map((g) => ({ ...g, items: g.items.map((item) => ({ item, i: i++ })) }));
}

/** Items carry no sub-task counts, so their rows show none. */
export function taskRow(item: Task): Row {
  return { ...item, subtasks: { open: 0, total: 0 } };
}

type TaskRowRef = { focus(): void; toggle(): Promise<void> };
/** The rendered rows by j/k index: a `TaskRow` for a task, else the object's link. */
export type RowRefs = { rows: (TaskRowRef | undefined)[]; links: (HTMLAnchorElement | undefined)[] };

function keepsKeys(target: EventTarget | null): boolean {
  return ownsKeys(target) || (target instanceof HTMLElement && target.closest('[data-owns-keys]') !== null);
}

/** Completes (or reopens) a task, then reloads every list, so it leaves a view it no longer matches. */
async function toggle(row: TaskRowRef) {
  await row.toggle();
  window.dispatchEvent(new Event(TASKS_CHANGED));
}

/**
 * The `TaskList` keys over `items` (in j/k order) with `sel` selected (-1: none yet): `j`/`k` select
 * and focus the next/previous item, `x` toggles a selected task, `Enter` opens the selected item
 * unless a link has focus (it follows itself). Ignored while a control owns the keys.
 * Returns the new selection when the key was handled (default prevented), else `null`.
 */
export function rowKey(e: KeyboardEvent, sel: number, items: Item[], refs: RowRefs): number | null {
  if (e.ctrlKey || e.metaKey || e.altKey || keepsKeys(e.target) || items.length === 0) return null;
  const row = refs.rows[sel];
  if (e.key === 'j' || e.key === 'k') {
    sel = Math.min(Math.max(sel + (e.key === 'j' ? 1 : -1), 0), items.length - 1);
    const next = refs.rows[sel];
    if (next) next.focus();
    else refs.links[sel]?.focus();
  } else if (e.key === 'x' && row) void toggle(row);
  else if (e.key === 'Enter' && sel >= 0 && !(e.target instanceof HTMLAnchorElement))
    location.hash = hitHref(items[sel]);
  else return null;
  e.preventDefault();
  return sel;
}
