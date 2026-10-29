import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import Attachments from './Attachments.svelte';
import { stubApi, type Route } from './testing';
import type { Attachment } from '../lib/attachments';

function att(id: number, name: string, mime: string, size = 5120): Attachment {
  return { id, object_id: 7, name, mime, size, created: '2026-10-29T09:15:00Z' };
}

const items = [att(1, 'report.pdf', 'application/pdf'), att(2, 'cat.png', 'image/png', 512)];

function api(more: Record<string, Route> = {}, list = items) {
  return stubApi({ 'GET /api/objects/7/attachments': () => Response.json({ attachments: list }), ...more });
}

/** Answers each upload with a new attachment named from `?name=`, and records the request. */
function uploads() {
  const seen: { url: string; body: unknown }[] = [];
  const route: Route = (url, init) => {
    seen.push({ url, body: init?.body });
    const name = new URL(url, 'http://x').searchParams.get('name') ?? '';
    return Response.json(att(10 + seen.length, name, 'text/plain', 3), { status: 201 });
  };
  return { seen, route };
}

function row(name: string) {
  const li = screen.getByText(name).closest('li');
  if (!li) throw new Error(`no row ${name}`);
  return within(li);
}

async function open() {
  render(Attachments, { props: { id: 7 } });
  await waitFor(() => expect(screen.queryByText('loading …')).not.toBeInTheDocument());
}

const pick = () => screen.getByLabelText('files to attach') as HTMLInputElement;

async function choose(files: File[]) {
  Object.defineProperty(pick(), 'files', { value: files, configurable: true });
  await fireEvent.change(pick());
}

afterEach(() => vi.unstubAllGlobals());

