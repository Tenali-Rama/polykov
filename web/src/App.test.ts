import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import App from './App.svelte';

function mockFetch(impl: () => Promise<Response>) {
  const fn = vi.fn(impl);
  vi.stubGlobal('fetch', fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe('health status', () => {
  it('shows the server version when /health is ok', async () => {
    const fetch = mockFetch(async () => Response.json({ status: 'ok', version: '0.1.0' }));
    render(App);
    const status = await screen.findByText('server ok 0.1.0');
    expect(status).toHaveClass('ok');
    expect(fetch).toHaveBeenCalledWith('/health', expect.anything());
  });

  it('shows unreachable when fetch rejects', async () => {
    mockFetch(async () => {
      throw new TypeError('Failed to fetch');
    });
    render(App);
    expect(await screen.findByText('server unreachable')).toHaveClass('err');
  });

  it('shows unreachable on an error status', async () => {
    mockFetch(async () => Response.json({ status: 'error', version: '0.1.0' }, { status: 503 }));
    render(App);
    expect(await screen.findByText('server unreachable')).toBeInTheDocument();
  });
});

describe('prompt', () => {
  it('shows the block-cursor placeholder only while the input is empty', async () => {
    mockFetch(async () => Response.json({ status: 'ok', version: '0.1.0' }));
    render(App);
    const input = screen.getByRole('textbox', { name: 'command' });
    expect(screen.getByText('type a command')).toBeInTheDocument();
    await fireEvent.input(input, { target: { value: 'today' } });
    expect(screen.queryByText('type a command')).not.toBeInTheDocument();
  });
});
