# web/ API

## Modules
| Path | Purpose |
|------|---------|
| `src/main.ts` | Entry: mounts `App` on `#app`. |
| `src/App.svelte` | Router: on load `authState()`, then shows `Setup`, `Login`, `Shell`, or `server unreachable: <cause>` with `[ retry ]`. Setup success → login (with note `owner created — log in`); login success → shell (no reload); logout → login. |
| `src/Shell.svelte` | Shell view: header with server status and a `logout` button, empty body, prompt input. Props: `onlogout: () => void`, called after `logout()` succeeds; a failure shows `logout failed: <cause>` and stays. |
| `src/auth/Setup.svelte` | First-run screen: `password` + `confirm`. Inline errors as you type: `password must be at least 12 characters` (Unicode chars), `passwords do not match`; both forced on submit. Server 400/409 `error` text shown as-is. Props: `ondone`. |
| `src/auth/Login.svelte` | Login screen: `password`; 401 → `wrong password`. Props: `ondone`, optional `note`. |
| `src/auth/AuthPanel.svelte` | Box frame for auth screens: title, optional note, field slot, error lines, `[ action ]` submit button (Enter submits). |
| `src/auth/PasswordPrompt.svelte` | Prompt-style password input (`> label ▌`). Props: `id`, `label`, bindable `value`, `autocomplete`, `autofocus`, `invalid`. |
| `src/app.css` | Global tokens: `--bg --fg --dim --line --accent --ok --err`, font stack, `:focus-visible`. |
| `src/lib/api.ts` | API client helper (below). |

## `src/lib/api.ts`
- `getJson<T>(path: string): Promise<T>` — same-origin `GET` with `Accept: application/json`; throws `ApiError` on non-2xx; parses the JSON body.
- `class ApiError extends Error { status: number }` — HTTP status of the failed call.
- `type Health = { status: string; version: string }` — body of `GET /health`.
- `post(path, body?): Promise<Response>` — same-origin `POST`; JSON body when given; on non-2xx throws `ApiError` whose message is the server's `{"error": "..."}` text, else `<path>: HTTP <status>`.
- `type AuthState = 'setup' | 'login' | 'shell'`.
- `authState(): Promise<AuthState>` — `GET /api/setup` (`needed` → `setup`), then `GET /api/me` (2xx → `shell`, 401 → `login`); other failures throw.
- `setupOwner(password): Promise<void>` — `POST /api/setup`; throws `ApiError` (400, 409).
- `login(password): Promise<boolean>` — `POST /api/login`; `false` on 401, other failures throw. The response token is ignored: the web relies only on the HttpOnly `polykov_session` cookie and never stores the token.
- `logout(): Promise<void>` — `POST /api/logout`; 401 counts as logged out; other failures throw.
- `serverVersion(): Promise<string | null>` — version when `/health` returns 2xx with `status: "ok"`, else `null` (network error, error status, bad body). Never throws.

## Shell status line
- `server …` while loading, `server ok <version>` on success, `server unreachable` otherwise.

## Build
- `npm run build` → `dist/index.html` plus `dist/assets/*`, asset URLs rooted at `/`.
