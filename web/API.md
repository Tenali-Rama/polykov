# web/ API

## Modules
| Path | Purpose |
|------|---------|
| `src/main.ts` | Entry: mounts `App` on `#app`. |
| `src/App.svelte` | Shell view: header with server status, empty body, prompt input. |
| `src/app.css` | Global tokens: `--bg --fg --dim --line --accent --ok --err`, font stack, `:focus-visible`. |
| `src/lib/api.ts` | API client helper (below). |

## `src/lib/api.ts`
- `getJson<T>(path: string): Promise<T>` — same-origin `GET` with `Accept: application/json`; throws `ApiError` on non-2xx; parses the JSON body.
- `class ApiError extends Error { status: number }` — HTTP status of the failed call.
- `type Health = { status: string; version: string }` — body of `GET /health`.
- `serverVersion(): Promise<string | null>` — version when `/health` returns 2xx with `status: "ok"`, else `null` (network error, error status, bad body). Never throws.

## Shell status line
- `server …` while loading, `server ok <version>` on success, `server unreachable` otherwise.

## Build
- `npm run build` → `dist/index.html` plus `dist/assets/*`, asset URLs rooted at `/`.
