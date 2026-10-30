# web/ API

## Modules
| Path | Purpose |
|------|---------|
| `src/main.ts` | Entry: mounts `App` on `#app`. |
| `src/App.svelte` | Router: on load `authState()`, then shows `Setup`, `Login`, `Shell`, or `server unreachable: <cause>` with `[ retry ]`. Setup success → login (with note `owner created — log in`); setup's `[ log in ]` → login; login success → shell (no reload) with focus in the `command` input; logout → login. |
| `src/Shell.svelte` | Shell view: header with server status and a `logout` button, `Nav`, a body routed on the hash (below), prompt input. Props: `onlogout: () => void`, optional `focusCommand` (focus the command input on mount); `onlogout` is called after `logout()` succeeds; a failure shows `logout failed: <cause>` and stays. A delete on the detail goes to `#/` with the note `moved to trash` above the list, cleared once the route leaves the list. |
| `src/tasks/route.ts` | `type TaskRoute = {view:'list', list} \| {view:'project', id} \| {view:'area', id}`, `parseTaskHash(hash)` (`null` when not a task route), `taskListHref(list)` (`#/<list>`), `projectHref(id)` (`#/p/<id>`), `areaHref(id)` (`#/a/<id>`). |
| `src/tasks/Nav.svelte` | Nav (no props, in Shell above the body): links to the five lists, then `objects` (`#/`) and `trash`, then active projects and non-archived areas (loaded once on mount; a failure shows `nav: <cause>` and keeps the fixed links). The current one has `aria-current="page"`. Below 600 px only the `[ lists ]` button (accessible name `lists`, `aria-expanded`) shows; it opens a vertical menu focused on its first link, ArrowUp/ArrowDown move between links, Esc closes it (handled: `preventDefault` + `stopPropagation`) and refocuses the button, a route change closes it. The open menu carries `data-owns-keys`, so list keys pass it by. |
| `src/tasks/TaskList.svelte` | Props `name: ListName` (`all` included), optional `filter: {project?, area?}` (embedded: no `<h1>`). Rows from `listTasks` in pages of 100 (`[ more ]`); `upcoming` is grouped under `<h2>` date headers (scheduled, else deadline); `nothing in <name>` when empty, `error: <cause>` on failure. Reloads on the window event `TASKS_CHANGED` (`polykov:tasks-changed`, exported from the module script). Keys (ignored while a control owns them, `ownsKeys`, or focus is inside a `[data-owns-keys]` element): `j`/`k` select and focus a row (nothing is selected until the first `j`), `Enter` opens it (`#/o/<id>`; a focused link follows itself), `x` toggles it done. |
| `src/tasks/TaskRow.svelte` | One `<li>`. Props `row: Row`, `today`, optional `selected`, `onfocus`. `[ ]`/`[x]` button (name `complete <title>`/`reopen <title>`) calls complete/reopen and keeps the row (struck through) until the list reloads; a recurring completion shows `done · next <date>`; failures show `error: <cause>`. Then `p1`–`p3` (P4 hidden) and the title, linking to `#/o/<id>`; meta: scheduled, `due <deadline>` (`--err` when its date is before `today`), `[open/total]` sub-tasks, `#tags`. Below 40 rem the meta wraps under the title. Exports `toggle()` and `focus()` (the title link). |
| `src/tasks/ProjectView.svelte` | Props `id` (read once; Shell re-creates it per route), `kind: 'project' \| 'area'`. Project: title, `status` and `area` selects (each change one `PATCH /api/projects/{id}`), `start`/`deadline` when set; the unsectioned tasks (list `no section`), then each section (`<h2>`, list named by its title) with its open tasks (`all?project=<id>`, limit 1000); `[ add section ]` → `section title` prompt (Enter adds it last); per section `[ rename ]` (prompt `rename <title>`, prefilled), `[ up ]`/`[ down ]` (disabled at the ends; focus stays on the moved section), `[ delete ]` → `delete <title>? [ yes ] [ no ]` (`[ no ]` focused). Rename, move and delete re-read the sections, so positions match the server (`0..n-1`). Prompts: Esc cancels (handled). Errors show `error: <cause>`; 404 → `project not found`. Reloads tasks on `polykov:tasks-changed`. Area: its title, its projects (links `#/p/<id>` with `<n> open`), then `TaskList name=all filter={{area}}`; 404 or not an area → `area not found`. |
| `src/tasks/testing.ts` | Test fixture `makeRow(id, task?, over?)`: an open P4 task row. |
| `src/objects/route.ts` | `type Route = {view:'list'} \| {view:'detail', id} \| {view:'trash'}`, `parseHash(hash)`, `listHref` (`#/`), `objectHref(id)` (`#/o/<id>`), `trashHref` (`#/trash`), `ownsKeys(target)` (true for inputs/selects/textareas/buttons/contenteditable, whose keys page shortcuts must ignore). |
| `src/objects/List.svelte` | Object list (no props): `TagTree` sidebar, `CreateForm`, filters (type, `pinned only`, archived `false`/`true`/`any`), a `trash` link to `#/trash`, rows `* title type #tags` linking to `#/o/<id>`, pages of 50 via `[ more ]`, `no objects` when empty, `error: <cause>` on failure. `j`/`k` move the selection (focusing the row), `Enter` opens it; ignored while focus is in a control (`ownsKeys`) other than a tag-folder button. |
| `src/objects/TagTree.svelte` | Tag folders from `GET /api/tags`: an `all` entry, then each folder indented by depth with its leaf name and count. Props: `selected` (`''` = all), `onselect(tag)`. Folder buttons carry `data-tag-folder`, so List keys pass through them. |
| `src/objects/CreateForm.svelte` | Create prompt: type select (default `note`), title input, Enter or `[ add ]` submits; clears the title and calls `oncreated(obj)`; a failure shows the server text (`ApiError` message) inline. |
| `src/objects/Detail.svelte` | Object page. Props: `id` (read once; Shell re-creates it per id), `ondeleted: () => void` (called after the `DELETE`). Title (`* ` if pinned) is a button that edits in place: Enter saves, Esc cancels. `[ pin ]`/`[ unpin ]`, `[ archive ]`/`[ unarchive ]`, `[ delete ]`. Then type, id, tags (`TagsEditor`), pinned, archived, created, updated, and `PropsEditor`. Every save is one `PATCH` whose response replaces the object; server errors show inline. Ends with the `panels` section: `Links`, `Comments`, `Attachments`. `Esc` (unless an editor used it to cancel, via `preventDefault`, or focus is in a control that owns keys, `ownsKeys`, such as the Shell `command` input) or `[ esc ] back` → `#/`; 404 → `object not found`. |
| `src/objects/Links.svelte` | Props: `id`. `links` (each `title type [ unlink ]`) and `backlinks` (`title type`); every title links to `#/o/<id>`; `no links`/`no backlinks` when empty. `[ link ]` opens the `id or title` prompt: Enter with a number sends the `PUT`; other text lists up to 10 matches from `findByTitle` (`no matches` if none), each a button that links it. Esc closes the prompt (with `preventDefault`). Success reloads the links and moves focus: after a link to `[ link ]`, after an unlink to the next row's `[ unlink ]`, else `[ link ]`; server errors show inline. |
| `src/objects/Comments.svelte` | Props: `id`. Comments oldest first, each a header row `<created>` then `[ edit ] [ delete ]` in one non-wrapping group (the time may wrap above it), over the body; `no comments` when empty. The `add comment` textarea: Ctrl+Enter or `[ add ]` posts it and clears it; Esc clears a non-empty one (with `preventDefault`). `[ edit ]` swaps the body for a focused textarea: Ctrl+Enter saves (focus back to its `[ edit ]`), Esc cancels (with `preventDefault`). `[ delete ]` deletes at once (focus to the next comment's `[ edit ]`, else the `add comment` textarea). Server errors (e.g. the 400 `body must be 1 to 10000 characters`) show inline and keep the draft. |
| `src/objects/TagsEditor.svelte` | Props: `tags`, `onsave(tags): Promise<void>`. `#tag [ x ]` removes one; the `add tag` prompt (Enter) adds one; each sends the whole set; a rejection's message shows inline. Esc clears a non-empty prompt. |
| `src/objects/PropsEditor.svelte` | Props: `properties`, `onsave(props): Promise<void>`. Shows `key: value` lines (non-strings as JSON) and `[ edit properties ]`, which opens a textarea; `[ save ]` or Ctrl+Enter saves, `[ cancel ]` or Esc cancels. A parse error (`line N: expected key: value`, `missing key`, `duplicate key "k"`) shows inline without sending. Module exports: `show(value)`, `formatProps(props)`, `parseValue(raw)` (integer/float, then `true`/`false`, else string, as CLI `--prop`), `parseProps(text, before)` (a line whose value text is unchanged keeps its old value, so arrays/`null`/numeric strings survive). |
| `src/objects/Trash.svelte` | Trash view (no props) from `lib/trash.ts`: rows `title`, `type · deleted <day> · purges <day>`, `[ restore ]`, `[ delete forever ]` → `delete forever? [ yes ] [ no ]`; `[ empty trash ]` → `delete all <n> forever? [ yes ] [ no ]` (`[ no ]` gets focus). Results show as a note (`restored <title>`, `deleted <title> forever`, `deleted <n> forever`), failures as `error: <cause>`; `trash is empty` when empty. Esc cancels a pending confirm, else → `#/`. |
| `src/objects/Attachments.svelte` | Attachments panel in `Detail`'s `panels` section. Props: `id`. Rows `<thumb> name size mime` (`humanSize`; a thumbnail from `?inline=1` for `image/*`), `[ download ]` (link to `/api/attachments/<aid>`), `[ delete ]` → `delete <name>? [ yes ] [ no ]` (`[ no ]` gets focus; Esc or `[ no ]` cancels, with `preventDefault`, and refocuses that row's `[ delete ]`). `no attachments · drop files here` when empty. `[ attach ]` opens a file picker (multiple); files picked or dropped on the panel upload one request each, in order, with `uploading <name>…`; the first failure stops the rest; a drop during an upload is ignored. At phone width the size, mime and actions wrap under the name (`min-width: 14ch`). Errors show inline: 413 → `file too large (max 25 MiB)`, else the server text. |
| `src/objects/testing.ts` | Test helpers: `stubApi(routes)` (fake `fetch` keyed `METHOD path`, query ignored, unknown → 599), `urls(fn, prefix)`, `makeObj(id, over)`. |
| `src/auth/Setup.svelte` | First-run screen: `password` + `confirm`. Inline errors as you type: `password must be at least 12 characters` (Unicode chars), `passwords do not match`; both forced on submit. Server 400/409 `error` text shown as-is; after a 409 the fields go and the action becomes `[ log in ]`. Props: `ondone`, `onlogin` (the `[ log in ]` action). |
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

