//! The `object_tags` table. Tags here are already normalized (see [`super::normalize`]).

use std::collections::BTreeMap;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::db::{self, Db};

/// SQL for the JSON array, sorted, of the tags of the `objects` row in scope.
pub const SORTED_JSON: &str = "(SELECT json_group_array(tag ORDER BY tag) FROM object_tags \
     WHERE object_id = objects.id)";

/// SQL condition on the `objects` row in scope: it carries the tag bound to `?{param}`, or a
/// tag under it (`work` matches `work/x`, not `workshop`).
pub fn under(param: usize) -> String {
    // Every tag under `t` sorts in [`t/`, `t0`) because `0` follows `/`, so the tag index
    // serves both halves.
    format!(
        "objects.id IN (SELECT object_id FROM object_tags WHERE tag = ?{param} \
         OR (tag >= ?{param} || '/' AND tag < ?{param} || '0'))"
    )
}

/// A tag folder and how many live, non-archived objects carry it or a tag under it.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Count {
    pub tag: String,
    pub count: u64,
}

/// Replaces the tags of object `id` with `tags`. The caller checks the object is live and
/// runs this in the same transaction as the object write.
pub fn replace(conn: &Connection, id: i64, tags: &[String]) -> Result<(), db::Error> {
    conn.execute("DELETE FROM object_tags WHERE object_id = ?1", [id])
        .map_err(db::Error::Sqlite)?;
    let mut insert = conn
        .prepare_cached("INSERT OR IGNORE INTO object_tags (object_id, tag) VALUES (?1, ?2)")
        .map_err(db::Error::Sqlite)?;
    for tag in tags {
        insert
            .execute(params![id, tag])
            .map_err(db::Error::Sqlite)?;
    }
    Ok(())
}

/// Every tag folder used by a live, non-archived object, implicit parents included, sorted by
/// tag. An object counts once per folder however many of its tags fall under it.
pub fn counts(db: &Db) -> Result<Vec<Count>, db::Error> {
    let conn = db.conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT t.object_id, t.tag FROM object_tags t JOIN objects o ON o.id = t.object_id \
             WHERE o.deleted_at IS NULL AND o.archived = 0 ORDER BY t.object_id",
        )
        .map_err(db::Error::Sqlite)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(db::Error::Sqlite)?;
    // Folder -> (count, last object counted). Rows come grouped by object, so skipping the
    // last object counted counts each object once per folder. Ids start at 1.
    let mut folders: BTreeMap<String, (u64, i64)> = BTreeMap::new();
    for row in rows {
        let (id, tag) = row.map_err(db::Error::Sqlite)?;
        let ends = tag.match_indices('/').map(|(i, _)| i).chain([tag.len()]);
        for end in ends {
            let entry = folders.entry(tag[..end].to_owned()).or_insert((0, 0));
            if entry.1 != id {
                *entry = (entry.0 + 1, id);
            }
        }
    }
    Ok(folders
        .into_iter()
        .map(|(tag, (count, _))| Count { tag, count })
        .collect())
}
