// Comment calls (server API "Comments"), all built on `send`.
import { send } from './api';

/** Times are RFC 3339. */
export type Comment = { id: number; object_id: number; body: string; created: string; updated: string };

/** Oldest first. */
export async function listComments(id: number): Promise<Comment[]> {
  const { comments } = (await (await send('GET', `/api/objects/${id}/comments`)).json()) as { comments: Comment[] };
  return comments;
}

/** Throws `ApiError` 400 (`body must be 1 to 10000 characters`) for an empty or too long body. */
export async function addComment(id: number, body: string): Promise<Comment> {
  return (await send('POST', `/api/objects/${id}/comments`, { body })).json();
}

/** Replaces the body. Throws `ApiError` 400 as `addComment`, or 404 (`comment not found`). */
export async function editComment(cid: number, body: string): Promise<Comment> {
  return (await send('PATCH', `/api/comments/${cid}`, { body })).json();
}

export async function deleteComment(cid: number): Promise<void> {
  await send('DELETE', `/api/comments/${cid}`);
}
