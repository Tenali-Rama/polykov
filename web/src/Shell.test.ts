import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { userEvent } from '@testing-library/user-event';
import Shell from './Shell.svelte';
import { makeObj, stubApi } from './objects/testing';

const props = { onlogout: () => {} };

/** `impl` answers everything except the object list body and nav calls, which return empty results. */
function mockFetch(impl: (path?: string) => Promise<Response>) {
  const fn = vi.fn(async (path?: string) => {
    if (path?.startsWith('/api/objects')) return Response.json({ objects: [], total: 0 });
    if (path === '/api/tags') return Response.json({ tags: [] });
    if (path === '/api/projects') return Response.json({ projects: [] });
    return impl(path);
  });
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => {
  vi.unstubAllGlobals();
  location.hash = '';
});

describe('health status', () => {
  it('shows the server version when /health is ok', async () => {
    const fetch = mockFetch(async () => Response.json({ status: 'ok', version: '0.1.0' }));
    render(Shell, { props });
    const status = await screen.findByText('server ok 0.1.0');
    expect(status).toHaveClass('ok');
    expect(fetch).toHaveBeenCalledWith('/health', expect.anything());
  });

  it('shows unreachable when fetch rejects', async () => {
    mockFetch(async () => {
      throw new TypeError('Failed to fetch');
    });
    render(Shell, { props });
    expect(await screen.findByText('server unreachable')).toHaveClass('err');
  });

  it('shows unreachable on an error status', async () => {
    mockFetch(async () => Response.json({ status: 'error', version: '0.1.0' }, { status: 503 }));
    render(Shell, { props });
    expect(await screen.findByText('server unreachable')).toBeInTheDocument();
  });
});

describe('prompt', () => {
  it('shows the block-cursor placeholder only while the input is empty', async () => {
    mockFetch(async () => Response.json({ status: 'ok', version: '0.1.0' }));
    render(Shell, { props });
    const input = screen.getByRole('textbox', { name: 'command' });
    expect(screen.getByText('type a command')).toBeInTheDocument();
    await fireEvent.input(input, { target: { value: 'today' } });
    expect(screen.queryByText('type a command')).not.toBeInTheDocument();
  });
});

describe('logout', () => {
  it('stays in the shell and shows the cause when logout fails', async () => {
    mockFetch(async (path?: string) =>
      path === '/api/logout'
        ? new Response('', { status: 500 })
        : Response.json({ status: 'ok', version: '0.1.0' }),
    );
    const onlogout = vi.fn();
    render(Shell, { props: { onlogout } });
    await fireEvent.click(screen.getByRole('button', { name: 'logout' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('logout failed: /api/logout: HTTP 500');
    expect(onlogout).not.toHaveBeenCalled();
  });
});

describe('hash routes', () => {
  const routes = {
    'GET /health': () => Response.json({ status: 'ok', version: '0.1.0' }),
    'GET /api/tags': () => Response.json({ tags: [] }),
    'GET /api/objects': () => Response.json({ objects: [makeObj(4, { title: 'Listed' })], total: 1 }),
    'GET /api/objects/4': () => Response.json(makeObj(4, { title: 'Opened' })),
    'GET /api/projects': () => Response.json({ projects: [] }),
    'GET /api/lists/today': () => Response.json({ tasks: [], total: 0 }),
  };

  it('shows a task list on #/today, checked before the object routes, and the list again on #/', async () => {
    stubApi(routes);
    location.hash = '#/today';
    render(Shell, { props });
    expect(await screen.findByText('nothing in today')).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'lists' })).toBeInTheDocument();
    location.hash = '#/';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(await screen.findByRole('link', { name: /Listed/ })).toBeInTheDocument();
    expect(screen.queryByText('nothing in today')).not.toBeInTheDocument();
  });

  it('shows the list on an unknown hash', async () => {
    stubApi(routes);
    location.hash = '#/nope';
    render(Shell, { props });
    expect(await screen.findByRole('link', { name: /Listed/ })).toBeInTheDocument();
  });

  it('shows the detail on #/o/<id> and follows hash changes back and forth', async () => {
    stubApi(routes);
    location.hash = '#/o/4';
    render(Shell, { props });
    expect(await screen.findByRole('heading', { name: 'Opened' })).toBeInTheDocument();

    location.hash = '#/';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(await screen.findByRole('link', { name: /Listed/ })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Opened' })).not.toBeInTheDocument();

    location.hash = '#/o/4';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(await screen.findByRole('heading', { name: 'Opened' })).toBeInTheDocument();
  });

  it('Esc in the command input keeps the detail; Esc on the page goes back', async () => {
    stubApi(routes);
    location.hash = '#/o/4';
    render(Shell, { props });
    await screen.findByRole('heading', { name: 'Opened' });
    await userEvent.type(screen.getByRole('textbox', { name: 'command' }), 'today{Escape}');
    expect(location.hash).toBe('#/o/4');
    await fireEvent.keyDown(screen.getByRole('button', { name: '[ pin ]' }), { key: 'Escape' });
    expect(location.hash).toBe('#/o/4');
    await fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(location.hash).toBe('#/');
  });

  it('the list links to #/trash, which shows the trash view', async () => {
    stubApi({ ...routes, 'GET /api/trash': () => Response.json({ objects: [] }) });
    location.hash = '#/';
    render(Shell, { props });
    // The nav has a trash link too.
    await waitFor(() => expect(screen.getAllByRole('link', { name: 'trash' })).toHaveLength(2));
    for (const a of screen.getAllByRole('link', { name: 'trash' })) expect(a).toHaveAttribute('href', '#/trash');
    location.hash = '#/trash';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(await screen.findByText('trash is empty')).toBeInTheDocument();
  });

  it('delete on the detail returns to the list with moved to trash, cleared on leaving it', async () => {
    stubApi({ ...routes, 'DELETE /api/objects/4': () => new Response(null, { status: 204 }) });
    location.hash = '#/o/4';
    render(Shell, { props });
    await fireEvent.click(await screen.findByRole('button', { name: '[ delete ]' }));
    expect(await screen.findByText('moved to trash')).toBeInTheDocument();
    expect(location.hash).toBe('#/');
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(await screen.findByRole('link', { name: /Listed/ })).toBeInTheDocument();
    expect(screen.getByText('moved to trash')).toBeInTheDocument();

    location.hash = '#/o/4';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    await screen.findByRole('heading', { name: 'Opened' });
    location.hash = '#/';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    await screen.findByRole('link', { name: /Listed/ });
    expect(screen.queryByText('moved to trash')).not.toBeInTheDocument();
  });
});
