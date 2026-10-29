//! The single owner row in the `owner` table.

use rusqlite::OptionalExtension;

use super::Error;
use crate::db::{self, Db};

/// Whether the owner account has been created.
pub fn exists(db: &Db) -> Result<bool, Error> {
    Ok(password_hash(db)?.is_some())
}

/// Creates the owner with `password_hash`. Returns `false`, changing nothing, if one exists.
pub fn create(db: &Db, password_hash: &str) -> Result<bool, Error> {
    let conn = db.conn()?;
    let added = conn
        .execute(
            "INSERT OR IGNORE INTO owner (id, password_hash) VALUES (1, ?1)",
            [password_hash],
        )
        .map_err(db::Error::Sqlite)?;
    Ok(added == 1)
}

/// The owner's stored password hash, or `None` before setup.
pub fn password_hash(db: &Db) -> Result<Option<String>, Error> {
    let conn = db.conn()?;
    let hash = conn
        .query_row("SELECT password_hash FROM owner WHERE id = 1", [], |r| {
            r.get(0)
        })
        .optional()
        .map_err(db::Error::Sqlite)?;
    Ok(hash)
}
