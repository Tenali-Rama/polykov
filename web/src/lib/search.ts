// Global search (server API `GET /api/search`), built on `send`.
import { send } from './api';
import { objectHref } from '../objects/route';
import type { Obj } from './objects';

/** A hit's object: tasks and projects carry their own fields, so `done` is visible. */
export type Found = Obj & { task?: { status: string }; project?: { status: string } };
/** One ranked hit; `snippet` marks each matched word with `[` `]`. */
export type Hit = { object: Found; snippet: string; score: number };
export type Results = { results: Hit[]; total: number };
/** A run of snippet text; `hit` when it was inside `[` `]`. */
export type Piece = { text: string; hit: boolean };

/** `GET /api/search?q=<q>&limit=<limit>`; `signal` aborts it. Failures throw `ApiError`. */
export async function search(q: string, opts: { signal?: AbortSignal; limit?: number } = {}): Promise<Results> {
  const limit = opts.limit === undefined ? '' : `&limit=${opts.limit}`;
  return (await send('GET', `/api/search?q=${encodeURIComponent(q)}${limit}`, undefined, opts.signal)).json();
}

/** A completed task, or a project with status `done`. */
export function isDone(o: Found): boolean {
  return o.task?.status === 'done' || o.project?.status === 'done';
}

/** Where a hit leads: a view to its page (ADR 0016), anything else to the object detail. */
export function hitHref(o: Found): string {
  return o.type === 'view' ? `#/v/${o.id}` : objectHref(o.id);
}

/** Splits a snippet on its `[` `]` marks; an unclosed `[` stays plain text. Pieces join back to the text without marks. */
export function pieces(snippet: string): Piece[] {
  const out: Piece[] = [];
  const re = /\[([^\]]*)\]/g;
  let at = 0;
  for (let m = re.exec(snippet); m; m = re.exec(snippet)) {
    if (m.index > at) out.push({ text: snippet.slice(at, m.index), hit: false });
    if (m[1]) out.push({ text: m[1], hit: true });
    at = m.index + m[0].length;
  }
  if (at < snippet.length) out.push({ text: snippet.slice(at), hit: false });
  return out;
}
