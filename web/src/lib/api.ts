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