describe('attachments', () => {
  it('lists rows with name, human size, mime and a download link', async () => {
    api();
    await open();
    const r = row('report.pdf');
    expect(r.getByText('5.0 KiB')).toBeInTheDocument();
    expect(r.getByText('application/pdf')).toBeInTheDocument();
    expect(r.getByRole('link', { name: '[ download ]' })).toHaveAttribute('href', '/api/attachments/1');
    expect(r.queryByRole('img')).not.toBeInTheDocument();
  });

  it('shows an inline thumbnail for image rows', async () => {
    api();
    await open();
    expect(row('cat.png').getByRole('img')).toHaveAttribute('src', '/api/attachments/2?inline=1');
  });

  it('says so when there are none', async () => {
    api({}, []);
    await open();
    expect(screen.getByText('no attachments · drop files here')).toBeInTheDocument();
  });

  it('[ attach ] opens the file picker', async () => {
    api();
    await open();
    const click = vi.spyOn(pick(), 'click').mockImplementation(() => {});
    await fireEvent.click(screen.getByRole('button', { name: '[ attach ]' }));
    expect(click).toHaveBeenCalled();
  });

  it('uploads each picked file as the raw body with its name encoded', async () => {
    const { seen, route } = uploads();
    api({ 'POST /api/objects/7/attachments': route });
    await open();
    const a = new File(['hi'], 'a b&c.txt', { type: 'text/plain' });
    const b = new File(['yo'], 'b.txt', { type: 'text/plain' });
    await choose([a, b]);
    expect(await screen.findByText('b.txt')).toBeInTheDocument();
    expect(screen.getByText('a b&c.txt')).toBeInTheDocument();
    expect(seen.map((s) => s.url)).toEqual([
      '/api/objects/7/attachments?name=a%20b%26c.txt',
      '/api/objects/7/attachments?name=b.txt',
    ]);
    expect(seen[0].body).toBe(a);
    expect(seen[1].body).toBe(b);
  });

  it('shows uploading <name>… while a request is in flight', async () => {
    let finish: (res: Response) => void = () => {};
    vi.stubGlobal('fetch', async (_url: string, init?: RequestInit) =>
      init?.method === 'POST'
        ? new Promise<Response>((resolve) => (finish = resolve))
        : Response.json({ attachments: [] }),
    );
    await open();
    await choose([new File(['x'], 'slow.txt')]);
    expect(await screen.findByRole('status')).toHaveTextContent('uploading slow.txt…');
    expect(screen.getByRole('button', { name: '[ attach ]' })).toBeDisabled();
    finish(Response.json(att(5, 'slow.txt', 'text/plain', 1), { status: 201 }));
    await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
    expect(screen.getByText('slow.txt')).toBeInTheDocument();
  });

  it('uploads dropped files, one request per file', async () => {
    const { seen, route } = uploads();
    api({ 'POST /api/objects/7/attachments': route }, []);
    await open();
    const files = [new File(['1'], 'one.txt'), new File(['2'], 'two.txt')];
    await fireEvent.drop(screen.getByRole('region', { name: 'attachments' }), { dataTransfer: { files } });
    expect(await screen.findByText('two.txt')).toBeInTheDocument();
    expect(seen).toHaveLength(2);
    expect(seen.map((s) => s.body)).toEqual(files);
  });

  it('ignores a drop while an upload is running', async () => {
    let finish: (res: Response) => void = () => {};
    const posts: string[] = [];
    vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
      if (init?.method !== 'POST') return Response.json({ attachments: [] });
      posts.push(url);
      return new Promise<Response>((resolve) => (finish = resolve));
    });
    await open();
    await choose([new File(['x'], 'slow.txt')]);
    await screen.findByRole('status');
    const panel = screen.getByRole('region', { name: 'attachments' });
    await fireEvent.drop(panel, { dataTransfer: { files: [new File(['y'], 'late.txt')] } });
    finish(Response.json(att(5, 'slow.txt', 'text/plain', 1), { status: 201 }));
    await waitFor(() => expect(screen.queryByRole('status')).not.toBeInTheDocument());
    expect(posts).toEqual(['/api/objects/7/attachments?name=slow.txt']);
    expect(screen.queryByText('late.txt')).not.toBeInTheDocument();
  });

  it('shows file too large on a 413', async () => {
    api({
      'POST /api/objects/7/attachments': () =>
        Response.json({ error: 'attachment must be at most 25 MiB' }, { status: 413 }),
    });
    await open();
    await choose([new File(['x'], 'huge.iso')]);
    expect(await screen.findByRole('alert')).toHaveTextContent('file too large (max 25 MiB)');
    expect(screen.queryByText('huge.iso')).not.toBeInTheDocument();
  });

  it('shows the server text for other upload errors', async () => {
    api({
      'POST /api/objects/7/attachments': () =>
        Response.json({ error: 'name must be 1 to 255 characters with no slashes or control characters' }, { status: 400 }),
    });
    await open();
    await choose([new File(['x'], 'bad\u0001name')]);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'name must be 1 to 255 characters with no slashes or control characters',
    );
  });

  it('deletes only after the confirm', async () => {
    const fetch = api({ 'DELETE /api/attachments/1': () => new Response(null, { status: 204 }) });
    await open();
    await fireEvent.click(row('report.pdf').getByRole('button', { name: '[ delete ]' }));
    expect(row('report.pdf').getByText('delete report.pdf?')).toBeInTheDocument();
    expect(fetch.mock.calls.some(([, init]) => init?.method === 'DELETE')).toBe(false);
    await fireEvent.click(row('report.pdf').getByRole('button', { name: '[ yes ]' }));
    await waitFor(() => expect(screen.queryByText('report.pdf')).not.toBeInTheDocument());
    expect(fetch.mock.calls.filter(([, init]) => init?.method === 'DELETE').map(([url]) => url)).toEqual([
      '/api/attachments/1',
    ]);
  });

  it('[ no ] and Esc cancel the delete without a request and refocus [ delete ]', async () => {
    const fetch = api();
    await open();
    await fireEvent.click(row('report.pdf').getByRole('button', { name: '[ delete ]' }));
    const no = row('report.pdf').getByRole('button', { name: '[ no ]' });
    expect(no).toHaveFocus();
    await fireEvent.click(no);
    expect(row('report.pdf').getByRole('button', { name: '[ delete ]' })).toHaveFocus();
    await fireEvent.click(row('report.pdf').getByRole('button', { name: '[ delete ]' }));
    const esc = await fireEvent.keyDown(row('report.pdf').getByRole('button', { name: '[ no ]' }), { key: 'Escape' });
    expect(esc).toBe(false);
    expect(row('report.pdf').queryByText('delete report.pdf?')).not.toBeInTheDocument();
    expect(row('report.pdf').getByRole('button', { name: '[ delete ]' })).toHaveFocus();
    expect(fetch.mock.calls.some(([, init]) => init?.method === 'DELETE')).toBe(false);
  });

  it('shows a delete failure', async () => {
    api({ 'DELETE /api/attachments/1': () => Response.json({ error: 'attachment not found' }, { status: 404 }) });
    await open();
    await fireEvent.click(row('report.pdf').getByRole('button', { name: '[ delete ]' }));
    await fireEvent.click(row('report.pdf').getByRole('button', { name: '[ yes ]' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('attachment not found');
    expect(screen.getByText('report.pdf')).toBeInTheDocument();
  });
});
