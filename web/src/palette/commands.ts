// Palette commands and the window events that join the palette and the capture bar.
import { LISTS } from '../lib/tasks';
import { listHref, trashHref } from '../objects/route';
import { taskListHref } from '../tasks/route';

/** Opens the palette; `detail: { text }` prefills its input. */
export const PALETTE_OPEN = 'polykov:palette-open';
/** Opens the capture bar. */
export const CAPTURE_OPEN = 'polykov:capture-open';

export type Command = { label: string; run: () => void };

const go = (hash: string) => () => {
  location.hash = hash;
};

export const COMMANDS: Command[] = [
  ...LISTS.map((list) => ({ label: `go to ${list}`, run: go(taskListHref(list)) })),
  { label: 'go to objects', run: go(listHref) },
  { label: 'go to trash', run: go(trashHref) },
  { label: 'new task', run: () => window.dispatchEvent(new CustomEvent(CAPTURE_OPEN)) },
];

/** True when the characters of `query` occur in `label` in order, ignoring case. */
export function subsequence(query: string, label: string): boolean {
  const hay = label.toLowerCase();
  let at = 0;
  for (const ch of query.toLowerCase()) {
    at = hay.indexOf(ch, at);
    if (at < 0) return false;
    at += 1;
  }
  return true;
}

/** The commands whose label matches `query`, in list order; all of them for an empty `query`. */
export function filterCommands(query: string, commands: Command[] = COMMANDS): Command[] {
  return commands.filter((c) => subsequence(query, c.label));
}
