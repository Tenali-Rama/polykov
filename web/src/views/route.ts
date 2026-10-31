// View hash routes: `#/views` (the index) and `#/v/<id>` (one view).

export type ViewRoute = { view: 'index' } | { view: 'view'; id: number };

export const viewsHref = '#/views';

export function viewHref(id: number): string {
  return `#/v/${id}`;
}

/** The view route of `hash`, or `null` when it is not one. */
export function parseViewHash(hash: string): ViewRoute | null {
  if (hash === viewsHref) return { view: 'index' };
  const m = /^#\/v\/([1-9]\d*)$/.exec(hash);
  return m ? { view: 'view', id: Number(m[1]) } : null;
}
