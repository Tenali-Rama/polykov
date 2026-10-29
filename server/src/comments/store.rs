//! The `comments` table. Comments on an object in the trash are hidden: they cannot be read,
//! changed or deleted until the object is restored.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use crate::db::{self, Db};

/// A comment as served over the API. Times are RFC 3339 UTC.
#[derive(Debug, Serialize)]
pub struct Comment {
    pub id: i64,
    pub object_id: i64,
    pub body: String,
    pub created: String,
    pub updated: String,
}

const COLUMNS: &str = "c.id, c.object_id, c.body, \
     strftime('%Y-%m-%dT%H:%M:%SZ', c.created, 'unixepoch'), \
     strftime('%Y-%m-%dT%H:%M:%SZ', c.updated, 'unixepoch')";

/// Joins comments to their live objects only.
const LIVE: &str = "comments c JOIN objects o ON o.id = c.object_id AND o.deleted_at IS NULL";

/// Adds a comment with the validated `body` to object `object_id`, which the caller has
/// checked is live, and returns it.
pub fn create(db: &Db, object_id: i64, body: &str) -> Result<Comment, db::Error> {
    let conn = db.conn()?;
    conn.execute(
        "INSERT INTO comments (object_id, body) VALUES (?1, ?2)",
        params![object_id, body],
    )
    .map_err(db::Error::Sqlite)?;
    select(&conn, conn.last_insert_rowid())?
        .ok_or(db::Error::Sqlite(rusqlite::Error::QueryReturnedNoRows))
}

/// The comments on object `object_id`, oldest first.
pub fn list(db: &Db, object_id: i64) -> Result<Vec<Comment>, db::Error> {
    let conn = db.conn()?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM {LIVE} WHERE c.object_id = ?1 ORDER BY c.created, c.id"
        ))
        .map_err(db::Error::Sqlite)?;
    let rows = stmt
        .query_map([object_id], comment)
        .map_err(db::Error::Sqlite)?;
    rows.collect::<Result<_, _>>().map_err(db::Error::Sqlite)
}

/// Sets the body of comment `id` to the validated `body`, bumps its `updated` and returns it,
/// or `None` if it is unknown or its object is in the trash.
pub fn update(db: &Db, id: i64, body: &str) -> Result<Option<Comment>, db::Error> {
    let conn = db.conn()?;
    conn.execute(
        "UPDATE comments SET body = ?2, updated = unixepoch() WHERE id = ?1 AND object_id IN \
         (SELECT id FROM objects WHERE deleted_at IS NULL)",
        params![id, body],
    )
    .map_err(db::Error::Sqlite)?;
    select(&conn, id)
}

/// Deletes comment `id`. Returns whether it existed on a live object.
pub fn delete(db: &Db, id: i64) -> Result<bool, db::Error> {
    let n = db
        .conn()?
        .execute(
            "DELETE FROM comments WHERE id = ?1 AND object_id IN \
             (SELECT id FROM objects WHERE deleted_at IS NULL)",
            [id],
        )
        .map_err(db::Error::Sqlite)?;
    Ok(n > 0)
}

fn select(conn: &Connection, id: i64) -> Result<Option<Comment>, db::Error> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM {LIVE} WHERE c.id = ?1"),
        [id],
        comment,
    )
    .optional()
    .map_err(db::Error::Sqlite)
}

fn comment(r: &Row) -> rusqlite::Result<Comment> {
    Ok(Comment {
        id: r.get(0)?,
        object_id: r.get(1)?,
        body: r.get(2)?,
        created: r.get(3)?,
        updated: r.get(4)?,
    })
}
