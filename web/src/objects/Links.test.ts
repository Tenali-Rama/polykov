import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import Links from './Links.svelte';
import { makeObj, stubApi, urls, type Route } from './testing';
import type { Links as LinkSet } from '../lib/links';

const empty: LinkSet = { links: [], backlinks: [] };
const s = (id: number, title: string, type = 'note') => ({ id, type, title });
const noContent = () => new Response(null, { status: 204 });

/** Serves the links of 7 from `state` (a later route may change it) plus `more`. */
function api(state: { links: LinkSet }, more: Record<string, Route> = {}) {
  return stubApi({ 'GET /api/objects/7/links': () => Response.json(state.links), ...more });
}

async function open() {
  render(Links, { props: { id: 7 } });
  return screen.findByRole('button', { name: '[ link ]' });
}

function section(name: 'links' | 'backlinks') {
  // The backlinks list is the last list; links the first.
  const lists = within(screen.getByRole('region', { name: 'links' })).getAllByRole('list');
  return name === 'links' ? lists[0] : lists[lists.length - 1];
}

beforeEach(() => {
  location.hash = '#/o/7';
});
afterEach(() => vi.unstubAllGlobals());

describe('links panel', () => {
  it('shows links and backlinks, each opening its object', async () => {
    api({ links: { links: [s(2, 'Write copy', 'task')], backlinks: [s(3, 'Ideas')] } });
    await open();
    expect(within(section('links')).getByRole('link', { name: 'Write copy' })).toHaveAttribute('href', '#/o/2');
    expect(within(section('links')).getByText('task')).toBeInTheDocument();
    expect(within(section('backlinks')).getByRole('link', { name: 'Ideas' })).toHaveAttribute('href', '#/o/3');
    expect(screen.getByRole('button', { name: 'unlink Write copy' })).toHaveTextContent('[ unlink ]');
    expect(screen.queryByRole('button', { name: 'unlink Ideas' })).not.toBeInTheDocument();
  });

  it('shows empty sides', async () => {
    api({ links: empty });
    await open();
    expect(screen.getByText('no links')).toBeInTheDocument();
    expect(screen.getByText('no backlinks')).toBeInTheDocument();
  });

  it('clicking a backlink navigates to it', async () => {
    api({ links: { links: [], backlinks: [s(3, 'Ideas')] } });
    await open();
    await fireEvent.click(screen.getByRole('link', { name: 'Ideas' }));
    await vi.waitFor(() => expect(location.hash).toBe('#/o/3'));
  });

  it('links by id with one PUT and shows the new link', async () => {
    const state: { links: LinkSet } = { links: empty };
    const fetch = api(state, {
      'PUT /api/objects/7/links/2': () => {
        state.links = { links: [s(2, 'Write copy')], backlinks: [] };
        return noContent();
      },
    });
    await fireEvent.click(await open());
    const input = screen.getByRole('textbox', { name: 'link to id or title' });
    expect(input).toHaveFocus();
    await fireEvent.input(input, { target: { value: ' 2 ' } });
    await fireEvent.submit(input);
    expect(await screen.findByRole('link', { name: 'Write copy' })).toBeInTheDocument();
    expect(urls(fetch, '/api/objects/7/links/')).toEqual(['/api/objects/7/links/2']);
    expect(urls(fetch, '/api/objects?')).toEqual([]);
    expect(screen.queryByRole('textbox', { name: 'link to id or title' })).not.toBeInTheDocument();
  });

  it('links by title: searches, lists matches without self, and links the picked one', async () => {
    const state: { links: LinkSet } = { links: empty };
    const objects = [makeObj(7, { title: 'Groceries list' }), makeObj(4, { title: 'Groceries' }), makeObj(5, { title: 'Taxes' })];
    const fetch = api(state, {
      'GET /api/objects': () => Response.json({ objects, total: 3 }),
      'PUT /api/objects/7/links/4': () => {
        state.links = { links: [s(4, 'Groceries')], backlinks: [] };
        return noContent();
      },
    });
    await fireEvent.click(await open());
    const input = screen.getByRole('textbox', { name: 'link to id or title' });
    await fireEvent.input(input, { target: { value: 'grocer' } });
    await fireEvent.submit(input);
    const matches = await screen.findByRole('list', { name: 'matches' });
    expect(within(matches).getAllByRole('button').map((b) => b.textContent)).toEqual(['Groceries']);
    await fireEvent.click(within(matches).getByRole('button', { name: 'Groceries' }));
    expect(await screen.findByRole('link', { name: 'Groceries' })).toHaveAttribute('href', '#/o/4');
    expect(urls(fetch, '/api/objects/7/links/')).toEqual(['/api/objects/7/links/4']);
  });

  it('shows no matches for a title nothing has', async () => {
    api({ links: empty }, { 'GET /api/objects': () => Response.json({ objects: [makeObj(5, { title: 'Taxes' })], total: 1 }) });
    await fireEvent.click(await open());
    const input = screen.getByRole('textbox', { name: 'link to id or title' });
    await fireEvent.input(input, { target: { value: 'zebra' } });
    await fireEvent.submit(input);
    expect(await screen.findByText('no matches')).toBeInTheDocument();
  });

  it('shows the server text for a bad link and keeps the prompt', async () => {
    api(
      { links: empty },
      { 'PUT /api/objects/7/links/7': () => Response.json({ error: 'cannot link an object to itself' }, { status: 400 }) },
    );
    await fireEvent.click(await open());
    const input = screen.getByRole('textbox', { name: 'link to id or title' });
    await fireEvent.input(input, { target: { value: '7' } });
    await fireEvent.submit(input);
    expect(await screen.findByRole('alert')).toHaveTextContent('cannot link an object to itself');
    expect(screen.getByRole('textbox', { name: 'link to id or title' })).toBeInTheDocument();
  });

  it('Esc closes the prompt without leaving the page', async () => {
    const fetch = api({ links: empty });
    await fireEvent.click(await open());
    const input = screen.getByRole('textbox', { name: 'link to id or title' });
    const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    input.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(true);
    expect(await screen.findByRole('button', { name: '[ link ]' })).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('unlink sends DELETE and drops the link', async () => {
    const state: { links: LinkSet } = { links: { links: [s(2, 'Write copy')], backlinks: [] } };
    const fetch = api(state, {
      'DELETE /api/objects/7/links/2': () => {
        state.links = empty;
        return noContent();
      },
    });
    await open();
    await fireEvent.click(screen.getByRole('button', { name: 'unlink Write copy' }));
    expect(await screen.findByText('no links')).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith('/api/objects/7/links/2', expect.objectContaining({ method: 'DELETE' }));
  });

  it('shows a load failure', async () => {
    stubApi({ 'GET /api/objects/7/links': () => new Response('', { status: 500 }) });
    render(Links, { props: { id: 7 } });
    expect(await screen.findByRole('alert')).toHaveTextContent('/api/objects/7/links: HTTP 500');
  });
});
