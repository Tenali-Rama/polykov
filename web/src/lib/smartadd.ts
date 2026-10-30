// Smart add calls (server API "Smart add"), both built on `post` and sent with the local date.
import { post } from './api';
import type { Obj } from './objects';

export type TokenKind = 'date' | 'time' | 'duration' | 'priority' | 'project' | 'area' | 'tag' | 'deadline' | 'recurrence';

/** `start`/`end` are UTF-16 offsets into the parsed text (JavaScript string indices). */
export type Token = { start: number; end: number; kind: TokenKind; text: string; resolved: boolean };

export type Recurrence = { rule: string; from: 'scheduled' | 'completion' };

/** Task fields a line gives; `null` when it doesn't. */
export type ParsedTask = {
  scheduled: string | null;
  deadline: string | null;
  estimate: number | null;
  priority: number | null;
  recurrence: Recurrence | null;
  project_id: number | null;
  area_id: number | null;
};

export type Parsed = { title: string; tokens: Token[]; tags: string[]; task: ParsedTask };

/** The task JSON `POST /api/tasks/add` returns (the fields the web reads). */
export type AddedTask = Obj & {
  task: Omit<ParsedTask, 'estimate' | 'priority' | 'recurrence'> & { parent_id: number | null; someday: boolean };
};

/** The local date `YYYY-MM-DD` (not UTC), which the server reads dates against. */
export function localToday(now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** `POST /api/parse`: the live preview, nothing written. `signal` cancels it. */
export async function parse(text: string, signal?: AbortSignal): Promise<Parsed> {
  return (await post('/api/parse', { text, today: localToday() }, signal)).json();
}

/** `POST /api/tasks/add`: creates the task. Throws `ApiError` 400 (e.g. `unknown project: x`). */
export async function add(text: string): Promise<AddedTask> {
  return (await post('/api/tasks/add', { text, today: localToday() })).json();
}

/** The GTD list a new task lands in (server "Lists" defaults), as of `today`. */
export function landing(task: AddedTask['task'], today: string): 'inbox' | 'today' | 'upcoming' | 'anytime' {
  const { scheduled, deadline } = task;
  if (!task.project_id && !task.area_id && !task.parent_id && !scheduled && !deadline && !task.someday) return 'inbox';
  const day = (scheduled ?? deadline)?.slice(0, 10);
  const due = deadline?.slice(0, 10);
  if ((day && day <= today) || (due && due <= today)) return 'today';
  if (day && day > today) return 'upcoming';
  return 'anytime';
}