## `src/lib/links.ts`
All built on `send`; failures throw `ApiError`.
- `type Summary = { id, type, title }`; `type Links = { links: Summary[], backlinks: Summary[] }` (each by title).
- `getLinks(id): Promise<Links>` — `GET /api/objects/{id}/links`.
- `addLink(id, to): Promise<void>` — `PUT /api/objects/{id}/links/{to}` (400 self-link, 404 unknown end).
- `removeLink(id, to): Promise<void>` — `DELETE /api/objects/{id}/links/{to}` (404 `link not found`).
- `MATCHES` (10); `findByTitle(query, self): Promise<Obj[]>` — one `listObjects({archived:'any', limit:1000})`, then the first `MATCHES` whose title contains the trimmed `query` ignoring case, `self` excluded.

## `src/lib/comments.ts`
All built on `send`; failures throw `ApiError` (400 `body must be 1 to 10000 characters`, 404).
- `type Comment = { id, object_id, body, created, updated }` (RFC 3339).
- `listComments(id): Promise<Comment[]>` — `GET /api/objects/{id}/comments`, unwrapped, oldest first.
- `addComment(id, body): Promise<Comment>` — `POST /api/objects/{id}/comments`.
- `editComment(cid, body): Promise<Comment>` — `PATCH /api/comments/{cid}`.
- `deleteComment(cid): Promise<void>` — `DELETE /api/comments/{cid}`.

