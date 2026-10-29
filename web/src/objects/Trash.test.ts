import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import Trash from './Trash.svelte';
import { makeObj, stubApi, type Route } from './testing';
import type { Trashed } from '../lib/trash';

function trashed(id: number, title: string): Trashed {
  return { ...makeObj(id, { title }), deleted: '2026-10-29T10:00:00Z', purge_at: '2026-11-28T10:00:00Z' };
}

const items = [trashed(2, 'Second'), trashed(1, 'First')];

function api(more: Record<string, Route> = {}) {
  return stubApi({ 'GET /api/trash': () => Response.json({ objects: items }), ...more });
}

function row(title: string) {
  const li = screen.getByText(title).closest('li');
  if (!li) throw new Error(`no row ${title}`);
  return within(li);
}

/** Calls other than the initial `GET /api/trash`. */
function writes(fetch: ReturnType<typeof api>) {
  return fetch.mock.calls.filter(([, init]) => init?.method !== 'GET').map(([url, init]) => `${init?.method} ${url}`);
}

beforeEach(() => {
  location.hash = '#/trash';
});
afterEach(() => vi.unstubAllGlobals());

describe('trash', () => {
  it('lists deleted objects with deleted and purge dates', async () => {
    api();
    render(Trash);
    expect(await screen.findByText('Second')).toBeInTheDocument();
    expect(row('Second').getByText('note · deleted 2026-10-29 · purges 2026-11-28')).toBeInTheDocument();
    expect(screen.getAllByRole('listitem')).toHaveLength(2);
  });

  it('shows trash is empty and no empty button', async () => {
    stubApi({ 'GET /api/trash': () => Response.json({ objects: [] }) });
    render(Trash);
    expect(await screen.findByText('trash is empty')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '[ empty trash ]' })).not.toBeInTheDocument();
  });

  it('shows a load failure', async () => {
    stubApi({ 'GET /api/trash': () => new Response('', { status: 500 }) });
    render(Trash);
    expect(await screen.findByRole('alert')).toHaveTextContent('error: /api/trash: HTTP 500');
  });

  it('restore POSTs and removes the row', async () => {
    const fetch = api({ 'POST /api/trash/1/restore': () => Response.json(makeObj(1, { title: 'First' })) });
    render(Trash);
    await screen.findByText('First');
    await fireEvent.click(row('First').getByRole('button', { name: '[ restore ]' }));
    expect(await screen.findByText('restored First')).toBeInTheDocument();
    expect(screen.getAllByRole('listitem')).toHaveLength(1);
    expect(writes(fetch)).toEqual(['POST /api/trash/1/restore']);
  });

  it('delete forever asks first; no cancels without a request', async () => {
    const fetch = api({ 'DELETE /api/trash/2': () => new Response(null, { status: 204 }) });
    render(Trash);
    await screen.findByText('Second');
    await fireEvent.click(row('Second').getByRole('button', { name: '[ delete forever ]' }));
    expect(row('Second').getByText('delete forever?')).toBeInTheDocument();
    const no = row('Second').getByRole('button', { name: '[ no ]' });
    expect(no).toHaveFocus();
    await fireEvent.click(no);
    expect(row('Second').getByRole('button', { name: '[ delete forever ]' })).toBeInTheDocument();
    expect(writes(fetch)).toEqual([]);

    await fireEvent.click(row('Second').getByRole('button', { name: '[ delete forever ]' }));
    await fireEvent.click(row('Second').getByRole('button', { name: '[ yes ]' }));
    expect(await screen.findByText('deleted Second forever')).toBeInTheDocument();
    expect(screen.queryByText('Second')).not.toBeInTheDocument();
    expect(writes(fetch)).toEqual(['DELETE /api/trash/2']);
  });

  it('Esc cancels a pending confirm, then goes back to the list', async () => {
    api();
    render(Trash);
    await screen.findByText('Second');
    await fireEvent.click(row('Second').getByRole('button', { name: '[ delete forever ]' }));
    await fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(row('Second').getByRole('button', { name: '[ delete forever ]' })).toBeInTheDocument();
    expect(location.hash).toBe('#/trash');
    await fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(location.hash).toBe('#/');
  });

  it('empty trash asks first, then DELETEs the trash', async () => {
    const fetch = api({ 'DELETE /api/trash': () => Response.json({ purged: 2 }) });
    render(Trash);
    await screen.findByText('Second');
    await fireEvent.click(screen.getByRole('button', { name: '[ empty trash ]' }));
    expect(screen.getByText('delete all 2 forever?')).toBeInTheDocument();
    expect(writes(fetch)).toEqual([]);
    await fireEvent.click(screen.getByRole('button', { name: '[ yes ]' }));
    expect(await screen.findByText('deleted 2 forever')).toBeInTheDocument();
    expect(screen.getByText('trash is empty')).toBeInTheDocument();
    expect(writes(fetch)).toEqual(['DELETE /api/trash']);
  });

  it('a failed purge shows the server text and keeps the row', async () => {
    api({ 'DELETE /api/trash/1': () => Response.json({ error: 'not in trash' }, { status: 404 }) });
    render(Trash);
    await screen.findByText('First');
    await fireEvent.click(row('First').getByRole('button', { name: '[ delete forever ]' }));
    await fireEvent.click(row('First').getByRole('button', { name: '[ yes ]' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('error: not in trash');
    await waitFor(() => expect(row('First').getByRole('button', { name: '[ restore ]' })).toBeInTheDocument());
  });
});
