---
id: 003
title: Open SQLite in WAL mode with versioned automatic migrations
milestone: M0
status: todo
deps: [002]
touches: [server/src/db/, server/migrations/, server/src/main.rs, server/src/app.rs, server/Cargo.toml, Cargo.lock, server/tests/, server/API.md, docs/decisions/]
spec: ["2. Architecture & Stack", "10. Non-Functional"]
---

## Goal
The server opens `<data dir>/polykov.db` in WAL mode with FTS5 available and applies versioned migrations from `server/migrations/` automatically at start-up.

## Acceptance criteria
- [ ] On start, `polykov.db` is created in the data dir; `PRAGMA journal_mode` returns `wal` and `PRAGMA foreign_keys` returns `1`
- [ ] Migrations are numbered SQL files in `server/migrations/`, embedded in the binary, recorded in a schema version table, and applied once each, in order
- [ ] First migration creates only what the migration system itself needs (no product tables)
- [ ] Tests: fresh DB migrates to the latest version; running migrations twice is a no-op; a DB whose recorded version is newer than the binary fails start-up with a clear error
- [ ] A test proves FTS5 is available (creating an FTS5 virtual table in an in-memory DB succeeds)
- [ ] `/health` returns `{"status":"ok","version":"...","db":"ok"}` and returns 503 with `"db":"error"` if the DB query fails (tested)
- [ ] ADR in `docs/decisions/` records the SQLite crate and migration approach
- [ ] `server/API.md` documents the db module and the `/health` change

## Out of scope
- Any product tables (objects, tasks, ...), owner/session tables (004), backups (M17).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
