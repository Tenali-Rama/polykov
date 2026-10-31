// Kanban columns and moves: a card moved to another column becomes one task API call.
import { completeTask, reopenTask, updateTask, type FieldChanges, type Task } from '../lib/tasks';
import type { Group, GroupBy, View } from '../lib/views';

export type Key = Group['key'];
/** The task call that puts a task in a column. */
export type Move = { call: 'complete' } | { call: 'reopen' } | { call: 'patch'; task: FieldChanges };

/** Priority 4 is the server's "no priority": its column also holds the objects without one. */
const NO_PRIORITY = 4;
const MOVABLE: readonly GroupBy[] = ['status', 'priority', 'project', 'section', 'area'];
/** The columns of a fixed set of values, shown even when empty so a card can move there. */
const FIXED: Partial<Record<GroupBy, Group[]>> = {
  status: ['open', 'done'].map((s) => ({ key: s, label: s, items: [] })),
  priority: [1, 2, 3, 4].map((p) => ({ key: p, label: p === NO_PRIORITY ? `P${p} · none` : `P${p}`, items: [] })),
};

/** The grouping a view's board comes in: the server groups a kanban view without one by `status`. */
export function boardGroup(view: View): GroupBy {
  return view.properties.group === 'none' ? 'status' : view.properties.group;
}

/** Whether cards can move between columns of this grouping (`tag` and `type` are read-only). */
export function movable(group: GroupBy): boolean {
  return MOVABLE.includes(group);
}

/**
 * The server's groups as columns, in its order, plus the empty columns of a fixed set of values
 * (`status`, `priority`) in their place, with the fixed labels. The `none` group stays last, except
 * under `priority`, where its items join `P4 · none`.
 */
export function columns(group: GroupBy, groups: Group[]): Group[] {
  const fixed = FIXED[group];
  if (!fixed) return groups;
  const none = groups.filter((g) => g.key === null);
  const filled = fixed.map((f) => ({ ...f, items: [...(groups.find((g) => g.key === f.key)?.items ?? [])] }));
  if (group !== 'priority') return [...filled, ...none];
  filled[NO_PRIORITY - 1].items.push(...none.flatMap((g) => g.items));
  return filled;
}

/** The call that moves a task to column `key` under `group`; `null` when no call does. */
export function planMove(group: GroupBy, key: Key): Move | null {
  switch (group) {
    case 'status':
      return key === 'done' ? { call: 'complete' } : key === 'open' ? { call: 'reopen' } : null;
    case 'priority':
      return { call: 'patch', task: { priority: typeof key === 'number' ? key : NO_PRIORITY } };
    case 'project':
      return { call: 'patch', task: { project_id: idOf(key) } };
    case 'section':
      return { call: 'patch', task: { section_id: idOf(key) } };
    case 'area':
      return { call: 'patch', task: { area_id: idOf(key) } };
    default:
      return null;
  }
}

function idOf(key: Key): number | null {
  return typeof key === 'number' ? key : null;
}

/**
 * Moves task `id` to column `key` and returns the task as the server now has it, and `next`: the
 * new occurrence a completed recurring task gets, else `null`.
 */
export async function moveTask(group: GroupBy, id: number, key: Key): Promise<{ task: Task; next: Task | null }> {
  const move = planMove(group, key);
  if (!move) throw new Error(movable(group) ? `cannot move to ${key ?? 'none'}` : 'read-only grouping');
  if (move.call === 'complete') return completeTask(id);
  if (move.call === 'reopen') return { task: await reopenTask(id), next: null };
  return { task: await updateTask(id, { task: move.task }), next: null };
}
