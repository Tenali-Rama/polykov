// Attachment calls (server API "Attachments"), all built on `send`.
import { send } from './api';

/** The server attachment JSON. `created` is RFC 3339. */
export type Attachment = { id: number; object_id: number; name: string; mime: string; size: number; created: string };

/** Oldest first. */
export async function listAttachments(id: number): Promise<Attachment[]> {
  const res = await send('GET', `/api/objects/${id}/attachments`);
  const { attachments } = (await res.json()) as { attachments: Attachment[] };
  return attachments;
}

/** Sends the file as the raw body (its type as `Content-Type`); throws `ApiError` 413 over 25 MiB. */
export async function uploadAttachment(id: number, file: File): Promise<Attachment> {
  return (await send('POST', `/api/objects/${id}/attachments?name=${encodeURIComponent(file.name)}`, file)).json();
}

/** Deletes the row and the file. */
export async function deleteAttachment(aid: number): Promise<void> {
  await send('DELETE', `/api/attachments/${aid}`);
}

/** The file's URL; `inline` asks for an `<img>`-friendly response (images only, server side). */
export function downloadUrl(aid: number, inline = false): string {
  return `/api/attachments/${aid}${inline ? '?inline=1' : ''}`;
}

/** `512 B`, `5.0 KiB`, `1.2 MiB`, ... (as the CLI). */
export function humanSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  let value = bytes / 1024;
  for (const unit of ['KiB', 'MiB']) {
    if (value < 1024) return `${value.toFixed(1)} ${unit}`;
    value /= 1024;
  }
  return `${value.toFixed(1)} GiB`;
}
