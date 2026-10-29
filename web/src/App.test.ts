import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { userEvent } from '@testing-library/user-event';
import App from './App.svelte';

type Route = (init?: RequestInit) => Response;

/** Stub `fetch` with one handler per `METHOD path`; unknown routes fail the test with a 599. */
function server(routes: Record<string, Route>) {
  const fn = vi.fn(async (path: string, init?: RequestInit) => {
    const route = routes[`${init?.method ?? 'GET'} ${path}`];
    return route ? route(init) : new Response(`no route ${path}`, { status: 599 });
  });
  vi.stubGlobal('fetch', fn);
  return fn;
}

const health: Record<string, Route> = {
  'GET /health': () => Response.json({ status: 'ok', version: '0.1.0' }),
};
const fresh = { ...health, 'GET /api/setup': () => Response.json({ needed: true }) };
const loggedOut = {
  ...health,
  'GET /api/setup': () => Response.json({ needed: false }),
  'GET /api/me': () => Response.json({ error: 'authentication required' }, { status: 401 }),
};
const loggedIn = {
  ...loggedOut,
  'GET /api/me': () => Response.json({ owner: true, session_expires: '2026-11-28T09:15:00Z' }),
};

const TOKEN = 'a'.repeat(64);
const loginOk = () =>
  Response.json({ token: TOKEN }, { headers: { 'Set-Cookie': `polykov_session=${TOKEN}` } });

async function type(label: string, value: string) {
  await fireEvent.input(screen.getByLabelText(label), { target: { value } });
}

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe('routing', () => {
  it('shows setup with the first field focused when no owner exists', async () => {
    const fetch = server(fresh);
    render(App);
    expect(await screen.findByText(/polykov setup/)).toBeInTheDocument();
    expect(screen.getByLabelText('password')).toHaveFocus();
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith('/api/setup', expect.anything());
  });

  it('shows login with the password focused when there is no session', async () => {
    const fetch = server(loggedOut);
    render(App);
    expect(await screen.findByText('polykov login')).toBeInTheDocument();
    expect(screen.getByLabelText('password')).toHaveFocus();
    expect(fetch.mock.calls.map((c) => c[0])).toEqual(['/api/setup', '/api/me']);
  });

  it('shows the shell when the session is valid', async () => {
    server(loggedIn);
    render(App);
    expect(await screen.findByRole('button', { name: 'logout' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'command' })).toBeInTheDocument();
    // Only a fresh login moves focus to the command input.
    expect(document.activeElement).toBe(document.body);
  });

  it('offers a retry when the server cannot be reached', async () => {
    server({});
    render(App);
    expect(await screen.findByRole('alert')).toHaveTextContent('server unreachable');
    server(fresh);
    await fireEvent.click(screen.getByRole('button', { name: '[ retry ]' }));
    expect(await screen.findByText(/polykov setup/)).toBeInTheDocument();
  });
});

describe('setup', () => {
  it('shows a mismatch inline and does not submit', async () => {
    const fetch = server(fresh);
    render(App);
    await screen.findByText(/polykov setup/);
    await type('password', 'correct horse battery');
    await type('confirm', 'correct horse batterx');
    expect(screen.getByText('passwords do not match')).toBeInTheDocument();
    await fireEvent.submit(screen.getByRole('button', { name: /create owner/ }));
    expect(fetch).not.toHaveBeenCalledWith('/api/setup', expect.objectContaining({ method: 'POST' }));
  });

  it('shows too-short inline, counting characters not bytes', async () => {
    server(fresh);
    render(App);
    await screen.findByText(/polykov setup/);
    await type('password', 'ééééééééééé');
    expect(screen.getByText('password must be at least 12 characters')).toBeInTheDocument();
    await type('password', 'éééééééééééé');
    expect(screen.queryByText('password must be at least 12 characters')).not.toBeInTheDocument();
  });

  it('shows both errors when submitted empty', async () => {
    server(fresh);
    render(App);
    await screen.findByText(/polykov setup/);
    await type('confirm', 'x');
    await fireEvent.submit(screen.getByRole('button', { name: /create owner/ }));
    expect(screen.getByText('password must be at least 12 characters')).toBeInTheDocument();
    expect(screen.getByText('passwords do not match')).toBeInTheDocument();
  });

  it('shows the server 409 text', async () => {
    server({
      ...fresh,
      'POST /api/setup': () => Response.json({ error: 'owner already exists' }, { status: 409 }),
    });
    render(App);
    await screen.findByText(/polykov setup/);
    await type('password', 'correct horse battery');
    await type('confirm', 'correct horse battery');
    await fireEvent.submit(screen.getByRole('button', { name: /create owner/ }));
    expect(await screen.findByText('owner already exists')).toBeInTheDocument();
    expect(screen.queryByLabelText('password')).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: '[ log in ]' }));
    expect(await screen.findByText('polykov login')).toBeInTheDocument();
    expect(screen.getByLabelText('password')).toHaveFocus();
    expect(screen.queryByText('owner created — log in')).not.toBeInTheDocument();
  });

  it('posts the password and moves on to login', async () => {
    const fetch = server({ ...fresh, 'POST /api/setup': () => new Response(null, { status: 201 }) });
    render(App);
    await screen.findByText(/polykov setup/);
    await type('password', 'correct horse battery');
    await type('confirm', 'correct horse battery');
    await fireEvent.submit(screen.getByRole('button', { name: /create owner/ }));
    expect(await screen.findByText('polykov login')).toBeInTheDocument();
    expect(screen.getByText('owner created — log in')).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith(
      '/api/setup',
      expect.objectContaining({ method: 'POST', body: '{"password":"correct horse battery"}' }),
    );
  });
});

describe('login', () => {
  it('shows wrong password on 401 and stays on login', async () => {
    server({ ...loggedOut, 'POST /api/login': () => new Response(null, { status: 401 }) });
    render(App);
    await screen.findByText('polykov login');
    await type('password', 'not the password');
    await fireEvent.submit(screen.getByRole('button', { name: /log in/ }));
    expect(await screen.findByText('wrong password')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'logout' })).not.toBeInTheDocument();
  });

  it('shows the shell on success without storing the token', async () => {
    server({ ...loggedOut, 'POST /api/login': loginOk });
    render(App);
    await screen.findByText('polykov login');
    await type('password', 'correct horse battery');
    await fireEvent.submit(screen.getByRole('button', { name: /log in/ }));
    expect(await screen.findByRole('button', { name: 'logout' })).toBeInTheDocument();
    expect(document.activeElement).toBe(screen.getByRole('textbox', { name: 'command' }));
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.length).toBe(0);
  });
});

describe('logout', () => {
  it('posts /api/logout and returns to login', async () => {
    const fetch = server({ ...loggedIn, 'POST /api/logout': () => new Response(null, { status: 204 }) });
    render(App);
    const button = await screen.findByRole('button', { name: 'logout' });
    await userEvent.tab();
    expect(button).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    expect(await screen.findByText('polykov login')).toBeInTheDocument();
    expect(screen.queryByText('owner created — log in')).not.toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith('/api/logout', expect.objectContaining({ method: 'POST' }));
  });
});
