// Trash calls (server API "Trash"), all built on `send`.
import { send } from './api';
import type { Obj } from './objects';

/** A trashed object: the object JSON plus when it was deleted and when the server purges it (RFC 3339). */
export type Trashed = Obj & { deleted: string; purge_at: string };

/** Newest deleted first. */
export async function listTrash(): Promise<Trashed[]> {
  const { objects } = (await (await send('GET', '/api/trash')).json()) as { objects: Trashed[] };
  return objects;
}

/** Throws `ApiError` 404 (`not in trash`) for an unknown or live id. */
export async function restoreObject(id: number): Promise<Obj> {
  return (await send('POST', `/api/trash/${id}/restore`)).json();
}

/** Deletes one trashed object for good. */
export async function purgeObject(id: number): Promise<void> {
  await send('DELETE', `/api/trash/${id}`);
}

/** Deletes every trashed object for good; returns how many. */
export async function emptyTrash(): Promise<number> {
  const { purged } = (await (await send('DELETE', '/api/trash')).json()) as { purged: number };
  return purged;
}
