# server API

## Binary
`polykov-server [--data-dir <dir>] [--bind <addr>] [--web-dir <dir>]`

On start it resolves the data dir, creates `attachments/`, `models/` and `backups/` in it, writes a fully commented `config.toml` if none exists (an existing one is never touched), then reads its settings. It then opens `<data dir>/polykov.db` (see Database). Any config or database error exits non-zero with a one-line message naming the file and line, key, or env var at fault.

### Data dir
First match wins: `--data-dir`, `POLYKOV_DATA_DIR`, `$XDG_DATA_HOME/polykov` (absolute `XDG_DATA_HOME` only), `$HOME/.local/share/polykov`.

### Settings
Precedence: flag > env var > `<data dir>/config.toml` > default. An empty env var counts as unset. An unknown key in `config.toml` is an error.

| Key | Flag | Env | Default | Meaning |
|---|---|---|---|---|
| `bind` | `--bind` | `POLYKOV_BIND` | `127.0.0.1:7700` | Socket address to listen on |
| `web_dir` | `--web-dir` | `POLYKOV_WEB_DIR` | `web/dist` | Built web UI dir; relative paths resolve against the working dir |

### Database
`<data dir>/polykov.db`, SQLite (bundled, FTS5 available) in WAL mode with `foreign_keys` on. On open, every migration in `server/migrations/` newer than the recorded version is applied in order, each in its own transaction, and recorded in `schema_version (version, applied_at)`. A database whose recorded version is newer than the binary's latest migration fails start-up with `database schema version N is newer than this binary supports (M); upgrade polykov-server`.

Tables (migration 4, all M1 data; times in Unix seconds; every side table cascades on delete of its object): `objects (id AUTOINCREMENT, type, title, properties JSON text default '{}', pinned, archived, created, updated, deleted_at NULL = live)`, `object_tags (object_id, tag)`, `links (src_id, dst_id, created)` (no self-links), `comments (id, object_id, body, created, updated)`, `attachments (id, object_id, name, mime, size, created)`. See ADR 0007.

