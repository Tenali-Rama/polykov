//! Objects in the trash (`deleted_at` set): list, restore and delete for good. Deleting for
//! good removes the attachment files first, then the row, which cascades to its side tables.

use rusqlite::{OptionalExtension, Row, ToSql};
use serde::Serialize;

use crate::attachments::{self, store::Error};
use crate::db::{self, Db};
use crate::objects::store::{self as objects, Object};

/// How long an object stays in the trash before it is purged, in seconds (30 days).
pub const RETENTION_SECS: i64 = 30 * 24 * 60 * 60;

/// A trashed object: the usual object JSON plus when it was deleted and when it will be
/// purged, both RFC 3339 UTC.
#[derive(Debug, Serialize)]
pub struct Trashed {
    #[serde(flatten)]
    pub object: Object,
    pub deleted: String,
    pub purge_at: String,
}

fn sql(e: rusqlite::Error) -> db::Error {
    db::Error::Sqlite(e)
}

/// Every object in the trash, newest deleted first.
pub fn list(db: &Db) -> Result<Vec<Trashed>, db::Error> {
    let conn = db.conn()?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {}, strftime('%Y-%m-%dT%H:%M:%SZ', deleted_at, 'unixepoch'), \
             strftime('%Y-%m-%dT%H:%M:%SZ', deleted_at + ?1, 'unixepoch') \
             FROM objects WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC, id DESC",
            objects::columns()
        ))
        .map_err(sql)?;
    let rows = stmt.query_map([RETENTION_SECS], trashed).map_err(sql)?;
    rows.collect::<Result<_, _>>().map_err(sql)
}

fn trashed(r: &Row) -> rusqlite::Result<Trashed> {
    Ok(Trashed {
        object: objects::object(r)?,
        deleted: r.get(9)?,
        purge_at: r.get(10)?,
    })
}

/// Takes object `id` out of the trash and returns it, or `None` if it is not in the trash.
pub fn restore(db: &Db, id: i64) -> Result<Option<Object>, db::Error> {
    let n = db
        .conn()?
        .execute(
            "UPDATE objects SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
            [id],
        )
        .map_err(sql)?;
    if n == 0 {
        return Ok(None);
    }
    objects::get(db, id)
}

/// Deletes object `id` for good if it is in the trash. Returns whether it was.
pub fn purge(db: &Db, id: i64) -> Result<bool, Error> {
    let in_trash = db
        .conn()?
        .query_row(
            "SELECT 1 FROM objects WHERE id = ?1 AND deleted_at IS NOT NULL",
            [id],
            |_| Ok(()),
        )
        .optional()
        .map_err(sql)?
        .is_some();
    if !in_trash {
        return Ok(false);
    }
    attachments::remove_files(db, id)?;
    let n = db
        .conn()?
        .execute(
            "DELETE FROM objects WHERE id = ?1 AND deleted_at IS NOT NULL",
            [id],
        )
        .map_err(sql)?;
    Ok(n > 0)
}

/// Deletes every object in the trash for good. Returns how many were deleted.
pub fn empty(db: &Db) -> Result<usize, Error> {
    purge_where(db, "deleted_at IS NOT NULL", &[])
}

/// Deletes for good every object deleted more than [`RETENTION_SECS`] before `now_unix`.
/// Returns how many were deleted. Live objects are never touched.
pub fn purge_expired(db: &Db, now_unix: i64) -> Result<usize, Error> {
    purge_where(db, "deleted_at < ?1", &[&(now_unix - RETENTION_SECS)])
}

/// Purges, one by one, the trashed objects matching `condition`.
fn purge_where(db: &Db, condition: &str, args: &[&dyn ToSql]) -> Result<usize, Error> {
    let ids: Vec<i64> = {
        let conn = db.conn()?;
        let mut stmt = conn
            .prepare(&format!("SELECT id FROM objects WHERE {condition}"))
            .map_err(sql)?;
        let rows = stmt.query_map(args, |r| r.get(0)).map_err(sql)?;
        rows.collect::<Result<_, _>>().map_err(sql)?
    };
    let mut purged = 0;
    for id in ids {
        if purge(db, id)? {
            purged += 1;
        }
    }
    Ok(purged)
}
