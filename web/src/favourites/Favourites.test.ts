import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/svelte';
import Favourites, { OBJECTS_CHANGED } from './Favourites.svelte';
import { makeObj, stubApi, urls } from '../objects/testing';
import { TASKS_CHANGED } from '../tasks/TaskList.svelte';
import type { Obj } from '../lib/objects';

function api(...pages: Obj[][]) {
  let call = 0;
  return stubApi({
    'GET /api/objects': () => {
      const objects = pages[Math.min(call++, pages.length - 1)];
      return Response.json({ objects, total: objects.length });
    },
  });
}

const favs = () => within(screen.getByRole('list', { name: 'favourites' })).getAllByRole('link');

const mixed = [
  makeObj(1, { type: 'view', title: 'Work due soon' }),
  makeObj(2, { type: 'project', title: 'Launch' }),
  makeObj(3, { type: 'area', title: 'Home' }),
  makeObj(4, { type: 'note', title: 'Ideas' }),
  makeObj(5, { type: 'task', title: 'Pay rent' }),
];

beforeEach(() => {
  location.hash = '';
});
afterEach(() => {
  vi.unstubAllGlobals();
  location.hash = '';
});

describe('Favourites', () => {
  it('loads pinned objects and links each by type, showing type and title', async () => {
    const fetch = api(mixed);
    render(Favourites);
    await waitFor(() => expect(favs()).toHaveLength(5));
    expect(favs().map((a) => [a.textContent, a.getAttribute('href')])).toEqual([
      ['view Work due soon', '#/v/1'],
      ['project Launch', '#/p/2'],
      ['area Home', '#/a/3'],
      ['note Ideas', '#/o/4'],
      ['task Pay rent', '#/o/5'],
    ]);
    expect(urls(fetch, '/api/objects')).toEqual(['/api/objects?pinned=true&limit=100']);
  });

  it('is hidden with no pinned objects', async () => {
    const fetch = api([]);
    render(Favourites);
    await waitFor(() => expect(fetch).toHaveBeenCalled());
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.queryByText('favourites')).not.toBeInTheDocument();
    expect(screen.queryByRole('listitem')).not.toBeInTheDocument();
  });

  it('shows a load error', async () => {
    stubApi({ 'GET /api/objects': () => new Response('', { status: 500 }) });
    render(Favourites);
    expect(await screen.findByText('favourites: /api/objects?pinned=true&limit=100: HTTP 500')).toBeInTheDocument();
  });

  it.each([OBJECTS_CHANGED, TASKS_CHANGED])('reloads on %s', async (event) => {
    const fetch = api([makeObj(4, { title: 'Ideas' })], []);
    render(Favourites);
    await waitFor(() => expect(favs()).toHaveLength(1));
    window.dispatchEvent(new Event(event));
    await waitFor(() => expect(screen.queryByRole('list', { name: 'favourites' })).not.toBeInTheDocument());
    expect(urls(fetch, '/api/objects')).toHaveLength(2);
  });

  it('marks the current route and follows hashchange with a reload', async () => {
    location.hash = '#/p/2';
    const fetch = api(mixed);
    render(Favourites);
    await waitFor(() => expect(favs()).toHaveLength(5));
    expect(screen.getByRole('link', { name: 'project Launch' })).toHaveAttribute('aria-current', 'page');
    expect(favs().filter((a) => a.hasAttribute('aria-current'))).toHaveLength(1);

    location.hash = '#/v/1'; // jsdom fires hashchange
    await waitFor(() =>
      expect(screen.getByRole('link', { name: 'view Work due soon' })).toHaveAttribute('aria-current', 'page'),
    );
    expect(screen.getByRole('link', { name: 'project Launch' })).not.toHaveAttribute('aria-current');
    await waitFor(() => expect(urls(fetch, '/api/objects')).toHaveLength(2));
  });
});
