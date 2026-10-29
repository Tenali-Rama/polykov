import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import Shell from './Shell.svelte';
import { makeObj, stubApi } from './objects/testing';

const props = { onlogout: () => {} };

/** `impl` answers everything except the object list body calls, which return empty results. */
function mockFetch(impl: (path?: string) => Promise<Response>) {
  const fn = vi.fn(async (path?: string) => {
    if (path?.startsWith('/api/objects')) return Response.json({ objects: [], total: 0 });
    if (path === '/api/tags') return Response.json({ tags: [] });
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
  };

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

  it('the list links to #/trash, which shows the trash view', async () => {
    stubApi({ ...routes, 'GET /api/trash': () => Response.json({ objects: [] }) });
    location.hash = '#/';
    render(Shell, { props });
    expect(await screen.findByRole('link', { name: 'trash' })).toHaveAttribute('href', '#/trash');
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
