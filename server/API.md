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
- `auth::Session { token: String, expires_at: String }`: Axum extractor (`FromRequestParts` for any state where `Db: FromRef<S>`) that resolves the caller's session. See Authentication.
- `auth::owner::{exists, create, password_hash}` and `auth::password::{hash, verify}`: the single `owner` row and argon2id hashing (PHC strings). `auth::Error` displays a one-line message that never contains a password or token.
- `config::load(Flags, env: impl Fn(&str) -> Option<String>) -> Result<Config, config::Error>`: prepares the data dir and merges all sources. `Config { data_dir, bind, web_dir }`; `Flags` holds the same fields as `Option`s. `config::Error` displays a one-line message.

## HTTP
### Authentication
A request is authenticated by a session token, taken from `Authorization: Bearer <token>` (scheme case-insensitive) if that header is present, otherwise from the `polykov_session` cookie (`auth::COOKIE_NAME`). A present but malformed or invalid bearer header is rejected without looking at the cookie.

A route requires auth by taking `auth::Session` as a handler argument; that is the whole change:
```rust
async fn handler(State(db): State<Db>, session: auth::Session) -> Response { ... }
```
A missing, unknown or expired session gets `401 {"error":"authentication required"}` and the handler never runs; a store failure gets a bare `500`. Every route except `/health`, `GET`/`POST /api/setup`, `POST /api/login` and the web UI must take it.

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
Auth required. `200 {"owner":true,"session_expires":"<RFC 3339 UTC>"}`, e.g. `"2026-11-28T09:15:00Z"`. `401` as in Authentication.

### `POST /api/logout`
Auth required, no body. Deletes the caller's session (other sessions stay valid) and returns `204`, empty body, with `Set-Cookie: polykov_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0`. The token then gets `401` everywhere. `401` as in Authentication, with no cookie change.

Malformed or missing JSON bodies get axum's standard `400`/`415`/`422` rejections. Store failures return a bare `500` and log a one-line cause.

Unknown paths under `/api` (including `/api` and `/api/`) and under `/health/` (including `/health/`) return `404 {"error":"not found"}`. Every other path goes to the web UI.

### Web UI (every other path)
Public. Files are read from `web_dir` at request time (not embedded), so the server builds without a web build and a rebuilt UI is picked up without a restart.
- `GET`/`HEAD` of a file under `web_dir` returns it with a `Content-Type` guessed from its extension; `/` returns `index.html`.
- Any other path returns `200` with `index.html` (SPA fallback), including paths that try to leave `web_dir` (`..`, percent-encoded `..`, `/` or `\`): nothing outside `web_dir` is ever read.
- A directory path without a trailing slash (e.g. `/assets`) returns a bare `307` redirect to the same path with `/` appended.
- Other methods return `405`.
- If `web_dir/index.html` does not exist when the router is built, the server still starts, logs one warning, and every web path (any method) returns `503` text/plain `web UI not built: no index.html in web_dir`. Restart after building the UI. `/health` and `/api` are unaffected.
