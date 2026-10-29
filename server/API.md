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
- `objects::store::{create(&Db, &New) -> Result<Object, db::Error>, get(&Db, id) -> Result<Option<Object>, _>, update(&Db, id, &Changes) -> Result<Option<Object>, _>, trash(&Db, id) -> Result<bool, _>}`: the `objects` table. Reads and writes only see live objects (`deleted_at IS NULL`). `New { type, title, properties, pinned, archived }` and `Changes` (all `Option`, `None` = unchanged) must already be validated with `objects::validate::{object_type, title, properties}`. `Object` serializes to the object JSON below.
- `objects::store::summary(&Db, id) -> Result<Option<Summary>, db::Error>`: `Summary { id, type, title }` of a live object, `None` if unknown or in the trash. Links, comments and attachments use it to check their target.
- `objects::validate::{TYPES, MAX_TITLE_CHARS (1000), MAX_PROPERTIES (100), MAX_KEY_CHARS (64)}`: the object input rules below.
- `links::routes() -> Router<Db>` and `comments::routes() -> Router<Db>`: the link and comment routes below, merged by `app::router`.
- `links::store::{add(&Db, src, dst) -> Result<(), db::Error>, remove(&Db, src, dst) -> Result<bool, _>, list(&Db, id) -> Result<Links, _>}`: the `links` table. Callers check both ends with `objects::store::summary`. `Links { links, backlinks }` hold `Summary`s of live objects, by title.
- `comments::store::{create(&Db, object_id, body) -> Result<Comment, db::Error>, list(&Db, object_id) -> Result<Vec<Comment>, _>, update(&Db, id, body) -> Result<Option<Comment>, _>, delete(&Db, id) -> Result<bool, _>}`: the `comments` table; comments on trashed objects are invisible to `update`/`delete`. `body` must already be trimmed and 1 to `comments::MAX_BODY_CHARS` (10000) characters. `Comment` serializes to the comment JSON below.
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
{"id":1,"type":"note","title":"Groceries","created":"2026-10-29T09:15:00Z","updated":"2026-10-29T09:15:00Z","archived":false,"pinned":false,"tags":[],"properties":{"aisle":3}}
```
Ids are integers that are never reused. `created`/`updated` are RFC 3339 UTC, to the second. `tags` is always `[]` for now (task 014). Rules, each failure a `400 {"error":"..."}`:
- `type`: one of `task project area goal milestone habit note notebook journal_entry daily_note review event time_entry view forecast_record` (`type must be one of: task, project, ...`).
- `title`: trimmed, then 1 to 1000 characters (`title must be 1 to 1000 characters`).
- `properties`: a JSON object (`properties must be an object`; `null` included) of at most 100 keys (`properties must have at most 100 keys`), each 1 to 64 characters (`property names must be 1 to 64 characters`), whose values are a string, number, bool, null or an array of those (`property "<key>" must be a string, number, bool, null or an array of those`).

Unknown body fields, a missing required field, a wrong JSON type or an explicit `null` for `title`/`pinned`/`archived` get axum's standard `422`. A non-integer `{id}` gets `400`. An unknown id, or one in the trash, gets `404 {"error":"object not found"}`.

#### `POST /api/objects`
Body `{"type":"...","title":"...","properties":{...},"pinned":false,"archived":false}`; `properties` defaults to `{}`, the flags to `false`. `201` with the object.

#### `GET /api/objects/{id}`
`200` with the object.

#### `PATCH /api/objects/{id}`
Body with any of `title`, `properties`, `pinned`, `archived` (`type` cannot change). `properties` replaces the whole map. Any given field sets `updated` to now; `{}` changes nothing. `200` with the object.

#### `DELETE /api/objects/{id}`
Moves the object to the trash (sets `deleted_at`); it then gets `404` everywhere. `204`, empty body.

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

Malformed or missing JSON bodies get axum's standard `400`/`415`/`422` rejections. Store failures return a bare `500` and log a one-line cause.

Unknown paths under `/api` (including `/api` and `/api/`) and under `/health/` (including `/health/`) return `404 {"error":"not found"}`. Every other path goes to the web UI.

### Web UI (every other path)
Public. Files are read from `web_dir` at request time (not embedded), so the server builds without a web build and a rebuilt UI is picked up without a restart.
- `GET`/`HEAD` of a file under `web_dir` returns it with a `Content-Type` guessed from its extension; `/` returns `index.html`.
- Any other path returns `200` with `index.html` (SPA fallback), including paths that try to leave `web_dir` (`..`, percent-encoded `..`, `/` or `\`): nothing outside `web_dir` is ever read.
- A directory path without a trailing slash (e.g. `/assets`) returns a bare `307` redirect to the same path with `/` appended.
- Other methods return `405`.
- If `web_dir/index.html` does not exist when the router is built, the server still starts, logs one warning, and every web path (any method) returns `503` text/plain `web UI not built: no index.html in web_dir`. Restart after building the UI. `/health` and `/api` are unaffected.
