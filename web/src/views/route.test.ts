import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { makeObj, stubApi } from '../objects/testing';
import type { View } from '../lib/views';
import { parseViewHash, viewHref, viewsHref } from './route';
import ViewsIndex from './ViewsIndex.svelte';

describe('parseViewHash', () => {
  it('parses the index and one view', () => {
    expect(parseViewHash('#/views')).toEqual({ view: 'index' });
    expect(parseViewHash('#/v/12')).toEqual({ view: 'view', id: 12 });
    expect(viewsHref).toBe('#/views');
    expect(viewHref(12)).toBe('#/v/12');
  });

  it('leaves other routes and junk to the other routers', () => {
    for (const hash of ['', '#/', '#/view', '#/views/', '#/v/', '#/v/0', '#/v/01', '#/v/x', '#/v/1/2', '#/o/3', '#/today']) {
      expect(parseViewHash(hash)).toBeNull();
    }
  });
});

// The index's tests live here: it is the `#/views` route.
describe('ViewsIndex', () => {
  const view = (id: number, title: string, pinned = false): View => ({
    ...makeObj(id, { type: 'view', title, pinned }),
    properties: { query: 'type:task', layout: 'kanban', group: 'none', sort: 'smart' },
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    location.hash = '';
  });

  it('lists the views, pinned marked, each linking to #/v/<id>', async () => {
    stubApi({ 'GET /api/views': () => Response.json({ views: [view(2, 'Work', true), view(3, 'Home')] }) });
    render(ViewsIndex);
    const list = within(await screen.findByRole('list', { name: 'saved views' }));
    expect(list.getAllByRole('link').map((a) => [a.textContent, a.getAttribute('href')])).toEqual([
      ['* Work', '#/v/2'],
      ['Home', '#/v/3'],
    ]);
    expect(list.getAllByText('kanban · type:task')).toHaveLength(2);
  });

  it('says so when there are none, and shows a load failure', async () => {
    stubApi({ 'GET /api/views': () => Response.json({ views: [] }) });
    const { unmount } = render(ViewsIndex);
    expect(await screen.findByText('no views yet')).toBeInTheDocument();
    unmount();
    stubApi({ 'GET /api/views': () => new Response('', { status: 500 }) });
    render(ViewsIndex);
    expect(await screen.findByRole('alert')).toHaveTextContent('error: /api/views: HTTP 500');
    expect(screen.queryByText('no views yet')).toBeNull();
  });

  it('[ new view ] opens the editor; a save goes to the new view; Esc closes it', async () => {
    stubApi({
      'GET /api/views': () => Response.json({ views: [] }),
      'POST /api/views': () => Response.json(view(7, 'New'), { status: 201 }),
    });
    render(ViewsIndex);
    const button = screen.getByRole('button', { name: '[ new view ]' });
    await fireEvent.click(button);
    expect(button).toHaveAttribute('aria-expanded', 'true');
    await fireEvent.keyDown(screen.getByLabelText('query'), { key: 'Escape' });
    expect(screen.queryByRole('form')).toBeNull();
    expect(button).toHaveFocus();
    await fireEvent.click(button);
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    await waitFor(() => expect(location.hash).toBe('#/v/7'));
  });
});
