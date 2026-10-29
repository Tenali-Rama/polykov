# web/ API

## Modules
| Path | Purpose |
|------|---------|
| `src/main.ts` | Entry: mounts `App` on `#app`. |
| `src/App.svelte` | Router: on load `authState()`, then shows `Setup`, `Login`, `Shell`, or `server unreachable: <cause>` with `[ retry ]`. Setup success → login (with note `owner created — log in`); login success → shell (no reload); logout → login. |
| `src/Shell.svelte` | Shell view: header with server status and a `logout` button, a body routed on the hash (below), prompt input. Props: `onlogout: () => void`, called after `logout()` succeeds; a failure shows `logout failed: <cause>` and stays. A delete on the detail goes to `#/` with the note `moved to trash` above the list, cleared once the route leaves the list. |
| `src/objects/route.ts` | `type Route = {view:'list'} \| {view:'detail', id} \| {view:'trash'}`, `parseHash(hash)`, `listHref` (`#/`), `objectHref(id)` (`#/o/<id>`), `trashHref` (`#/trash`), `ownsKeys(target)` (true for inputs/selects/textareas/buttons/contenteditable, whose keys page shortcuts must ignore). |
| `src/objects/List.svelte` | Object list (no props): `TagTree` sidebar, `CreateForm`, filters (type, `pinned only`, archived `false`/`true`/`any`), a `trash` link to `#/trash`, rows `* title type #tags` linking to `#/o/<id>`, pages of 50 via `[ more ]`, `no objects` when empty, `error: <cause>` on failure. `j`/`k` move the selection (focusing the row), `Enter` opens it; ignored while focus is in a control (`ownsKeys`). |
| `src/objects/TagTree.svelte` | Tag folders from `GET /api/tags`: an `all` entry, then each folder indented by depth with its leaf name and count. Props: `selected` (`''` = all), `onselect(tag)`. |
| `src/objects/CreateForm.svelte` | Create prompt: type select (default `note`), title input, Enter or `[ add ]` submits; clears the title and calls `oncreated(obj)`; a failure shows the server text (`ApiError` message) inline. |
| `src/objects/Detail.svelte` | Object page. Props: `id` (read once; Shell re-creates it per id), `ondeleted: () => void` (called after the `DELETE`). Title (`* ` if pinned) is a button that edits in place: Enter saves, Esc cancels. `[ pin ]`/`[ unpin ]`, `[ archive ]`/`[ unarchive ]`, `[ delete ]`. Then type, id, tags (`TagsEditor`), pinned, archived, created, updated, and `PropsEditor`. Every save is one `PATCH` whose response replaces the object; server errors show inline. Ends with the `panels` section where 023/024 add one line each. `Esc` (unless an editor used it to cancel, via `preventDefault`) or `[ esc ] back` → `#/`; 404 → `object not found`. |
| `src/objects/TagsEditor.svelte` | Props: `tags`, `onsave(tags): Promise<void>`. `#tag [ x ]` removes one; the `add tag` prompt (Enter) adds one; each sends the whole set; a rejection's message shows inline. Esc clears a non-empty prompt. |
| `src/objects/PropsEditor.svelte` | Props: `properties`, `onsave(props): Promise<void>`. Shows `key: value` lines (non-strings as JSON) and `[ edit properties ]`, which opens a textarea; `[ save ]` or Ctrl+Enter saves, `[ cancel ]` or Esc cancels. A parse error (`line N: expected key: value`, `missing key`, `duplicate key "k"`) shows inline without sending. Module exports: `show(value)`, `formatProps(props)`, `parseValue(raw)` (integer/float, then `true`/`false`, else string, as CLI `--prop`), `parseProps(text, before)` (a line whose value text is unchanged keeps its old value, so arrays/`null`/numeric strings survive). |
| `src/objects/Trash.svelte` | Trash view (no props) from `lib/trash.ts`: rows `title`, `type · deleted <day> · purges <day>`, `[ restore ]`, `[ delete forever ]` → `delete forever? [ yes ] [ no ]`; `[ empty trash ]` → `delete all <n> forever? [ yes ] [ no ]` (`[ no ]` gets focus). Results show as a note (`restored <title>`, `deleted <title> forever`, `deleted <n> forever`), failures as `error: <cause>`; `trash is empty` when empty. Esc cancels a pending confirm, else → `#/`. |
| `src/objects/Attachments.svelte` | Attachments panel in `Detail`'s `panels` section. Props: `id`. Rows `<thumb> name size mime` (`humanSize`; a thumbnail from `?inline=1` for `image/*`), `[ download ]` (link to `/api/attachments/<aid>`), `[ delete ]` → `delete <name>? [ yes ] [ no ]` (`[ no ]` gets focus; Esc or `[ no ]` cancels, with `preventDefault`, and refocuses that row's `[ delete ]`). `no attachments · drop files here` when empty. `[ attach ]` opens a file picker (multiple); files picked or dropped on the panel upload one request each, in order, with `uploading <name>…`; the first failure stops the rest; a drop during an upload is ignored. At phone width the size, mime and actions wrap under the name (`min-width: 14ch`). Errors show inline: 413 → `file too large (max 25 MiB)`, else the server text. |
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

## `src/lib/trash.ts`
All built on `send`; failures throw `ApiError` (404 `not in trash` for an unknown or live id).
- `type Trashed = Obj & { deleted, purge_at }` (RFC 3339).
- `listTrash(): Promise<Trashed[]>` — `GET /api/trash`, unwrapped, newest deleted first.
- `restoreObject(id): Promise<Obj>` — `POST /api/trash/{id}/restore`.
- `purgeObject(id): Promise<void>` — `DELETE /api/trash/{id}`.
- `emptyTrash(): Promise<number>` — `DELETE /api/trash`; the purged count.

## `src/lib/attachments.ts`
All built on `send`; failures throw `ApiError`.
- `type Attachment` — the server attachment JSON (`id, object_id, name, mime, size, created`).
- `listAttachments(id): Promise<Attachment[]>` — `GET /api/objects/{id}/attachments`, unwrapped, oldest first.
- `uploadAttachment(id, file: File): Promise<Attachment>` — `POST /api/objects/{id}/attachments?name=<encodeURIComponent(file.name)>`, the file as the raw body (its type as `Content-Type`); 413 over 25 MiB.
- `deleteAttachment(aid): Promise<void>` — `DELETE /api/attachments/{aid}`.
- `downloadUrl(aid, inline = false): string` — `/api/attachments/{aid}`, plus `?inline=1` when `inline`.
- `humanSize(bytes): string` — `512 B`, `5.0 KiB`, `1.2 MiB`, ... (as the CLI).

## Hash routes (Shell body)
- `#/trash` → `Trash`.
- `#/o/<id>` (positive integer) → `Detail`; `#/`, empty and any other hash → `List`. Shell follows `hashchange`, so browser back/forward works.

## Shell status line
- `server …` while loading, `server ok <version>` on success, `server unreachable` otherwise.

## Build
- `npm run build` → `dist/index.html` plus `dist/assets/*`, asset URLs rooted at `/`.
