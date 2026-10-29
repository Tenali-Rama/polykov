import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import List from './List.svelte';
import { makeObj, stubApi, urls, type Route } from './testing';
import type { Obj } from '../lib/objects';

const tags = [
  { tag: 'work', count: 2 },
  { tag: 'work/clienta', count: 1 },
];

function api(objects: Obj[], total = objects.length, more: Record<string, Route> = {}) {
  return stubApi({
    'GET /api/objects': () => Response.json({ objects, total }),
    'GET /api/tags': () => Response.json({ tags }),
    ...more,
  });
}

function rows() {
  return within(screen.getByRole('list', { name: 'objects' })).queryAllByRole('link');
}

beforeEach(() => {
  location.hash = '';
});
afterEach(() => vi.unstubAllGlobals());

describe('rows', () => {
  it('shows pin marker, title, type and tags, with the default query', async () => {
    const fetch = api([makeObj(2, { pinned: true, title: 'Plan', type: 'task', tags: ['work/clienta'] }), makeObj(1)]);
    render(List);
    await waitFor(() => expect(rows()).toHaveLength(2));
    expect(rows()[0]).toHaveTextContent('* Plan task #work/clienta');
    expect(rows()[1]).toHaveTextContent('object 1 note');
    expect(rows()[1]).not.toHaveTextContent('*');
    expect(rows()[0]).toHaveAttribute('href', '#/o/2');
    expect(urls(fetch, '/api/objects')).toEqual(['/api/objects?archived=false&limit=50&offset=0']);
  });

  it('shows the empty state', async () => {
    api([]);
    render(List);
    expect(await screen.findByText('no objects')).toBeInTheDocument();
  });

  it('shows a load error', async () => {
    api([], 0, { 'GET /api/objects': () => new Response('', { status: 500 }) });
    render(List);
    expect(await screen.findByRole('alert')).toHaveTextContent('error: /api/objects?archived=false&limit=50&offset=0: HTTP 500');
    expect(screen.queryByText('no objects')).not.toBeInTheDocument();
  });

  it('pages with [ more ] and hides it once everything is loaded', async () => {
    let call = 0;
    const fetch = stubApi({
      'GET /api/objects': () =>
        Response.json(call++ === 0 ? { objects: [makeObj(3), makeObj(2)], total: 3 } : { objects: [makeObj(1)], total: 3 }),
      'GET /api/tags': () => Response.json({ tags }),
    });
    render(List);
    await fireEvent.click(await screen.findByRole('button', { name: '[ more ]' }));
    await waitFor(() => expect(rows()).toHaveLength(3));
    expect(urls(fetch, '/api/objects')[1]).toBe('/api/objects?archived=false&limit=50&offset=2');
    expect(screen.queryByRole('button', { name: '[ more ]' })).not.toBeInTheDocument();
  });
});

describe('filters', () => {
  async function lastQuery(fetch: ReturnType<typeof api>) {
    await waitFor(() => expect(urls(fetch, '/api/objects').length).toBeGreaterThan(1));
    return urls(fetch, '/api/objects').at(-1);
  }

  it('type filter', async () => {
    const fetch = api([]);
    render(List);
    await fireEvent.change(screen.getByLabelText('type filter'), { target: { value: 'task' } });
    expect(await lastQuery(fetch)).toBe('/api/objects?type=task&archived=false&limit=50&offset=0');
  });

  it('pinned only', async () => {
    const fetch = api([]);
    render(List);
    await fireEvent.click(screen.getByLabelText('pinned only'));
    expect(await lastQuery(fetch)).toBe('/api/objects?pinned=true&archived=false&limit=50&offset=0');
  });

  it('archived', async () => {
    const fetch = api([]);
    render(List);
    await fireEvent.change(screen.getByLabelText('archived filter'), { target: { value: 'any' } });
    expect(await lastQuery(fetch)).toBe('/api/objects?archived=any&limit=50&offset=0');
  });

  it('tag folders are indented by depth with counts, and a click filters by tag', async () => {
    const fetch = api([]);
    render(List);
    const nav = screen.getByRole('navigation', { name: 'tag folders' });
    const client = await within(nav).findByRole('button', { name: 'clienta/1' });
    const work = within(nav).getByRole('button', { name: 'work/2' });
    expect(work.style.paddingLeft).toBe('0.5ch');
    expect(client.style.paddingLeft).toBe('2.5ch');
    await fireEvent.click(client);
    expect(await lastQuery(fetch)).toBe('/api/objects?tag=work%2Fclienta&archived=false&limit=50&offset=0');
    expect(client).toHaveAttribute('aria-pressed', 'true');
  });
});

