# server

`polykov-server`: Axum + Tokio HTTP server. Spec §2.

## Layout
- `src/lib.rs`: crate `polykov_server`, used by the binary, `tests/`, and the CLI's tests.
- `src/app.rs`: `router(db, web_dir)`, which merges each feature's routes. Add a feature's `.merge(...)` line here.
- `src/<feature>/` (e.g. `tasks/`, `links/`): one module per feature, with `mod.rs` exposing `pub fn routes() -> Router<Db>` plus its handlers and queries.
- `src/db/`: `mod.rs` opens the SQLite connection (`Db`, WAL, foreign keys) and migrates on open; `migrate.rs` applies the versioned migrations. To add one, create `migrations/NNNN_name.sql` (next number, no gaps) and append `(N, include_str!(...))` to `MIGRATIONS` in `migrate.rs`.
- `src/config/`: data dir layout and settings (`config.toml` + env + flags). New settings go in `file.rs` (struct and `DEFAULT` template), `mod.rs` (merge), `main.rs` (flag) and `API.md`.
- `src/main.rs`: argument parsing and port binding only. No app logic.
- `tests/`: integration tests that drive `app::router(db, web_dir)` with `tower::ServiceExt::oneshot`, no port needed.

## Rules
- No `unwrap`/`expect` outside tests.
- Every new endpoint goes in `API.md` and gets an integration test.
