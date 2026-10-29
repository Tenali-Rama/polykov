//! The `links` table. Callers check that both ends are live objects; reads skip objects in
//! the trash, so their links come back once they are restored.

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::{
    db::{self, Db},
    objects::store::Summary,
};

/// The live objects `id` links to, and the live objects that link to it, each by title.
#[derive(Debug, Serialize)]
pub struct Links {
    pub links: Vec<Summary>,
    pub backlinks: Vec<Summary>,
}

/// Links `src` to `dst`. Linking twice is not an error.
pub fn add(db: &Db, src: i64, dst: i64) -> Result<(), db::Error> {
    db.conn()?
        .execute(
            "INSERT OR IGNORE INTO links (src_id, dst_id) VALUES (?1, ?2)",
            params![src, dst],
        )
        .map_err(db::Error::Sqlite)?;
    Ok(())
}

/// Removes the link from `src` to `dst`. Returns whether there was one.
pub fn remove(db: &Db, src: i64, dst: i64) -> Result<bool, db::Error> {
    let n = db
        .conn()?
        .execute(
            "DELETE FROM links WHERE src_id = ?1 AND dst_id = ?2",
            params![src, dst],
        )
        .map_err(db::Error::Sqlite)?;
    Ok(n > 0)
}

/// The links and backlinks of `id`, skipping objects in the trash.
pub fn list(db: &Db, id: i64) -> Result<Links, db::Error> {
    let conn = db.conn()?;
    Ok(Links {
        links: ends(&conn, id, "src_id", "dst_id")?,
        backlinks: ends(&conn, id, "dst_id", "src_id")?,
    })
}

/// The live objects at the `other` end of the links whose `this` end is `id`, by title.
fn ends(conn: &Connection, id: i64, this: &str, other: &str) -> Result<Vec<Summary>, db::Error> {
    let sql = format!(
        "SELECT o.id, o.type, o.title FROM links l JOIN objects o ON o.id = l.{other} \
         WHERE l.{this} = ?1 AND o.deleted_at IS NULL ORDER BY o.title, o.id"
    );
    let mut stmt = conn.prepare(&sql).map_err(db::Error::Sqlite)?;
    let rows = stmt
        .query_map([id], |r| {
            Ok(Summary {
                id: r.get(0)?,
                r#type: r.get(1)?,
                title: r.get(2)?,
            })
        })
        .map_err(db::Error::Sqlite)?;
    rows.collect::<Result<_, _>>().map_err(db::Error::Sqlite)
}
