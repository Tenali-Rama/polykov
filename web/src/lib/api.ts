// Thin client for the Polykov server API. Paths are same-origin; `vite dev` proxies them.

export type Health = { status: string; version: string };

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

/** Fetch `path` and parse a JSON body; throws `ApiError` on a non-2xx status. */
export async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(path, { headers: { Accept: 'application/json' } });
  if (!res.ok) throw new ApiError(res.status, `${path}: HTTP ${res.status}`);
  return (await res.json()) as T;
}

/** Server version if `/health` reports ok, otherwise `null` (unreachable, error status, bad body). */
export async function serverVersion(): Promise<string | null> {
  try {
    const h = await getJson<Health>('/health');
    return h.status === 'ok' && typeof h.version === 'string' ? h.version : null;
  } catch {
    return null;
  }
}

/** `POST` `body` as JSON (or no body); throws `ApiError` with the server's `error` text on non-2xx. */
export async function post(path: string, body?: unknown): Promise<Response> {
  const headers: Record<string, string> = { Accept: 'application/json' };
  const init: RequestInit = { method: 'POST', headers };
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
    init.body = JSON.stringify(body);
  }
  const res = await fetch(path, init);
  if (!res.ok) throw new ApiError(res.status, await errorText(res, path));
  return res;
}

async function errorText(res: Response, path: string): Promise<string> {
  try {
    const body = (await res.json()) as { error?: unknown };
    if (typeof body.error === 'string' && body.error !== '') return body.error;
  } catch {
    // Empty or non-JSON body: use the generic message below.
  }
  return `${path}: HTTP ${res.status}`;
}

/** Which screen to show: first-run setup, login, or the signed-in shell. */
export type AuthState = 'setup' | 'login' | 'shell';

/** `GET /api/setup`, then `GET /api/me` (401 → `login`). Any other failure throws. */
export async function authState(): Promise<AuthState> {
  const { needed } = await getJson<{ needed: boolean }>('/api/setup');
  if (needed) return 'setup';
  try {
    await getJson('/api/me');
    return 'shell';
  } catch (e) {
    if (e instanceof ApiError && e.status === 401) return 'login';
    throw e;
  }
}

/** Create the owner. Throws `ApiError` carrying the server text (400 too short, 409 already set up). */
export async function setupOwner(password: string): Promise<void> {
  await post('/api/setup', { password });
}

/** Log in; the server sets the HttpOnly `polykov_session` cookie. `false` on 401; other failures throw. */
export async function login(password: string): Promise<boolean> {
  try {
    await post('/api/login', { password });
    return true;
  } catch (e) {
    if (e instanceof ApiError && e.status === 401) return false;
    throw e;
  }
}

/** End this session. A 401 (session already gone) counts as logged out; other failures throw. */
export async function logout(): Promise<void> {
  try {
    await post('/api/logout');
  } catch (e) {
    if (!(e instanceof ApiError && e.status === 401)) throw e;
  }
}