Tables (migration 5, all M2 data; ADR 0011): `tasks (object_id PK, status 'open'|'done', priority 1..4 default 4, scheduled, deadline, estimate minutes, someday, parent_id, project_id, section_id, area_id, checklist JSON default '[]', recurrence JSON NULL, completed)`, `projects (object_id PK, status 'active'|'someday'|'done', area_id, start, deadline)`, `sections (id AUTOINCREMENT, project_id, title, position, created)`. Dates are floating local TEXT (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`). The side rows cascade with their object; references to other objects or sections are `ON DELETE SET NULL`. Indexes: `tasks (parent_id | project_id | area_id | scheduled | deadline)`, `sections (project_id)`.

Tables (migration 6; ADR 0014): `search`, an FTS5 table (trigram tokenizer) with one row per live object, `rowid` = object id, columns `title, tags, props, comments` (tags space-joined, every string value of `properties`, comment bodies space-joined), filled from the view `search_source` and kept in step by triggers on `objects`, `object_tags` and `comments`. Trashed objects have no row; restoring one re-indexes it. Never write to it directly.

Adding a migration: create `server/migrations/NNNN_<name>.sql` (next number, no gaps) and append it to `MIGRATIONS` in `src/db/migrate.rs`. A test fails if the list and the dir disagree. Never edit a migration that has been merged.

## Library (`polykov_server`)
- `app::router(db: db::Db, web_dir: &Path) -> axum::Router`: the full HTTP app over `db`, serving the web UI from `web_dir` (see Web UI), with no bound port.
- `web::service(web_dir: &Path) -> axum::Router`: the web UI fallback mounted by `app::router`. `web::NOT_BUILT` is the body of its `503`.
- `db::Db::open(data_dir: &Path) -> Result<Db, db::Error>`: opens `data_dir/polykov.db` (`db::FILE_NAME`), sets the pragmas and migrates. `Db` is a cheap-to-clone handle to one connection behind a mutex.
- `db::Db::check(&self) -> Result<(), db::Error>`: reads `schema_version`; what `/health` reports as `db`.
- `db::migrate::{latest() -> u32, current(&Connection) -> rusqlite::Result<u32>, run(&mut Connection) -> Result<(), db::Error>}`: the binary's latest version, a DB's recorded version (0 if never migrated), and apply pending migrations.
- `db::Error` displays a one-line message.
- `auth::routes() -> Router<Db>`: the setup and login routes below, merged by `app::router`.
- `auth::session::create(&Db) -> Result<String, auth::Error>`: starts a session expiring `session::TTL_SECS` (30 days) from now and returns its token (64 hex chars, 256 random bits). Only `session::hash_token(token)` (hex SHA-256) is stored, in `sessions (token_hash, created_at, expires_at)`, times in Unix seconds.
- `auth::session::is_valid(&Db, token: &str) -> Result<bool, auth::Error>`: whether `token` is a known, unexpired session.
- `auth::session::expires_at(&Db, token: &str) -> Result<Option<String>, auth::Error>`: the unexpired session's expiry as RFC 3339 UTC (`YYYY-MM-DDTHH:MM:SSZ`), `None` if unknown or expired.
- `auth::session::delete(&Db, token: &str) -> Result<(), auth::Error>`: ends that session; an unknown token is not an error.
- `auth::Session { token: String, expires_at: Option<String> }` (`None` for an API token): Axum extractor (`FromRequestParts` for any state where `Db: FromRef<S>`) that resolves the caller's session. See Authentication.
- `auth::tokens::{create(&Db, name) -> Result<(i64, String), auth::Error>, list(&Db) -> Result<Vec<tokens::Info>, _>, delete_by_id(&Db, id) -> Result<bool, _>, authenticate(&Db, token) -> Result<bool, _>}`: the API token store, `api_tokens (id, name, token_hash, created_at, last_used)` (Unix seconds, `last_used` NULL until first use). Tokens are made like session tokens and stored only as `session::hash_token`. `authenticate` records `last_used` on a hit. `Info { id, name, created, last_used: Option<_> }` (RFC 3339 UTC) never holds the token or hash. `tokens::MAX_NAME_CHARS` is 64.
- `auth::owner::{exists, create, password_hash}` and `auth::password::{hash, verify}`: the single `owner` row and argon2id hashing (PHC strings). `auth::Error` displays a one-line message that never contains a password or token.
- `http::ApiError` (`IntoResponse`): `ApiError::bad_request(msg)` / `not_found(msg)` (or `Client(status, msg)`) send `4xx {"error":"<msg>"}`; `Internal(cause)` logs one line and sends a bare `500`. `From<db::Error>` gives `Internal`. Feature-module handlers return `Result<_, ApiError>`.
- `http::blocking(f) -> Result<T, ApiError>`: runs the store call `f: FnOnce() -> Result<T, E>` (`E: Into<ApiError>`) on Tokio's blocking pool; a panic becomes `Internal`.
- `objects::routes() -> Router<Db>`: the object routes below, merged by `app::router`.
- `objects::store::{create(&Db, &New) -> Result<Object, db::Error>, get(&Db, id) -> Result<Option<Object>, _>, update(&Db, id, &Changes) -> Result<Option<Object>, _>, trash(&Db, id) -> Result<bool, _>}`: the `objects` table. Reads and writes only see live objects (`deleted_at IS NULL`). `New { type, title, properties, pinned, archived, tags }` and `Changes` (all `Option`, `None` = unchanged; `tags` replaces the set) must already be validated with `objects::validate::{object_type, title, properties}` and `tags::normalize_all`. `Object` serializes to the object JSON below.
- `objects::store::summary(&Db, id) -> Result<Option<Summary>, db::Error>`: `Summary { id, type, title }` of a live object, `None` if unknown or in the trash. Links, comments and attachments use it to check their target.
- `db::Db::data_dir(&self) -> &Path`: the data dir the database was opened in; feature modules keep their files under it.
- `attachments::routes() -> Router<Db>`: the attachment routes below, merged by `app::router`.
- `attachments::remove_files(&Db, object_id) -> Result<(), attachments::store::Error>`: deletes the files of every attachment of that object, live or in the trash, and leaves the rows (a permanent `DELETE FROM objects` cascades to them). A missing file is not an error. Call it before purging an object (task 017).
- `attachments::store::{create, list, get, delete, file_path}`: the `attachments` rows plus `<data dir>/attachments/<id>` files. `create` writes `<id>.tmp`, syncs and renames it inside the row's transaction, so a failed write leaves no row. `store::Error` (`Db` or `Io(path, _)`) displays a one-line message and becomes a bare `500`. `attachments::{MAX_BYTES (25 MiB), MAX_NAME_CHARS (255)}`.
- `tags::routes() -> Router<Db>`: `GET /api/tags`, merged by `app::router`.
- `tags::{normalize(&str) -> Result<String, ApiError>, normalize_all(&[String]) -> Result<Vec<String>, ApiError>, MAX_TAG_CHARS (100), MAX_TAGS (50)}`: the tag rules below; `normalize_all` also dedups and sorts.
- `tags::store::{replace(&Connection, id, &[String]), counts(&Db) -> Result<Vec<Count>, _>, SORTED_JSON, under(param) -> String}`: the `object_tags` table. `replace` runs inside the caller's object-write transaction. `SORTED_JSON` and `under(n)` are SQL fragments over the `objects` row in scope (its sorted tags as a JSON array; "carries the tag bound to `?n` or one under it").
- `objects::validate::{TYPES, MAX_TITLE_CHARS (1000), MAX_PROPERTIES (100), MAX_KEY_CHARS (64)}`: the object input rules below.
- `links::routes() -> Router<Db>` and `comments::routes() -> Router<Db>`: the link and comment routes below, merged by `app::router`.
- `links::store::{add(&Db, src, dst) -> Result<(), db::Error>, remove(&Db, src, dst) -> Result<bool, _>, list(&Db, id) -> Result<Links, _>}`: the `links` table. Callers check both ends with `objects::store::summary`. `Links { links, backlinks }` hold `Summary`s of live objects, by title.
- `comments::store::{create(&Db, object_id, body) -> Result<Comment, db::Error>, list(&Db, object_id) -> Result<Vec<Comment>, _>, update(&Db, id, body) -> Result<Option<Comment>, _>, delete(&Db, id) -> Result<bool, _>}`: the `comments` table; comments on trashed objects are invisible to `update`/`delete`. `body` must already be trimmed and 1 to `comments::MAX_BODY_CHARS` (10000) characters. `Comment` serializes to the comment JSON below.
- `trash::routes() -> Router<Db>`: the trash routes below, merged by `app::router`.
- `trash::purge_expired(&Db, now_unix: i64) -> Result<usize, attachments::store::Error>`: deletes for good every object trashed more than `trash::store::RETENTION_SECS` (30 days) before `now_unix`, files first (`attachments::remove_files`), and returns how many. Live objects are never touched.
- `trash::spawn_purger(Db) -> std::io::Result<()>`: starts a thread that runs `purge_expired` at start-up and then every `trash::PURGE_EVERY` (1 hour); a failed run logs one line. Called by `main.rs`.
- `trash::store::{list(&Db) -> Result<Vec<Trashed>, db::Error>, restore(&Db, id) -> Result<Option<Object>, _>, purge(&Db, id) -> Result<bool, attachments::store::Error>, empty(&Db) -> Result<usize, _>}`: objects with `deleted_at` set. `Trashed` serializes to the object JSON plus `deleted` and `purge_at`.
- `tasks::routes() -> Router<Db>`: the task routes below, merged by `app::router`.
- `tasks::store::{create(&Db, &New) -> Result<Task, ApiError>, get(&Db, id) -> Result<Option<Task>, db::Error>, get_many(&Db, &[i64]) -> Result<Vec<Task>, db::Error>, update(&Db, id, &Changes) -> Result<Option<Task>, ApiError>, reopen(&Db, id) -> Result<Option<Task>, db::Error>}`: live `task` objects joined with their `tasks` row (defaults when there is none). `get_many` returns the live tasks among the ids in the given order and skips the rest. `New { title, properties, pinned, tags, task: FieldChanges }` and `Changes { object: objects::store::Changes, task: FieldChanges }` need their object fields validated as for objects; the store checks every task rule itself (a broken one is `ApiError` `400`) and writes object and task row in one transaction. `FieldChanges` (also the `"task"` request body; all `Option`, `None` = unchanged, `Some(None)` clears) has `priority, scheduled, deadline, estimate, someday, parent_id, project_id, section_id, area_id, checklist, recurrence` (`Option<Option<recurrence::Recurrence>>`). `Task { object: Object, task: Fields }` serializes to the task JSON below.
- `tasks::recurrence::{complete(&Db, id, today: NaiveDate) -> Result<Option<(Task, Option<Task>)>, ApiError>, validate(&Recurrence) -> Result<Recurrence, ApiError>, Recurrence { rule: String, from: Anchor }, Anchor::{Scheduled, Completion}}`: marks the task done and, if it recurs, creates its next occurrence in the same transaction (see `complete` below). `validate` returns the rule in canonical form or `400 invalid recurrence: <reason>`.
- `recur::{Rule { freq: Freq, interval: u32, by_day: Vec<chrono::Weekday>, month_day: Option<u32> }, Freq::{Day, Week, Month, Year}, Rule::parse(&str) -> Result<Rule, String>, next(&Rule, after: NaiveDate) -> NaiveDate, MAX_INTERVAL (365)}`: pure recurrence rules (ADR 0013). `parse` reads `FREQ=DAILY|WEEKLY|MONTHLY|YEARLY[;INTERVAL=1..365][;BYDAY=MO,TU,..][;BYMONTHDAY=1..31]` in any order and case (`BYDAY` only with `WEEKLY`, `BYMONTHDAY` only with `MONTHLY`); the error is a short reason. `Display` gives the canonical form (`FREQ`, then `INTERVAL` if not 1, `BYDAY` Monday first, `BYMONTHDAY`). `next` is the first date strictly after `after`, counting the series from `after`: weeks start on Monday, `WEEKLY` without `BYDAY` keeps `after`'s weekday, `MONTHLY`/`YEARLY` without a month day keep `after`'s day, and a day past the month's end (also Feb 29) clamps to its last day. It saturates at `NaiveDate::MAX`.
- `lists::routes() -> Router<Db>`: the list routes below, merged by `app::router`.
- `lists::store::{list(&Db, &Query) -> Result<Page, db::Error>, subtasks(&Db, parent) -> Result<Vec<Row>, _>, List::parse(&str) -> Option<List>}`: the GTD list SQL over `tasks`/`projects`; rows come from `tasks::store::get_many`. `Query { list, today: "YYYY-MM-DD", sort: Sort, project, area, limit, offset }` must already be validated; `Sort::{Default, Smart}` picks the list's own order or `smartsort`. `Row { task: Task, subtasks: Counts { open, total } }` and `Page { tasks: Vec<Row>, total }` serialize to the JSON below. `lists::{DEFAULT_LIMIT (100), MAX_LIMIT (1000)}`.
- `smartsort::{score(&Fields, today: NaiveDate) -> f64, sort(&mut [Task], today)}`: smart sort, pure and deterministic. Higher score first, ties by ascending id. The formula and weights (`PRIORITY_WEIGHT`, `OVERDUE`, `DEADLINE_WEIGHT`, `DEADLINE_HORIZON_DAYS`, `QUICK_WIN_WEIGHT`, `ESTIMATE_CAP_MINUTES`, `GOAL_WEIGHT`) are in ADR 0017.
- `tasks::validate::{today() -> chrono::NaiveDate, priority, date, estimate, checklist, MAX_PRIORITY (4), MAX_ESTIMATE_MINUTES (10080), MAX_CHECKLIST_ITEMS (100), MAX_CHECKLIST_TEXT_CHARS (500)}`: the task value rules below; `today()` is the server's UTC date.
- Dates: the server depends on `chrono` (default features off; `clock`, `std`). Use `chrono::NaiveDate`/`NaiveDateTime` for task dates and `tasks::validate::today()` for "today"; later tasks need no new date dependency.
- `projects::routes() -> Router<Db>`: the project and section routes below, merged by `app::router`.
- `projects::store::{get(&Db, id) -> Result<Option<Project>, db::Error>, list(&Db, &Filter) -> Result<Vec<Project>, _>, update(&Db, id, &Changes) -> Result<Option<Project>, ApiError>, STATUSES}`: live `project` objects joined with their `projects` row (defaults when there is none) and their live task counts. `Filter { status: Option<String>, area_id: Option<i64> }` (`None` = any); `list` skips archived projects. `Changes` (also the `PATCH` body; `None` = unchanged, `Some(None)` clears) has `status, area_id, start, deadline`; the store checks every rule (`400`), upserts the row and bumps `updated`. `Project { object: Object, project: Fields }` serializes to the project JSON below.
- `projects::sections::{list(&Db, project) -> Result<Option<Vec<Section>>, db::Error>, create(&Db, project, title, Option<usize>) -> Result<Option<Section>, _>, update(&Db, id, Option<&str>, Option<usize>) -> Result<Option<Section>, _>, delete(&Db, id) -> Result<bool, _>, title(&str) -> Result<String, ApiError>, position(i64) -> Result<usize, ApiError>, MAX_TITLE_CHARS (200)}`: the `sections` table. `None`/`false` when the project (or the section's project) is not live. Every write renumbers the project's sections `0..n`.
- `smartadd::routes() -> Router<Db>`: `POST /api/parse` and `POST /api/tasks/add`, merged by `app::router`.
- `smartadd::add(&Db, text: &str, today: NaiveDate) -> Result<Task, ApiError>`: the smart-add capture path every channel shares (Telegram in M10). Blocking: it parses `text` with `polykov_parse::default_parser()` as of `today` (the user's local date) and creates the task with `tasks::store::create`; the `400`s are those of `POST /api/tasks/add`, and on any of them nothing is created.
- `smartadd::line::{read(&Db, text, today) -> Result<Line, ApiError>, Line { parsed: polykov_parse::Parsed, resolved: Vec<bool>, project_id, area_id: Option<i64> }, MAX_TEXT_CHARS (1000)}`: one parsed line. The parser's `Context` holds the titles of live, non-archived projects and areas; `resolved[i]` is `false` only for a `+project`/`@area` token naming none of them. The parser is built once per thread.
- `query::{parse(&str) -> Result<Query, ParseError>, Query { terms: Vec<Term>, archived: Archived }, Term { negated, filter: Filter }, ParseError { message, offset }}`: the pure view query parser (ADR 0015). Terms are whitespace-separated and ANDed, `-` negates one, and `"quotes"` keep spaces, `:` and `,` literal. `field:a,b` is OR within the field. A term without an unquoted `:` is `Filter::Text`. The fields are `type`, `tag` (`t` or `t/*` subtree), `status:open|done`, `priority` 1–4, the dates `due|scheduled|created|updated|completed`, `project`/`area` (id or `"title"`), `pinned:true|false`, and `archived:true|false|any`, which sets `Query.archived` (default `No`) instead of adding a term. `priority` and the dates take `=` (implicit), `<`, `<=`, `>`, `>=` per item, and the other fields take no operator. A date is `today|tomorrow|yesterday|YYYY-MM-DD|[+-]N(d|w|m)|none`. Relative dates stay `DateValue::Relative(n, Unit)` until `DateValue::resolve(today) -> Option<NaiveDate>`. `ParseError.offset` is a char index into the input.
- `search::routes() -> Router<Db>`: `GET /api/search`, merged by `app::router`.
- `search::matching_ids(&rusqlite::Connection, text: &str) -> Result<Vec<i64>, db::Error>` (also `search::store::matching_ids`): the ids of every live object (archived and completed included) that matches `text` under the rules of `GET /api/search`, best first; empty when `text` has no letters or digits. For filtering by text inside another query (views, the palette), e.g. `... AND id IN (SELECT value FROM json_each(?))` bound to the ids as a JSON array. Call it with the connection you already hold (`Db::conn`).
- `search::store::{search(&Db, &Query) -> Result<Page, db::Error>, Query { text, type: Option<String>, limit: u32, offset: usize }, Page { hits: Vec<Hit>, total: usize }, Hit { id, score: f64, snippet }}`: one page of ranked hits; `Query` must already be validated. `search::fuzzy::{words, find, snippet, fold}`: the word splitting, per-text matching and snippet rules (ADR 0014). `search::{MAX_QUERY_CHARS (200), DEFAULT_LIMIT (20), MAX_LIMIT (100)}`.
- `config::load(Flags, env: impl Fn(&str) -> Option<String>) -> Result<Config, config::Error>`: prepares the data dir and merges all sources. `Config { data_dir, bind, web_dir }`; `Flags` holds the same fields as `Option`s. `config::Error` displays a one-line message.

## HTTP
### Authentication
A request is authenticated by a session token or an API token (see `/api/tokens`), taken from `Authorization: Bearer <token>` (scheme case-insensitive) if that header is present, otherwise from the `polykov_session` cookie (`auth::COOKIE_NAME`). A present but malformed or invalid bearer header is rejected without looking at the cookie.

A route requires auth by taking `auth::Session` as a handler argument; that is the whole change:
```rust
async fn handler(State(db): State<Db>, session: auth::Session) -> Response { ... }
```
A missing, unknown, expired or revoked token gets `401 {"error":"authentication required"}` and the handler never runs; a store failure gets a bare `500`. Every route except `/health`, `GET`/`POST /api/setup`, `POST /api/login` and the web UI must take it.

### `GET /health`
Public. `200` when the database answers:
```json
{"status":"ok","version":"<crate version>","db":"ok"}
```
`503` when the database query fails:
```json
{"status":"error","version":"<crate version>","db":"error"}
```

### `GET /api/setup`
Public. `200 {"needed":true}` until the owner exists, then `200 {"needed":false}`.

### `POST /api/setup`
Public, works once. Body `{"password":"..."}`. Creates the owner with an argon2id hash of the password.
- `201`, empty body: owner created.
- `400 {"error":"password must be at least 12 characters"}`: fewer than 12 characters (Unicode chars, not bytes).
- `409 {"error":"owner already exists"}`: setup already done; nothing changes.

### `POST /api/login`
Public. Body `{"password":"..."}`.
- `200 {"token":"<64 hex chars>"}` with `Set-Cookie: polykov_session=<token>; HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000`. The token is sent by the CLI as a bearer token; the web uses the cookie. Each login starts a new 30-day session.
- `401`, empty body and no cookie: wrong password, or no owner yet.

### `GET /api/me`
Auth required. `200 {"owner":true,"session_expires":"<RFC 3339 UTC>"}`, e.g. `"2026-11-28T09:15:00Z"`; `"session_expires":null` when called with an API token. `401` as in Authentication.

### `POST /api/logout`
Auth required, no body. Deletes the caller's session (other sessions stay valid) and returns `204`, empty body, with `Set-Cookie: polykov_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0`. The token then gets `401` everywhere. Called with an API token it is a no-op (still `204`): API tokens are only revoked through `DELETE /api/tokens/{id}`. `401` as in Authentication, with no cookie change.

### `POST /api/tokens`
Auth required. Body `{"name":"..."}`. Creates a named API token that authenticates exactly like a session token (bearer header or cookie) but never expires. Names are trimmed and need not be unique.
- `201 {"id":<integer>,"name":"<trimmed name>","token":"<64 hex chars>"}`. The plain token appears only in this response; only its SHA-256 hash is stored.
- `400 {"error":"name must be 1 to 64 characters"}`: empty or whitespace-only name, or more than 64 characters (Unicode chars, not bytes) after trimming.
- `401` as in Authentication.

### `GET /api/tokens`
Auth required. `200` with all API tokens, oldest first, never the token or its hash:
```json
[{"id":1,"name":"backup script","created":"2026-10-29T09:15:00Z","last_used":null}]
```
`last_used` (RFC 3339 UTC) is updated on every authenticated request made with the token; `null` until first use. `401` as in Authentication.

### `DELETE /api/tokens/{id}`
Auth required. Revokes the token; it then gets `401` everywhere (a token may revoke itself).
- `204`, empty body: revoked.
- `404 {"error":"token not found"}`: no token with that id. A non-integer id gets axum's standard `400`.
- `401` as in Authentication.

### Objects
All object routes require auth (`401` as in Authentication). An object (spec §3) is:
```json
{"id":1,"type":"note","title":"Groceries","created":"2026-10-29T09:15:00Z","updated":"2026-10-29T09:15:00Z","archived":false,"pinned":false,"tags":["shop/food"],"properties":{"aisle":3}}
```
Ids are integers that are never reused. `created`/`updated` are RFC 3339 UTC, to the second. `tags` is sorted. Rules, each failure a `400 {"error":"..."}`:
- `type`: one of `task project area goal milestone habit note notebook journal_entry daily_note review event time_entry view forecast_record` (`type must be one of: task, project, ...`).
- `title`: trimmed, then 1 to 1000 characters (`title must be 1 to 1000 characters`).
- `properties`: a JSON object (`properties must be an object`; `null` included) of at most 100 keys (`properties must have at most 100 keys`), each 1 to 64 characters (`property names must be 1 to 64 characters`), whose values are a string, number, bool, null or an array of those (`property "<key>" must be a string, number, bool, null or an array of those`).
- `tags`: an array of hierarchical tags (`work/clienta/design` lives in the folders `work` and `work/clienta`). Each is trimmed, one leading `#` is stripped and it is lowercased; the result is at most 100 characters of `/`-separated segments, each non-empty with no whitespace, `#` or `,` (`invalid tag: <input as sent>`). Duplicates after normalizing collapse into one; then at most 50 may remain (`an object can have at most 50 tags`).

Unknown body fields, a missing required field, a wrong JSON type or an explicit `null` for `title`/`pinned`/`archived`/`tags` get axum's standard `422`. A non-integer `{id}` gets `400`. An unknown id, or one in the trash, gets `404 {"error":"object not found"}`.

#### `GET /api/objects`
Query parameters, all optional: `type=<type>`, `tag=<tag>` (normalized like a tag; matches that tag and every tag under it: `tag=work` matches `work` and `work/clienta/design`, not `workshop`), `pinned=true|false`, `archived=false|true|any` (default `false`), `limit=1..1000` (default 100), `offset=<n ≥ 0>` (default 0). `200 {"objects":[...],"total":<n>}`: live objects only, pinned first, then `updated` newest first, then `id` highest first; `total` counts all matches regardless of paging. A bad value gets `400 {"error":"..."}` (e.g. `limit must be 1 to 1000`, `archived must be false, true or any`); an unknown parameter gets axum's standard `400`.

#### `POST /api/objects`
Body `{"type":"...","title":"...","properties":{...},"pinned":false,"archived":false,"tags":[...]}`; `properties` defaults to `{}`, the flags to `false`, `tags` to `[]`. `201` with the object.

#### `GET /api/objects/{id}`
`200` with the object.

#### `PATCH /api/objects/{id}`
Body with any of `title`, `properties`, `pinned`, `archived`, `tags` (`type` cannot change). `properties` replaces the whole map and `tags` the whole set. Any given field sets `updated` to now; `{}` changes nothing. `200` with the object.

#### `DELETE /api/objects/{id}`
Moves the object to the trash (sets `deleted_at`); it then gets `404` everywhere except under `/api/trash`. `204`, empty body.

### Links
All link routes require auth (`401` as in Authentication). Links go from one object to another (spec §3); backlinks are the links pointing at an object. Both ids must be live objects, else `404 {"error":"object not found"}`; non-integer ids get `400`. Links to or from an object in the trash are hidden, and reappear when it is restored.

#### `PUT /api/objects/{id}/links/{to}`
Links `id` to `to`. `204`, empty body; linking again is a no-op `204`. `400 {"error":"cannot link an object to itself"}` when `id` = `to`.

#### `DELETE /api/objects/{id}/links/{to}`
Removes that link. `204`, empty body, or `404 {"error":"link not found"}`.

#### `GET /api/objects/{id}/links`
`200`, each side ordered by title:
```json
{"links":[{"id":2,"type":"task","title":"Write copy"}],"backlinks":[{"id":3,"type":"note","title":"Ideas"}]}
```

### Comments
All comment routes require auth (`401` as in Authentication). A comment is:
```json
{"id":1,"object_id":2,"body":"looks good","created":"2026-10-29T09:15:00Z","updated":"2026-10-29T09:15:00Z"}
```
`body` is trimmed, then must be 1 to 10000 characters, else `400 {"error":"body must be 1 to 10000 characters"}`. Unknown fields or a missing `body` get axum's standard `422`.

#### `POST /api/objects/{id}/comments`
Body `{"body":"..."}`. `201` with the comment. An unknown or trashed object gets `404 {"error":"object not found"}`.

#### `GET /api/objects/{id}/comments`
`200 {"comments":[...]}`, oldest first. `404` as for `POST`.

#### `PATCH /api/comments/{cid}`
Body `{"body":"..."}`. Replaces the body and sets `updated` to now. `200` with the comment.

#### `DELETE /api/comments/{cid}`
`204`, empty body.

For `PATCH` and `DELETE`, an unknown comment, or one on an object in the trash, gets `404 {"error":"comment not found"}`.

### Attachments
All attachment routes require auth (`401` as in Authentication). Files are stored as `<data dir>/attachments/<id>`; metadata in `attachments`. See ADR 0008. An attachment is:
```json
{"id":1,"object_id":7,"name":"résumé.pdf","mime":"application/pdf","size":5120,"created":"2026-10-29T09:15:00Z"}
```
A non-integer `{id}`/`{aid}` gets `400`. Unknown or trashed objects get `404 {"error":"object not found"}`; an unknown attachment, or one whose object is in the trash, gets `404 {"error":"attachment not found"}`.

#### `POST /api/objects/{id}/attachments?name=<file name>`
The body is the raw file (no multipart, no JSON). `mime` is the request `Content-Type` as sent, default `application/octet-stream`. `201` with the attachment.
- `400 {"error":"name must be 1 to 255 characters with no slashes or control characters"}`: `name` missing, empty, over 255 characters (Unicode chars), or containing `/`, `\`, NUL or another control character.
- `400 {"error":"Content-Type must be ASCII"}`: a `Content-Type` that is not visible ASCII.
- `413 {"error":"attachment must be at most 25 MiB"}`: body over 26214400 bytes.
- `500`: the file could not be written; no attachment is recorded.

#### `GET /api/objects/{id}/attachments`
`200 {"attachments":[...]}`, oldest first.

#### `GET /api/attachments/{aid}[?inline=1]`
`200` with the file bytes, `Content-Type: <mime>`, `Content-Disposition: attachment; filename*=UTF-8''<percent-encoded name>` and `X-Content-Type-Options: nosniff`. `?inline=1` sends `inline` instead, but only for `image/*` other than `image/svg+xml` (for `<img>` previews); otherwise it is ignored.

#### `DELETE /api/attachments/{aid}`
Deletes the row and the file. `204`, empty body.

### `GET /api/tags`
Auth required. `200` with every tag folder in use by a live, non-archived object, implicit parent folders included, sorted by tag; `count` is the number of such objects carrying that tag or one under it (each object once):
```json
{"tags":[{"tag":"work","count":2},{"tag":"work/clienta","count":1}]}
```

### Trash
All trash routes require auth (`401` as in Authentication). Objects deleted with `DELETE /api/objects/{id}` stay in the trash for 30 days, then the server deletes them for good (checked at start-up and every hour). A non-integer `{id}` gets `400`; an id that is unknown or live gets `404 {"error":"not in trash"}`.

#### `GET /api/trash`
`200 {"objects":[...]}`, newest deleted first. Each is the object JSON plus `deleted` and `purge_at` (`deleted` + 30 days), RFC 3339 UTC:
```json
{"id":1,"type":"note","title":"Groceries",...,"deleted":"2026-10-29T09:15:00Z","purge_at":"2026-11-28T09:15:00Z"}
```

#### `POST /api/trash/{id}/restore`
Takes the object out of the trash, with its tags, links, comments and attachments. `200` with the object.

#### `DELETE /api/trash/{id}`
Deletes the object for good: its attachment files, then its row and its tags, links, comments and attachment rows. `204`, empty body.

#### `DELETE /api/trash`
Deletes every object in the trash for good. `200 {"purged":<n>}`.

### Tasks
All task routes require auth (`401` as in Authentication). A task is the object JSON (type `task`) plus its task fields:
```json
{"id":5,"type":"task","title":"Write copy",...,"task":{"status":"open","priority":4,"scheduled":null,"deadline":"2026-11-03T17:30","estimate":90,"someday":false,"parent_id":null,"project_id":2,"section_id":null,"area_id":null,"checklist":[{"text":"draft","done":false}],"recurrence":null,"completed":null}}
```
An object of type `task` made with `POST /api/objects` reads with these defaults. `completed` is RFC 3339 UTC, set while `status` is `done`. Rules, each failure a `400 {"error":"..."}`:
- `priority`: 1 to 4, 1 highest; 4 means "no priority" (`priority must be 1 to 4`).
- `scheduled`, `deadline`: a real date `YYYY-MM-DD` or a floating local time `YYYY-MM-DDTHH:MM`, no zone (`deadline must be a date YYYY-MM-DD or a local time YYYY-MM-DDTHH:MM`).
- `estimate`: whole minutes, 1 to 10080 (`estimate must be 1 to 10080 minutes`).
- `parent_id`: a live task (`parent must be a live task`), not the task itself or one of its sub-tasks (`a task cannot be its own ancestor`).
- `project_id`: a live `project` (`project must be a live project`); `area_id`: a live `area` (`area must be a live area`).
- `section_id`: a section of the task's project (`section is not in this project`). Setting a different `project_id` without `section_id` clears the section.
- `recurrence`: `{"rule":"FREQ=WEEKLY;BYDAY=SU","from":"scheduled"|"completion"}` or `null`. The rule is the `recur::Rule` subset and is stored and served in canonical form; a bad one is `invalid recurrence: <reason>` (e.g. `invalid recurrence: BYDAY needs FREQ=WEEKLY`). A bad `from` or a missing key is a `422`.
- `checklist`: at most 100 items `{"text","done"}` (`done` defaults to `false`) (`checklist must have at most 100 items`); each `text` is trimmed, then 1 to 500 characters (`checklist text must be 1 to 500 characters`).

`scheduled`, `deadline`, `estimate`, `parent_id`, `project_id`, `section_id`, `area_id` and `recurrence` may be `null` (cleared); the others may not. Unknown fields (including `status` and `completed` in a request) or a wrong JSON type get axum's standard `422`. A non-integer `{id}` gets `400`. An id that is unknown, in the trash or not a task gets `404 {"error":"task not found"}`.

#### `POST /api/tasks`
Body `{"title":"...","tags":[...],"properties":{...},"pinned":false,"task":{...}}`; all but `title` optional, `title`/`tags`/`properties` follow the object rules. `201` with the task.

#### `GET /api/tasks/{id}`
`200` with the task.

#### `PATCH /api/tasks/{id}`
Body with any of `title`, `properties`, `pinned`, `archived`, `tags` (as for objects) and `task` (any task fields). Any given field sets `updated` to now; `{}` changes nothing. `200` with the task.

#### `POST /api/tasks/{id}/complete[?today=YYYY-MM-DD]`
Sets `status` to `done` and `completed` to now. `200 {"task":{...},"next":null}`. A task already done is left as it is (still `200`, `next` `null`). `today` (default: the server's UTC date) must be a real date (`today must be a date YYYY-MM-DD`).

A recurring task also gets its `recurrence` set to `null` (so reopening and re-completing it doesn't fork the series), and `next` is a new open task made in the same transaction:
- the same `title`, `tags`, `properties`, `priority`, `estimate`, `project_id`, `section_id`, `area_id`, `parent_id` and `recurrence`; every checklist item unchecked; `someday` `false`. Not copied: `pinned`, links, comments, attachments, sub-tasks.
- `scheduled` = `recur::next(rule, anchor)`, keeping the old `scheduled`'s time of day if it had one. The anchor is, for `from: scheduled`, the scheduled date, else the deadline's date, else `today`; for `from: completion`, `today`.
- `deadline` (if any) moves by the same number of days as `scheduled` moved from the first of scheduled date / deadline date / `today`, keeping its time of day.
- A next date past year 9999 is `400 {"error":"invalid recurrence: next scheduled is out of range"}` and nothing changes.

#### `POST /api/tasks/{id}/reopen`
Sets `status` to `open` and clears `completed`. `200` with the task.

### Projects
All project and section routes require auth (`401` as in Authentication). A project is the object JSON (type `project`) plus its project fields:
```json
{"id":2,"type":"project","title":"Launch",...,"project":{"status":"active","area_id":3,"start":"2026-11-01","deadline":null,"open_tasks":4,"done_tasks":1}}
```
An object of type `project` made with `POST /api/objects` reads with the defaults (`active`, nulls); edit its title, tags and so on with `PATCH /api/objects/{id}`. `open_tasks`/`done_tasks` count the tasks (not in the trash) whose `project_id` is this project. A non-integer `{id}` gets `400`; an id that is unknown, in the trash or not a project gets `404 {"error":"project not found"}`.

#### `GET /api/projects`
Query parameters, both optional: `status=active|someday|done|any` (default `active`) and `area=<id>`. `200 {"projects":[...]}`: live, non-archived projects, by title (case-insensitive), then id. A bad value gets `400` (`status must be active, someday, done or any`, `area must be an integer id`); an unknown parameter gets axum's standard `400`.

#### `GET /api/projects/{id}`
`200` with the project (archived ones included).

#### `PATCH /api/projects/{id}`
Body with any of `status`, `area_id`, `start`, `deadline`. Each failure is a `400 {"error":"..."}`:
- `status`: `active`, `someday` or `done` (`status must be active, someday or done`).
- `area_id`: a live `area` or `null` (`area must be a live area`).
- `start`, `deadline`: a date or `null`, with the task date rules (`start must be a date YYYY-MM-DD or a local time YYYY-MM-DDTHH:MM`).

Any given field sets `updated` to now; `{}` changes nothing. Unknown fields (including object fields such as `title`), a wrong JSON type or `"status":null` get axum's standard `422`. `200` with the project.

#### Sections
A section is `{"id":1,"project_id":2,"title":"Plan","position":0}`. A project's positions are always `0..n`. `title` is trimmed, then 1 to 200 characters (`title must be 1 to 200 characters`); `position` is an integer ≥ 0 (`position must be a non-negative integer`), and one past the end means last. Unknown fields get `422`. A non-integer `{sid}` gets `400`; an unknown section, or one whose project is in the trash, gets `404 {"error":"section not found"}`.

#### `GET /api/projects/{id}/sections`
`200 {"sections":[...]}`, by position.

#### `POST /api/projects/{id}/sections`
Body `{"title":"...","position":n}`; `position` defaults to last, and an explicit one shifts the later sections down. `201` with the section.

#### `PATCH /api/sections/{sid}`
Body with `title` and/or `position`: renames and/or moves the section, shifting the ones between. `200` with the section.

#### `DELETE /api/sections/{sid}`
Deletes the section and closes the gap. Its tasks keep their project and get `section_id: null`. `204`, empty body.

### Lists
GTD lists (spec "Tasks (GTD)", "Capture"). Both routes require auth (`401` as in Authentication). A row is the task JSON plus `"subtasks":{"open":n,"total":n}`, counting its live, non-archived direct children.

#### `GET /api/lists/{name}?today=YYYY-MM-DD&project=<id>&area=<id>&sort=default|smart&limit=1..1000&offset=n`
`200 {"tasks":[...],"total":n}`: one page (`limit` default 100, `offset` default 0) and the number of matches across all pages. `today` defaults to the server's UTC date; clients send their local date. Only live, non-archived, `open` tasks are listed. A reference (`project_id`, `area_id`, `parent_id`) to a trashed object counts as none. A date-time `scheduled`/`deadline` compares by its date. The lists (defaults; the owner may change them):
- `inbox`: no project, no area, no parent, no `scheduled`, no `deadline`, not `someday`. Newest first.
- `today`: `scheduled` ≤ today or `deadline` ≤ today (overdue included), any depth. By `priority`, then `deadline` (nulls last), then `scheduled`, then `id`.
- `upcoming`: `scheduled` > today, or no `scheduled` and `deadline` > today, any depth. By that date, then `priority`, then `id`.
- `someday`: `someday`, no `scheduled`, top-level (no live parent). By `priority`, then `id`.
- `anytime`: not `someday`, not in the inbox, `scheduled` null or ≤ today, top-level. By `priority`, then `id`.
- `all`: every top-level task; with `project=` it is the project view. By `priority`, then `id`.

`sort=smart` orders any list by smart sort (ADR 0017: priority, deadline proximity to `today`, estimate; higher score first, ties by `id`), then pages. No `sort`, or `sort=default`, keeps the list's own order above; any other value gets `400 {"error":"sort must be default or smart"}`.

`project=<id>` keeps tasks in that project; `area=<id>` keeps tasks in that area or in a project whose area it is. An unknown `name` gets `404 {"error":"unknown list"}`. A bad `today` (`today must be a date YYYY-MM-DD`), `limit` (`limit must be 1 to 1000`), `offset` (`offset must be a non-negative integer`) or id (`project must be a positive integer id`) gets `400`; an unknown query parameter gets axum's standard `400`. A well-formed id that is no live project or area matches nothing.

#### `GET /api/tasks/{id}/subtasks`
`200 {"tasks":[...]}`: the live, non-archived direct children of the task, open first, then done, each by `id`. A non-integer `{id}` gets `400`; an id that is unknown, in the trash or not a task gets `404 {"error":"task not found"}`.

### Smart add
Both routes require auth (`401` as in Authentication) and take the body `{"text":"...","today":"YYYY-MM-DD"}`. `today` is the client's local date (default: the server's UTC date; a bad one is `400 {"error":"today must be a date YYYY-MM-DD"}`). `text` is at most 1000 characters (`400 {"error":"text must be at most 1000 characters"}`). Unknown fields get `422`. The syntax is `polykov-parse`'s (`parse/API.md`); `+project`/`@area` match live, non-archived projects and areas by title, ignoring case, with `-`/`_` read as spaces.

#### `POST /api/parse`
Parses without writing, for the live preview. `200`:
```json
{"title":"call mom","tokens":[{"start":9,"end":13,"kind":"date","text":"tmrw","resolved":true},...],"tags":["family"],"task":{"scheduled":"2026-10-31T17:00","deadline":null,"estimate":30,"priority":2,"recurrence":{"rule":"FREQ=WEEKLY;BYDAY=SU","from":"scheduled"},"project_id":null,"area_id":null}}
```
- `tokens` are in input order. `start`/`end` are UTF-16 code-unit offsets into `text` (JavaScript string indices); `kind` is `date time duration priority project area tag deadline recurrence`.
- `resolved` is `false` for a `+project`/`@area` naming no live project or area, else `true`.
- `task` fields are `null` when the line doesn't give them (`priority` too).

#### `POST /api/tasks/add`
Parses `text` and creates the task from the result (title, tags and the `task` fields above). `201` with the task. A line with no project, area or date lands in the Inbox. Each failure is a `400 {"error":"..."}` and creates nothing:
- `unknown project: <name>` / `unknown area: <name>` for the first unresolved `+project`/`@area`.
- `title must be 1 to 1000 characters` when nothing is left of the title after parsing.
- the tag and task rules, as for `POST /api/tasks`.

### `GET /api/search?q=<text>&type=<type>&limit=1..100&offset=n`
Requires auth (`401` as in Authentication). Full-text, fuzzy search over every live object, archived and completed included (trashed ones never match). `200`:
```json
{"results":[{"object":{"id":1,"type":"note","title":"Meeting notes",...},"snippet":"[Meet]ing notes","score":12.0}],"total":1}
```
- `q` is trimmed, then must be 1 to 200 characters (`400 {"error":"q must be 1 to 200 characters"}`). Its words are its runs of letters and digits, case-insensitive; anything else (FTS syntax included) only separates words, so a `q` with no words returns no results. An object matches when every word is found in its title, tags, string property values or comment bodies: as a substring (`ocer` finds `Groceries`), or, for a word of 6+ characters that occurs exactly in no object (of that `type`, if given), with one typo (`groceris`).
- `score` sums, per word, its best match: column weight (title 4, tags 2, properties 1, comments 1) times quality (whole word 4, word prefix 3, substring 2, typo 1). Results are best first, then newest id first; `total` counts every match.
- `object` is the object JSON, or for a task / project the task / project JSON (with `task.status` / `project.status`).
- `snippet` is the text of the column with the strongest match, on one line, with every matched word in `[` `]`, cut to about 80 characters (`…` marks a cut).
- `type=<type>` limits to one object type (bad type: `400` as for objects). `limit` 1 to 100 (default 20; else `400 {"error":"limit must be 1 to 100"}`), `offset` ≥ 0 (default 0; else `400 {"error":"offset must be a non-negative integer"}`). An unknown parameter gets axum's standard `400`.

Malformed or missing JSON bodies get axum's standard `400`/`415`/`422` rejections. Store failures return a bare `500` and log a one-line cause.

Unknown paths under `/api` (including `/api` and `/api/`) and under `/health/` (including `/health/`) return `404 {"error":"not found"}`. Every other path goes to the web UI.

### Web UI (every other path)
Public. Files are read from `web_dir` at request time (not embedded), so the server builds without a web build and a rebuilt UI is picked up without a restart.
- `GET`/`HEAD` of a file under `web_dir` returns it with a `Content-Type` guessed from its extension; `/` returns `index.html`.
- Any other path returns `200` with `index.html` (SPA fallback), including paths that try to leave `web_dir` (`..`, percent-encoded `..`, `/` or `\`): nothing outside `web_dir` is ever read.
- A directory path without a trailing slash (e.g. `/assets`) returns a bare `307` redirect to the same path with `/` appended.
- Other methods return `405`.
- If `web_dir/index.html` does not exist when the router is built, the server still starts, logs one warning, and every web path (any method) returns `503` text/plain `web UI not built: no index.html in web_dir`. Restart after building the UI. `/health` and `/api` are unaffected.