## `src/lib/tasks.ts`
All built on `send`; failures throw `ApiError`.
- `type TaskFields` (`status, priority, scheduled, deadline, estimate, someday, parent_id, project_id, section_id, area_id, checklist, recurrence, completed`), `type Task = Obj & { task }`, `type Row = Task & { subtasks: { open, total } }`, `Recurrence`, `ChecklistItem`, `FieldChanges` (settable task fields), `NewTask`, `TaskPatch`.
- `LISTS` (`inbox today upcoming someday anytime`); `type ListName` adds `all`; `ListFilter = { project?, area?, limit?, offset? }`.
- `localToday(now?)` — the browser's local `YYYY-MM-DD`; sent as `today` by `listTasks` and `completeTask`.
- `listTasks(name, filter?): Promise<{tasks: Row[], total}>` — `GET /api/lists/{name}?today=...&<set filter fields>`.
- `listSubtasks(id): Promise<Row[]>` — `GET /api/tasks/{id}/subtasks`, unwrapped.
- `getTask(id)`, `createTask(input)` (`POST /api/tasks`), `updateTask(id, patch)` (`PATCH /api/tasks/{id}`) → `Task`.
- `completeTask(id): Promise<{task, next: Task | null}>` — `POST /api/tasks/{id}/complete?today=...`; `reopenTask(id): Promise<Task>`.

