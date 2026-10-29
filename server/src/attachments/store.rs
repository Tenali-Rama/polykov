//! The `attachments` table and the files in `<data dir>/attachments/`, one per row, named by
//! the attachment id. Rows and files are written and removed together.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::db::{self, Db};

/// Name of the attachments dir inside the data dir.
pub const DIR_NAME: &str = "attachments";

/// An attachment as served over the API. `created` is RFC 3339 UTC.
#[derive(Debug, Serialize)]
pub struct Attachment {
    pub id: i64,
    pub object_id: i64,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub created: String,
}

/// A store failure: the database, or a file under the attachments dir.
#[derive(Debug)]
pub enum Error {
    Db(db::Error),
    Io(PathBuf, io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Db(e) => e.fmt(f),
            Error::Io(path, e) => write!(f, "{}: {e}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

impl From<db::Error> for Error {
    fn from(e: db::Error) -> Self {
        Error::Db(e)
    }
}

fn sql(e: rusqlite::Error) -> Error {
    Error::Db(db::Error::Sqlite(e))
}

const COLUMNS: &str = "a.id, a.object_id, a.name, a.mime, a.size, \
     strftime('%Y-%m-%dT%H:%M:%SZ', a.created, 'unixepoch')";

/// Path of the file holding attachment `id`.
pub fn file_path(db: &Db, id: i64) -> PathBuf {
    db.data_dir().join(DIR_NAME).join(id.to_string())
}

/// Stores `bytes` as a new attachment of the live object `object_id`, or returns `None` if it
/// is unknown or in the trash. The file is written to a temp file and renamed into place; if
/// that fails, no row is kept.
pub fn create(
    db: &Db,
    object_id: i64,
    name: &str,
    mime: &str,
    bytes: &[u8],
) -> Result<Option<Attachment>, Error> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(sql)?;
    let size = i64::try_from(bytes.len()).unwrap_or(i64::MAX);
    let inserted = tx
        .execute(
            "INSERT INTO attachments (object_id, name, mime, size) SELECT id, ?2, ?3, ?4 \
             FROM objects WHERE id = ?1 AND deleted_at IS NULL",
            params![object_id, name, mime, size],
        )
        .map_err(sql)?;
    if inserted == 0 {
        return Ok(None);
    }
    let id = tx.last_insert_rowid();
    let path = file_path(db, id);
    write_file(&path, bytes)?;
    let attachment = select(&tx, id);
    match attachment.and_then(|a| tx.commit().map(|()| a).map_err(sql)) {
        Ok(a) => Ok(a),
        Err(e) => {
            let _ = fs::remove_file(&path);
            Err(e)
        }
    }
}

/// Writes `bytes` to `<path>.tmp`, syncs it and renames it to `path`. Removes the temp file on
/// failure.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let tmp = path.with_extension("tmp");
    let result = (|| {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Error::Io(path.to_path_buf(), e)
    })
}

/// The attachments of the live object `object_id`, oldest first, or `None` if it is unknown
/// or in the trash.
pub fn list(db: &Db, object_id: i64) -> Result<Option<Vec<Attachment>>, Error> {
    let conn = db.conn()?;
    let live: Option<i64> = conn
        .query_row(
            "SELECT id FROM objects WHERE id = ?1 AND deleted_at IS NULL",
            [object_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql)?;
    if live.is_none() {
        return Ok(None);
    }
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM attachments a WHERE a.object_id = ?1 ORDER BY a.id"
        ))
        .map_err(sql)?;
    let rows = stmt.query_map([object_id], attachment).map_err(sql)?;
    rows.collect::<Result<_, _>>().map(Some).map_err(sql)
}

/// Attachment `id`, or `None` if it is unknown or its object is in the trash.
pub fn get(db: &Db, id: i64) -> Result<Option<Attachment>, Error> {
    select(&*db.conn()?, id)
}

/// Deletes attachment `id` and its file. Returns `false` (and changes nothing) if it is
/// unknown or its object is in the trash.
pub fn delete(db: &Db, id: i64) -> Result<bool, Error> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(sql)?;
    if select(&tx, id)?.is_none() {
        return Ok(false);
    }
    tx.execute("DELETE FROM attachments WHERE id = ?1", [id])
        .map_err(sql)?;
    remove_file(&file_path(db, id))?;
    tx.commit().map_err(sql)?;
    Ok(true)
}

/// Deletes the files of every attachment of `object_id`, live or in the trash, leaving the
/// rows (purging the object cascades to them). For the trash purge. A missing file is not an
/// error.
pub fn remove_files(db: &Db, object_id: i64) -> Result<(), Error> {
    let ids: Vec<i64> = {
        let conn = db.conn()?;
        let mut stmt = conn
            .prepare("SELECT id FROM attachments WHERE object_id = ?1")
            .map_err(sql)?;
        let rows = stmt.query_map([object_id], |r| r.get(0)).map_err(sql)?;
        rows.collect::<Result<_, _>>().map_err(sql)?
    };
    ids.into_iter()
        .try_for_each(|id| remove_file(&file_path(db, id)))
}

fn remove_file(path: &Path) -> Result<(), Error> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(Error::Io(path.to_path_buf(), e)),
        _ => Ok(()),
    }
}

fn select(conn: &Connection, id: i64) -> Result<Option<Attachment>, Error> {
    conn.query_row(
        &format!(
            "SELECT {COLUMNS} FROM attachments a JOIN objects o ON o.id = a.object_id \
             WHERE a.id = ?1 AND o.deleted_at IS NULL"
        ),
        [id],
        attachment,
    )
    .optional()
    .map_err(sql)
}

fn attachment(r: &Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: r.get(0)?,
        object_id: r.get(1)?,
        name: r.get(2)?,
        mime: r.get(3)?,
        size: r.get(4)?,
        created: r.get(5)?,
    })
}
