# web/ API

## Modules
| Path | Purpose |
|------|---------|
| `src/main.ts` | Entry: mounts `App` on `#app`. |
| `src/App.svelte` | Router: on load `authState()`, then shows `Setup`, `Login`, `Shell`, or `server unreachable: <cause>` with `[ retry ]`. Setup success → login (with note `owner created — log in`); login success → shell (no reload); logout → login. |
| `src/Shell.svelte` | Shell view: header with server status and a `logout` button, a body routed on the hash (below), prompt input. Props: `onlogout: () => void`, called after `logout()` succeeds; a failure shows `logout failed: <cause>` and stays. |
| `src/objects/route.ts` | `type Route = {view:'list'} \| {view:'detail', id}`, `parseHash(hash)`, `listHref` (`#/`), `objectHref(id)` (`#/o/<id>`), `ownsKeys(target)` (true for inputs/selects/textareas/buttons/contenteditable, whose keys page shortcuts must ignore). |
| `src/objects/List.svelte` | Object list (no props): `TagTree` sidebar, `CreateForm`, filters (type, `pinned only`, archived `false`/`true`/`any`), rows `* title type #tags` linking to `#/o/<id>`, pages of 50 via `[ more ]`, `no objects` when empty, `error: <cause>` on failure. `j`/`k` move the selection (focusing the row), `Enter` opens it; ignored while focus is in a control (`ownsKeys`). |
| `src/objects/TagTree.svelte` | Tag folders from `GET /api/tags`: an `all` entry, then each folder indented by depth with its leaf name and count. Props: `selected` (`''` = all), `onselect(tag)`. |
| `src/objects/CreateForm.svelte` | Create prompt: type select (default `note`), title input, Enter or `[ add ]` submits; clears the title and calls `oncreated(obj)`; a failure shows the server text (`ApiError` message) inline. |
| `src/objects/Detail.svelte` | Read-only object page. Props: `id` (read once; Shell re-creates it per id). Shows title (`* ` if pinned), type, id, tags, pinned, archived, created, updated, then properties as `key: value` lines (non-strings as JSON). Ends with the `panels` section where 023/024 add one line each. `Esc` or `[ esc ] back` → `#/`; 404 → `object not found`. |
| `src/objects/testing.ts` | Test helpers: `stubApi(routes)` (fake `fetch` keyed `METHOD path`, query ignored, unknown → 599), `urls(fn, prefix)`, `makeObj(id, over)`. |
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
- `type Method = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'`.
- `send(method, path, body?): Promise<Response>` — same-origin request with `Accept: application/json`. A `Blob`/`File` body is sent raw with its own `Content-Type` (none if the blob has no type); any other defined body is sent as JSON; `undefined` sends no body. On non-2xx throws `ApiError` whose message is the server's `{"error": "..."}` text, else `<path>: HTTP <status>`.
- `post(path, body?)` — `send('POST', path, body)`.
- `type AuthState = 'setup' | 'login' | 'shell'`.
- `authState(): Promise<AuthState>` — `GET /api/setup` (`needed` → `setup`), then `GET /api/me` (2xx → `shell`, 401 → `login`); other failures throw.
- `setupOwner(password): Promise<void>` — `POST /api/setup`; throws `ApiError` (400, 409).
- `login(password): Promise<boolean>` — `POST /api/login`; `false` on 401, other failures throw. The response token is ignored: the web relies only on the HttpOnly `polykov_session` cookie and never stores the token.
- `logout(): Promise<void>` — `POST /api/logout`; 401 counts as logged out; other failures throw.
- `serverVersion(): Promise<string | null>` — version when `/health` returns 2xx with `status: "ok"`, else `null` (network error, error status, bad body). Never throws.

## `src/lib/objects.ts`
All built on `send`; failures throw `ApiError`.
- `TYPES` — the server's object types. `type Obj` — the server object JSON (`id, type, title, created, updated, archived, pinned, tags, properties`).
- `type Filter = { type?, tag?, pinned?: boolean, archived?: 'false'|'true'|'any', limit?, offset? }`; `filterQuery(filter)` → `?k=v&...` of the set, non-empty fields, or `''`.
- `listObjects(filter): Promise<{objects: Obj[], total}>` — `GET /api/objects`.
- `getObject(id): Promise<Obj>` — `GET /api/objects/{id}` (404 for unknown or trashed).
- `createObject(input: NewObj): Promise<Obj>` — `POST /api/objects`; `NewObj = { type, title, properties?, pinned?, archived?, tags? }`.
- `updateObject(id, patch: Patch): Promise<Obj>` — `PATCH /api/objects/{id}`; `Patch` = any of `title, properties, pinned, archived, tags`.
- `deleteObject(id): Promise<void>` — `DELETE /api/objects/{id}` (to the trash).
- `listTags(): Promise<TagCount[]>` — `GET /api/tags`, unwrapped; `TagCount = { tag, count }`.

## Hash routes (Shell body)
- `#/o/<id>` (positive integer) → `Detail`; `#/`, empty and any other hash → `List`. Shell follows `hashchange`, so browser back/forward works.

## Shell status line
- `server …` while loading, `server ok <version>` on success, `server unreachable` otherwise.

## Build
- `npm run build` → `dist/index.html` plus `dist/assets/*`, asset URLs rooted at `/`.
