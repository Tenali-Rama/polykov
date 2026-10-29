import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import {
  deleteAttachment,
  downloadUrl,
  humanSize,
  listAttachments,
  uploadAttachment,
  type Attachment,
} from './attachments';

const att: Attachment = {
  id: 3,
  object_id: 7,
  name: 'résumé.pdf',
  mime: 'application/pdf',
  size: 5120,
  created: '2026-10-29T09:15:00Z',
};

function stubFetch(res: () => Response) {
  const fn = vi.fn<(path: string, init?: RequestInit) => Promise<Response>>(async () => res());
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('attachment calls', () => {
  it('listAttachments GETs and unwraps the attachments', async () => {
    const fetch = stubFetch(() => Response.json({ attachments: [att] }));
    expect(await listAttachments(7)).toEqual([att]);
    expect(fetch.mock.calls[0][0]).toBe('/api/objects/7/attachments');
    expect(fetch.mock.calls[0][1]?.method).toBe('GET');
  });

  it('uploadAttachment POSTs the raw file with its name encoded', async () => {
    const fetch = stubFetch(() => Response.json(att, { status: 201 }));
    const file = new File(['%PDF'], 'my résumé&v=2.pdf', { type: 'application/pdf' });
    expect(await uploadAttachment(7, file)).toEqual(att);
    const [url, init] = fetch.mock.calls[0];
    expect(url).toBe('/api/objects/7/attachments?name=my%20r%C3%A9sum%C3%A9%26v%3D2.pdf');
    expect(init?.method).toBe('POST');
    expect(init?.body).toBe(file);
    expect((init?.headers as Record<string, string>)['Content-Type']).toBe('application/pdf');
  });

  it('uploadAttachment throws ApiError 413 with the server text', async () => {
    stubFetch(() => Response.json({ error: 'attachment must be at most 25 MiB' }, { status: 413 }));
    const err = await uploadAttachment(7, new File(['x'], 'big.bin')).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(413);
    expect((err as ApiError).message).toBe('attachment must be at most 25 MiB');
  });

  it('deleteAttachment DELETEs and accepts an empty 204', async () => {
    const fetch = stubFetch(() => new Response(null, { status: 204 }));
    await deleteAttachment(3);
    expect(fetch.mock.calls[0][0]).toBe('/api/attachments/3');
    expect(fetch.mock.calls[0][1]?.method).toBe('DELETE');
  });

  it('downloadUrl adds inline=1 only when asked', () => {
    expect(downloadUrl(3)).toBe('/api/attachments/3');
    expect(downloadUrl(3, true)).toBe('/api/attachments/3?inline=1');
  });

  it('humanSize picks a binary unit', () => {
    expect(humanSize(512)).toBe('512 B');
    expect(humanSize(5120)).toBe('5.0 KiB');
    expect(humanSize(1258291)).toBe('1.2 MiB');
    expect(humanSize(3 * 1024 ** 3)).toBe('3.0 GiB');
  });
});
