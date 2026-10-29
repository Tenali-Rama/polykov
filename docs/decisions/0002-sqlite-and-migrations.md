# 0002 — SQLite crate and migration approach

- **Status**: accepted
- **Context**: The server needs SQLite with WAL and FTS5 on any host, and versioned migrations that run automatically on upgrade (spec §2, §10).
- **Decision**: Use `rusqlite` with the `bundled` feature (SQLite compiled in, FTS5 enabled, no system lib needed). Migrations are a small in-house runner: numbered files `server/migrations/NNNN_<name>.sql`, embedded with `include_str!` in `db::migrate::MIGRATIONS`, applied in order, one transaction each, and recorded in `schema_version`, which migration 1 creates. A DB newer than the binary refuses to start. Access goes through `db::Db` (one connection behind a mutex; blocking calls run via `spawn_blocking`).
- **Consequences**: No ORM or async driver and no extra migration crate. Merged migrations are immutable; add a new one instead. No down-migrations: restore from backup to roll back. If one connection becomes a bottleneck, a pool can replace the mutex inside `Db` without changing callers.