## `src/lib/projects.ts`
All built on `send`; failures throw `ApiError`.
- `PROJECT_STATUSES` (`active someday done`), `type Project = Obj & { project: { status, area_id, start, deadline, open_tasks, done_tasks } }`, `ProjectChanges`, `type Section = { id, project_id, title, position }`.
- `listProjects({status?, area?}?): Promise<Project[]>` — `GET /api/projects` (server default `active`), unwrapped.
- `getProject(id)` (404 `project not found`), `updateProject(id, changes)` (`PATCH`) → `Project`.
- `listSections(project): Promise<Section[]>` (by position); `createSection(project, title, position?)` (last by default); `updateSection(sid, {title?, position?})`; `deleteSection(sid)` (its tasks become unsectioned).

## `e2e/` (Playwright; run by `scripts/e2e.sh`)
- `playwright.config.ts` — projects `phone` (360×740, mobile, touch) and `desktop` (1280×800), Chromium; `BASE_URL` from the env; every test starts logged in from the storage state that `e2e/global-setup.ts` saves (`STATE`). Opt out with `test.use({ storageState: { cookies: [], origins: [] } })`.
- `helpers.ts`: `password()` (env `E2E_PASSWORD`), `uid()` (8 hex chars for unique titles), `login(page)` (login screen → shell), `api(page.request)` → `{ create(title, over?), link(from, to), comment(id, body), attach(id, name, mime, body), trash(id) }` (throws on non-2xx), `expectPhoneFit(page)` (throws, naming each element, on horizontal scroll or a visible `a, button, input, select, textarea, [role=button], [tabindex]` under 44×44 px), `tabTo(page, name)` (Tab until the focused element's accessible name is `name`; throws after 50 presses).

## Hash routes (Shell body)
- Task routes are checked first: `#/inbox`, `#/today`, `#/upcoming`, `#/someday`, `#/anytime` → `TaskList`; `#/p/<id>` → `ProjectView kind=project`; `#/a/<id>` → `ProjectView kind=area`.
- `#/trash` → `Trash`.
- `#/o/<id>` (positive integer) → `Detail`; `#/`, empty and any other hash → `List`. Shell follows `hashchange`, so browser back/forward works.

## Shell status line
- `server …` while loading, `server ok <version>` on success, `server unreachable` otherwise.

## Build
- `npm run build` → `dist/index.html` plus `dist/assets/*`, asset URLs rooted at `/`.
