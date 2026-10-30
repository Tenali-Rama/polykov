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

/**
 * Estimate text to whole minutes: `90`, `90m`, `2h`, `1h30m` (spaces and case ignored).
 * `null` when the text is none of these; the 1..10080 range is the server's rule.
 */
export function parseEstimate(text: string): number | null {
  const m = /^(?:(\d+)h)?(?:(\d+)m?)?$/.exec(text.replace(/\s+/g, '').toLowerCase());
  if (!m || (m[1] === undefined && m[2] === undefined)) return null;
  return Number(m[1] ?? 0) * 60 + Number(m[2] ?? 0);
}

/** Minutes as `45m`, `2h` or `1h30m`. */
export function formatEstimate(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return h === 0 ? `${m}m` : m === 0 ? `${h}h` : `${h}h${m}m`;
}

/** A task date (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`) split into its date and its time (`''` if none). */
export function splitDate(value: string | null): { date: string; time: string } {
  const [date = '', time = ''] = (value ?? '').split('T');
  return { date, time };
}

/** The task date for a date and an optional time; `null` (cleared) without a date. */
export function joinDate(date: string, time: string): string | null {
  if (date === '') return null;
  return time === '' ? date : `${date}T${time}`;
}

export const FREQS = ['day', 'week', 'month', 'year'] as const;
export type Freq = (typeof FREQS)[number];
/** Monday first, as the server's canonical `BYDAY`. */
export const WEEKDAYS = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const;
export type Weekday = (typeof WEEKDAYS)[number];
/** A recurrence rule (server `recur::Rule`): `byDay` only for `week`, `monthDay` only for `month`. */
export type RuleParts = { freq: Freq; interval: number; byDay: Weekday[]; monthDay: number | null };

const FREQ_WORDS: Record<Freq, string> = { day: 'DAILY', week: 'WEEKLY', month: 'MONTHLY', year: 'YEARLY' };
const dayCode = (d: Weekday) => d.slice(0, 2).toUpperCase();

/** The rule in the server's canonical form: `FREQ`, then `INTERVAL` if not 1, `BYDAY`, `BYMONTHDAY`. */
export function buildRule(parts: RuleParts): string {
  const out = [`FREQ=${FREQ_WORDS[parts.freq]}`];
  if (parts.interval !== 1) out.push(`INTERVAL=${parts.interval}`);
  const days = WEEKDAYS.filter((d) => parts.byDay.includes(d));
  if (parts.freq === 'week' && days.length > 0) out.push(`BYDAY=${days.map(dayCode).join(',')}`);
  if (parts.freq === 'month' && parts.monthDay !== null) out.push(`BYMONTHDAY=${parts.monthDay}`);
  return out.join(';');
}

/** Reads a rule the server served (any key order and case). */
export function parseRule(rule: string): RuleParts {
  const parts: RuleParts = { freq: 'day', interval: 1, byDay: [], monthDay: null };
  for (const pair of rule.toUpperCase().split(';')) {
    const [key, value = ''] = pair.split('=');
    if (key === 'FREQ') parts.freq = FREQS.find((f) => FREQ_WORDS[f] === value) ?? 'day';
    else if (key === 'INTERVAL') parts.interval = Number(value);
    else if (key === 'BYDAY') parts.byDay = WEEKDAYS.filter((d) => value.split(',').includes(dayCode(d)));
    else if (key === 'BYMONTHDAY') parts.monthDay = Number(value);
  }
  return parts;
}

/** E.g. `every 2 weeks on mon, thu · from completion`, `every month on day 15 · from scheduled`. */
export function describeRecurrence(rec: Recurrence): string {
  const p = parseRule(rec.rule);
  let text = p.interval === 1 ? `every ${p.freq}` : `every ${p.interval} ${p.freq}s`;
  if (p.freq === 'week' && p.byDay.length > 0) text += ` on ${p.byDay.join(', ')}`;
  if (p.freq === 'month' && p.monthDay !== null) text += ` on day ${p.monthDay}`;
  return `${text} · from ${rec.from}`;
}
