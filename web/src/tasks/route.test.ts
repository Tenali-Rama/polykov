import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { makeObj, stubApi } from '../objects/testing';
import type { Obj } from '../lib/objects';
import Nav from './Nav.svelte';
import { areaHref, parseTaskHash, projectHref, taskListHref } from './route';

describe('parseTaskHash', () => {
  it('parses the five lists, projects and areas', () => {
    for (const list of ['inbox', 'today', 'upcoming', 'someday', 'anytime'] as const) {
      expect(parseTaskHash(`#/${list}`)).toEqual({ view: 'list', list });
      expect(taskListHref(list)).toBe(`#/${list}`);
    }
    expect(parseTaskHash('#/p/12')).toEqual({ view: 'project', id: 12 });
    expect(parseTaskHash('#/a/3')).toEqual({ view: 'area', id: 3 });
    expect(projectHref(12)).toBe('#/p/12');
    expect(areaHref(3)).toBe('#/a/3');
  });

  it('leaves object routes and junk to the object router', () => {
    for (const hash of ['', '#/', '#/trash', '#/o/5', '#/all', '#/p/0', '#/p/x', '#/a/1/2', '#/todayx']) {
      expect(parseTaskHash(hash)).toBeNull();
    }
  });
});

describe('Nav', () => {
  function api(fail = false, pinned: Obj[] = []) {
    return stubApi({
      'GET /api/projects': () =>
        fail ? new Response('', { status: 500 }) : Response.json({ projects: [{ ...makeObj(7, { title: 'Launch' }) }] }),
      'GET /api/objects': (url) =>
        Response.json(
          url.includes('pinned=true')
            ? { objects: pinned, total: pinned.length }
            : { objects: [makeObj(9, { type: 'area', title: 'Work' })], total: 1 },
        ),
    });
  }

  beforeEach(() => {
    location.hash = '#/today';
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    location.hash = '';
  });

  const menu = () => within(screen.getByRole('list', { hidden: true }));

  it('lists the five lists, objects, trash, then projects and areas, marking the current one', async () => {
    const fetch = api();
    render(Nav);
    await waitFor(() => expect(menu().getAllByRole('link', { hidden: true })).toHaveLength(9));
    const links = menu().getAllByRole('link', { hidden: true });
    expect(links.map((a) => [a.textContent, a.getAttribute('href')])).toEqual([
      ['inbox', '#/inbox'],
      ['today', '#/today'],
      ['upcoming', '#/upcoming'],
      ['someday', '#/someday'],
      ['anytime', '#/anytime'],
      ['objects', '#/'],
      ['trash', '#/trash'],
      ['Launch', '#/p/7'],
      ['Work', '#/a/9'],
    ]);
    expect(links[1]).toHaveAttribute('aria-current', 'page');
    expect(links.filter((a) => a.hasAttribute('aria-current'))).toHaveLength(1);
    expect(fetch.mock.calls.map(([url]) => url)).toEqual([
      '/api/objects?pinned=true&limit=100',
      '/api/projects',
      '/api/objects?type=area&limit=1000',
    ]);

    location.hash = '#/p/7';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    await waitFor(() => expect(links[7]).toHaveAttribute('aria-current', 'page'));
    expect(links[1]).not.toHaveAttribute('aria-current');
  });

  it('marks objects for an empty hash', async () => {
    api();
    location.hash = '';
    render(Nav);
    expect(await screen.findByRole('link', { name: 'objects', hidden: true })).toHaveAttribute('aria-current', 'page');
  });

  it('keeps the fixed links and shows the cause when projects fail to load', async () => {
    api(true);
    render(Nav);
    expect(await screen.findByText('nav: /api/projects: HTTP 500')).toBeInTheDocument();
    expect(menu().getAllByRole('link', { hidden: true })).toHaveLength(7);
  });

  it('[ lists ] opens the menu on its first link; arrows move; Esc closes and refocuses the button', async () => {
    api();
    render(Nav);
    const toggle = screen.getByRole('button', { name: 'lists' });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    await fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('list', { hidden: true })).toHaveClass('open');
    const links = menu().getAllByRole('link', { hidden: true });
    expect(links[0]).toHaveFocus();
    await fireEvent.keyDown(links[0], { key: 'ArrowDown' });
    expect(links[1]).toHaveFocus();
    await fireEvent.keyDown(links[1], { key: 'ArrowUp' });
    await fireEvent.keyDown(links[0], { key: 'ArrowUp' });
    expect(links[0]).toHaveFocus();

    const outer = vi.fn();
    window.addEventListener('keydown', outer);
    await fireEvent.keyDown(links[0], { key: 'Escape' });
    window.removeEventListener('keydown', outer);
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(toggle).toHaveFocus();
    expect(outer).not.toHaveBeenCalled();
  });

  it('puts favourites first in the menu, inside the arrow-key order', async () => {
    api(false, [makeObj(3, { title: 'Ideas', pinned: true })]);
    render(Nav);
    const fav = await screen.findByRole('link', { name: 'note Ideas', hidden: true });
    await fireEvent.click(screen.getByRole('button', { name: 'lists' }));
    expect(fav).toHaveFocus();
    await fireEvent.keyDown(fav, { key: 'ArrowDown' });
    expect(screen.getByRole('link', { name: 'inbox', hidden: true })).toHaveFocus();
  });

  it('closes the menu when the route changes', async () => {
    api();
    render(Nav);
    const toggle = screen.getByRole('button', { name: 'lists' });
    await fireEvent.click(toggle);
    location.hash = '#/inbox';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    await waitFor(() => expect(toggle).toHaveAttribute('aria-expanded', 'false'));
  });
});
