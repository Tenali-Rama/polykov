// Pure helpers for the capture bar: highlight segments, chip labels, and quoting a token out.
import type { Parsed, Token, TokenKind } from '../lib/smartadd';

/** A run of `text`; `kind` is `null` for plain title text. */
export type Segment = { text: string; kind: TokenKind | null; resolved: boolean };

/**
 * Splits `text` at the token spans (UTF-16 offsets, in input order). Spans that overlap an earlier
 * one or fall outside `text` are skipped, so the segments always join back to `text`.
 */
export function segments(text: string, tokens: Token[]): Segment[] {
  const out: Segment[] = [];
  let at = 0;
  for (const t of tokens) {
    if (t.start < at || t.end > text.length || t.start >= t.end) continue;
    if (t.start > at) out.push({ text: text.slice(at, t.start), kind: null, resolved: true });
    out.push({ text: text.slice(t.start, t.end), kind: t.kind, resolved: t.resolved });
    at = t.end;
  }
  if (at < text.length) out.push({ text: text.slice(at), kind: null, resolved: true });
  return out;
}

/** `text` with the token's span wrapped in `"…"`, so the parser leaves it as title text. */
export function quote(text: string, token: Token): string {
  return `${text.slice(0, token.start)}"${text.slice(token.start, token.end)}"${text.slice(token.end)}`;
}

export type Chip = { token: Token; label: string };

/**
 * One chip per token, labelled with the value the parse took from it, e.g. `tomorrow`, `17:00`,
 * `30m`, `p2`, `#family`, `every sunday`. A repeated field (the parser keeps the first) shows the
 * token text.
 */
export function chips(parsed: Parsed, today: string): Chip[] {
  const seen = new Set<TokenKind>();
  return parsed.tokens.map((token) => {
    const first = !seen.has(token.kind);
    seen.add(token.kind);
    return { token, label: (first && valueLabel(token.kind, parsed, today)) || token.text.toLowerCase() };
  });
}

function valueLabel(kind: TokenKind, { task }: Parsed, today: string): string | null {
  switch (kind) {
    case 'date':
      return task.scheduled ? dayLabel(task.scheduled.slice(0, 10), today) : null;
    case 'time':
      return task.scheduled?.slice(11) || null;
    case 'duration':
      return task.estimate ? minutes(task.estimate) : null;
    case 'priority':
      return task.priority ? `p${task.priority}` : null;
    case 'deadline':
      return task.deadline ? `due ${dayLabel(task.deadline.slice(0, 10), today)} ${task.deadline.slice(11)}`.trim() : null;
    default:
      return null;
  }
}

/** `today`, `tomorrow`, else the date itself. */
export function dayLabel(day: string, today: string): string {
  if (day === today) return 'today';
  const next = new Date(`${today}T00:00:00Z`);
  next.setUTCDate(next.getUTCDate() + 1);
  return day === next.toISOString().slice(0, 10) ? 'tomorrow' : day;
}

/** `45m`, `2h`, `1h30m`. */
export function minutes(total: number): string {
  const h = Math.floor(total / 60);
  const m = total % 60;
  return h === 0 ? `${m}m` : m === 0 ? `${h}h` : `${h}h${m}m`;
}