describe('create', () => {
  it('posts a note by default and puts the new object on top', async () => {
    const fetch = api([makeObj(1)], 1, {
      'POST /api/objects': () => Response.json(makeObj(5, { title: 'Fresh' }), { status: 201 }),
    });
    render(List);
    await waitFor(() => expect(rows()).toHaveLength(1));
    const title = screen.getByLabelText('new object title');
    await fireEvent.input(title, { target: { value: 'Fresh' } });
    await fireEvent.submit(title.closest('form')!);
    await waitFor(() => expect(rows()).toHaveLength(2));
    expect(rows()[0]).toHaveTextContent('Fresh');
    expect(title).toHaveValue('');
    const post = fetch.mock.calls.find(([, init]) => init?.method === 'POST');
    expect(post?.[1]?.body).toBe('{"type":"note","title":"Fresh"}');
  });

  it('shows the server 400 text inline and keeps the title', async () => {
    api([], 0, {
      'POST /api/objects': () => Response.json({ error: 'title must be 1 to 1000 characters' }, { status: 400 }),
    });
    render(List);
    const title = screen.getByLabelText('new object title');
    await fireEvent.input(title, { target: { value: '   ' } });
    await fireEvent.submit(title.closest('form')!);
    expect(await screen.findByRole('alert')).toHaveTextContent('title must be 1 to 1000 characters');
    expect(title).toHaveValue('   ');
  });
});

describe('keyboard', () => {
  it('j/k move the selection and Enter opens it', async () => {
    api([makeObj(3), makeObj(2), makeObj(1)]);
    render(List);
    await waitFor(() => expect(rows()).toHaveLength(3));
    expect(rows()[0]).toHaveAttribute('aria-current', 'true');
    await fireEvent.keyDown(document.body, { key: 'j' });
    await fireEvent.keyDown(document.activeElement!, { key: 'j' });
    await fireEvent.keyDown(document.activeElement!, { key: 'j' });
    expect(rows()[2]).toHaveAttribute('aria-current', 'true');
    expect(rows()[2]).toHaveFocus();
    await fireEvent.keyDown(document.activeElement!, { key: 'k' });
    expect(rows()[1]).toHaveAttribute('aria-current', 'true');
    await fireEvent.keyDown(document.activeElement!, { key: 'Enter' });
    expect(location.hash).toBe('#/o/2');
  });

  it('j/k/Enter work from a focused tag folder; other buttons keep the keys', async () => {
    api([makeObj(3), makeObj(2), makeObj(1)]);
    render(List);
    await waitFor(() => expect(rows()).toHaveLength(3));
    const folder = screen.getByRole('button', { name: 'all' });
    folder.focus();
    await fireEvent.keyDown(folder, { key: 'j' });
    expect(rows()[1]).toHaveAttribute('aria-current', 'true');
    expect(rows()[1]).toHaveFocus();

    folder.focus();
    const enter = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true });
    folder.dispatchEvent(enter);
    expect(enter.defaultPrevented).toBe(true);
    expect(location.hash).toBe('#/o/2');

    location.hash = '';
    await fireEvent.keyDown(screen.getByRole('button', { name: '[ add ]' }), { key: 'j' });
    await fireEvent.keyDown(screen.getByRole('button', { name: '[ add ]' }), { key: 'Enter' });
    expect(rows()[1]).toHaveAttribute('aria-current', 'true');
    expect(location.hash).toBe('');
  });

  it('ignores j while typing a title', async () => {
    api([makeObj(2), makeObj(1)]);
    render(List);
    await waitFor(() => expect(rows()).toHaveLength(2));
    await fireEvent.keyDown(screen.getByLabelText('new object title'), { key: 'j' });
    expect(rows()[0]).toHaveAttribute('aria-current', 'true');
  });
});
