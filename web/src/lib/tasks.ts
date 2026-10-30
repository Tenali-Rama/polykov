// Task and GTD list calls (server API "Tasks", "Lists"), all built on `send`.
import { send } from './api';
import type { Obj } from './objects';

export type Recurrence = { rule: string; from: 'scheduled' | 'completion' };
export type ChecklistItem = { text: string; done: boolean };

export type TaskFields = {
  status: 'open' | 'done';
  priority: number;
  scheduled: string | null;
  deadline: string | null;
  estimate: number | null;
  someday: boolean;
  parent_id: number | null;
  project_id: number | null;
  section_id: number | null;
  area_id: number | null;
  checklist: ChecklistItem[];
  recurrence: Recurrence | null;
  completed: string | null;
};

/** The task JSON: the object JSON (type `task`) plus its task fields. */
export type Task = Obj & { task: TaskFields };
/** A list row: a task plus its direct sub-task counts. */
export type Row = Task & { subtasks: { open: number; total: number } };

/** The task fields a request may set; `null` clears a nullable one. */
export type FieldChanges = Partial<Omit<TaskFields, 'status' | 'completed'>>;
export type NewTask = {
  title: string;
  tags?: string[];
  properties?: Record<string, unknown>;
  pinned?: boolean;
  task?: FieldChanges;
};
export type TaskPatch = Partial<Pick<Obj, 'title' | 'properties' | 'pinned' | 'archived' | 'tags'>> & {
  task?: FieldChanges;
};

export const LISTS = ['inbox', 'today', 'upcoming', 'someday', 'anytime'] as const;
/** A GTD list name; `all` is every top-level task (the project and area views). */
export type ListName = (typeof LISTS)[number] | 'all';
export type ListFilter = { project?: number; area?: number; limit?: number; offset?: number };

/** The browser's local date as `YYYY-MM-DD`; every list and complete call sends it as `today`. */
export function localToday(now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** One page of a list and the number of matches across all pages. */
export async function listTasks(name: ListName, filter: ListFilter = {}): Promise<{ tasks: Row[]; total: number }> {
  const q = new URLSearchParams({ today: localToday() });
  for (const [k, v] of Object.entries(filter)) if (v !== undefined) q.set(k, String(v));
  return (await send('GET', `/api/lists/${name}?${q}`)).json();
}

/** The direct children of a task, open first. */
export async function listSubtasks(id: number): Promise<Row[]> {
  const { tasks } = (await (await send('GET', `/api/tasks/${id}/subtasks`)).json()) as { tasks: Row[] };
  return tasks;
}

export async function getTask(id: number): Promise<Task> {
  return (await send('GET', `/api/tasks/${id}`)).json();
}

export async function createTask(input: NewTask): Promise<Task> {
  return (await send('POST', '/api/tasks', input)).json();
}

export async function updateTask(id: number, patch: TaskPatch): Promise<Task> {
  return (await send('PATCH', `/api/tasks/${id}`, patch)).json();
}

/** Marks the task done; `next` is the new occurrence of a recurring task, else `null`. */
export async function completeTask(id: number): Promise<{ task: Task; next: Task | null }> {
  return (await send('POST', `/api/tasks/${id}/complete?today=${localToday()}`)).json();
}

export async function reopenTask(id: number): Promise<Task> {
  return (await send('POST', `/api/tasks/${id}/reopen`)).json();
}
