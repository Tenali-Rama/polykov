import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import Shell from './Shell.svelte';

const props = { onlogout: () => {} };

function mockFetch(impl: (path?: string) => Promise<Response>) {
  const fn = vi.fn(impl);
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

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
