---
id: 003
title: Open SQLite in WAL mode with versioned automatic migrations
milestone: M0
status: done
deps: [002]
touches: [server/src/db/, server/migrations/, server/src/main.rs, server/src/app.rs, server/Cargo.toml, Cargo.lock, server/tests/, server/API.md, docs/decisions/]
spec: ["2. Architecture & Stack", "10. Non-Functional"]
---

## Goal
The server opens `<data dir>/polykov.db` in WAL mode with FTS5 available and applies versioned migrations from `server/migrations/` automatically at start-up.

## Acceptance criteria
- [x] On start, `polykov.db` is created in the data dir; `PRAGMA journal_mode` returns `wal` and `PRAGMA foreign_keys` returns `1`
- [x] Migrations are numbered SQL files in `server/migrations/`, embedded in the binary, recorded in a schema version table, and applied once each, in order
- [x] First migration creates only what the migration system itself needs (no product tables)
- [x] Tests: fresh DB migrates to the latest version; running migrations twice is a no-op; a DB whose recorded version is newer than the binary fails start-up with a clear error
- [x] A test proves FTS5 is available (creating an FTS5 virtual table in an in-memory DB succeeds)
- [x] `/health` returns `{"status":"ok","version":"...","db":"ok"}` and returns 503 with `"db":"error"` if the DB query fails (tested)
- [x] ADR in `docs/decisions/` records the SQLite crate and migration approach
- [x] `server/API.md` documents the db module and the `/health` change

## Out of scope
- Any product tables (objects, tasks, ...), owner/session tables (004), backups (M17).

## Notes for next
- `app::router` now takes a `db::Db`; build state from `Db::open(data_dir)` (tests: open one in a temp dir, see `server/tests/health.rs`).
- New migration: add `server/migrations/NNNN_<name>.sql` and append it to `MIGRATIONS` in `server/src/db/migrate.rs`; `migration_list_matches_migrations_dir` enforces both. Never edit merged migrations.
- `Db` is one `rusqlite::Connection` behind a `Mutex`; call it from handlers inside `tokio::task::spawn_blocking`.
- Default chosen: on DB failure `/health` returns `"status":"error"` as well as `"db":"error"`.
- Tables use `STRICT`; `foreign_keys` is on per connection, so any new connection must set it too.
