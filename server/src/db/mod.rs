//! The SQLite database: `<data dir>/polykov.db`, opened in WAL mode with foreign keys on
//! and migrated to the latest schema on open.

pub mod migrate;
#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

/// File name of the database inside the data dir.
pub const FILE_NAME: &str = "polykov.db";

/// Shared handle to the database connection. Cheap to clone.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Opens (creating if needed) `<data_dir>/polykov.db` and applies pending migrations.
    pub fn open(data_dir: &Path) -> Result<Db, Error> {
        let path = data_dir.join(FILE_NAME);
        let at = |source| Error::Open {
            path: path.clone(),
            source,
        };
        let mut conn = Connection::open(&path).map_err(at)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .map_err(at)?;
        migrate::run(&mut conn)?;
        Ok(Db {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Reads the schema version table, so a missing or unreadable schema fails; used by `/health`.
    pub fn check(&self) -> Result<(), Error> {
        let conn = self.conn.lock().map_err(|_| Error::Poisoned)?;
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |_| Ok(()))
            .map_err(Error::Sqlite)
    }
}

#[derive(Debug)]
pub enum Error {
    /// The database file could not be opened or configured.
    Open {
        path: PathBuf,
        source: rusqlite::Error,
    },
    /// Applying a migration failed; it was rolled back.
    Migration {
        version: u32,
        source: rusqlite::Error,
    },
    /// The database was migrated by a newer binary.
    TooNew { db: u32, binary: u32 },
    /// A query failed.
    Sqlite(rusqlite::Error),
    /// A thread panicked while holding the connection.
    Poisoned,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Open { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Migration { version, source } => {
                write!(f, "database migration {version} failed: {source}")
            }
            Error::TooNew { db, binary } => write!(
                f,
                "database schema version {db} is newer than this binary supports ({binary}); upgrade polykov-server"
            ),
            Error::Sqlite(source) => write!(f, "database error: {source}"),
            Error::Poisoned => write!(f, "database connection is poisoned"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Open { source, .. }
            | Error::Migration { source, .. }
            | Error::Sqlite(source) => Some(source),
            Error::TooNew { .. } | Error::Poisoned => None,
        }
    }
}
