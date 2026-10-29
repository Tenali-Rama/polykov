//! Versioned schema migrations, embedded in the binary.
//!
//! Each file `migrations/NNNN_name.sql` is listed in [`MIGRATIONS`] with its number.
//! Applied versions are recorded in the `schema_version` table, which migration 1 creates.

use rusqlite::{Connection, OptionalExtension};

use super::Error;

/// All migrations, in order. Versions start at 1 and have no gaps.
pub(super) const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../../migrations/0001_schema_version.sql")),
    (2, include_str!("../../migrations/0002_owner_sessions.sql")),
];

/// The schema version this binary migrates to.
pub fn latest() -> u32 {
    MIGRATIONS.last().map_or(0, |(v, _)| *v)
}

/// The highest recorded version, or 0 for a database that has never been migrated.
pub fn current(conn: &Connection) -> rusqlite::Result<u32> {
    let has_table = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_version'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !has_table {
        return Ok(0);
    }
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
}

/// Applies every migration newer than the recorded version, each in its own transaction.
/// Fails without changes if the database is newer than this binary.
pub fn run(conn: &mut Connection) -> Result<(), Error> {
    run_list(conn, MIGRATIONS)
}

pub(super) fn run_list(conn: &mut Connection, migrations: &[(u32, &str)]) -> Result<(), Error> {
    let binary = migrations.last().map_or(0, |(v, _)| *v);
    let db = current(conn).map_err(Error::Sqlite)?;
    if db > binary {
        return Err(Error::TooNew { db, binary });
    }
    for &(version, sql) in migrations.iter().filter(|(v, _)| *v > db) {
        let failed = |source| Error::Migration { version, source };
        let tx = conn.transaction().map_err(failed)?;
        tx.execute_batch(sql).map_err(failed)?;
        tx.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [version],
        )
        .map_err(failed)?;
        tx.commit().map_err(failed)?;
    }
    Ok(())
}
