// Hash routes of the signed-in shell: `#/` (list) and `#/o/<id>` (detail). Anything else is the list.

export type Route = { view: 'list' } | { view: 'detail'; id: number };

export const listHref = '#/';

export function objectHref(id: number): string {
  return `#/o/${id}`;
}

export function parseHash(hash: string): Route {
  const m = /^#\/o\/([1-9]\d*)$/.exec(hash);
  return m ? { view: 'detail', id: Number(m[1]) } : { view: 'list' };
}

/** True when a key event comes from a control that handles keys itself (typing, buttons). */
export function ownsKeys(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || target.matches('input, select, textarea, button'));
}
