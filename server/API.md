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
- `app::router(db: db::Db) -> axum::Router`: the full HTTP app over `db`, with no bound port.
- `db::Db::open(data_dir: &Path) -> Result<Db, db::Error>`: opens `data_dir/polykov.db` (`db::FILE_NAME`), sets the pragmas and migrates. `Db` is a cheap-to-clone handle to one connection behind a mutex.
- `db::Db::check(&self) -> Result<(), db::Error>`: reads `schema_version`; what `/health` reports as `db`.
- `db::migrate::{latest() -> u32, current(&Connection) -> rusqlite::Result<u32>, run(&mut Connection) -> Result<(), db::Error>}`: the binary's latest version, a DB's recorded version (0 if never migrated), and apply pending migrations.
- `db::Error` displays a one-line message.
- `auth::routes() -> Router<Db>`: the setup and login routes below, merged by `app::router`.
- `auth::session::create(&Db) -> Result<String, auth::Error>`: starts a session expiring `session::TTL_SECS` (30 days) from now and returns its token (64 hex chars, 256 random bits). Only `session::hash_token(token)` (hex SHA-256) is stored, in `sessions (token_hash, created_at, expires_at)`, times in Unix seconds.
- `auth::session::is_valid(&Db, token: &str) -> Result<bool, auth::Error>`: whether `token` is a known, unexpired session. Use this to check bearer tokens and the `auth::COOKIE_NAME` (`polykov_session`) cookie.
- `auth::owner::{exists, create, password_hash}` and `auth::password::{hash, verify}`: the single `owner` row and argon2id hashing (PHC strings). `auth::Error` displays a one-line message that never contains a password or token.
- `config::load(Flags, env: impl Fn(&str) -> Option<String>) -> Result<Config, config::Error>`: prepares the data dir and merges all sources. `Config { data_dir, bind, web_dir }`; `Flags` holds the same fields as `Option`s. `config::Error` displays a one-line message.

## HTTP
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

Malformed or missing JSON bodies get axum's standard `400`/`415`/`422` rejections. Store failures return a bare `500` and log a one-line cause.

Unknown paths return `404` with an empty body.
