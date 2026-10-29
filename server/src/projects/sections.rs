//! The `sections` table: ordered headings inside a live project. Every write renumbers the
//! project's sections to a dense `0..n`, in order of position, then id.

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;

use super::store;
use crate::{
    db::{self, Db},
    http::ApiError,
};

/// Longest section title after trimming, in characters.
pub const MAX_TITLE_CHARS: usize = 200;

/// A section as served over the API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub position: i64,
}

/// Returns `title` trimmed, if it is 1 to [`MAX_TITLE_CHARS`] characters.
pub fn title(title: &str) -> Result<String, ApiError> {
    let title = title.trim();
    let len = title.chars().count();
    if len == 0 || len > MAX_TITLE_CHARS {
        return Err(ApiError::bad_request(format!(
            "title must be 1 to {MAX_TITLE_CHARS} characters"
        )));
    }
    Ok(title.to_owned())
}

/// Checks `position` is not negative. A position past the end means "last".
pub fn position(position: i64) -> Result<usize, ApiError> {
    usize::try_from(position)
        .map_err(|_| ApiError::bad_request("position must be a non-negative integer"))
}

/// The sections of the live project `project`, by position, or `None` if there is no such
/// project.
pub fn list(db: &Db, project: i64) -> Result<Option<Vec<Section>>, db::Error> {
    let conn = db.conn()?;
    if !store::is_live(&conn, project)? {
        return Ok(None);
    }
    select_all(&conn, project).map(Some)
}

/// Adds a section titled `title` (already validated) to the live project `project` at
/// `position` (last when `None` or past the end), shifting later ones. `None` if there is
/// no such project.
pub fn create(
    db: &Db,
    project: i64,
    title: &str,
    position: Option<usize>,
) -> Result<Option<Section>, db::Error> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    if !store::is_live(&tx, project)? {
        return Ok(None);
    }
    tx.execute(
        "INSERT INTO sections (project_id, title) VALUES (?1, ?2)",
        params![project, title],
    )
    .map_err(db::Error::Sqlite)?;
    let id = tx.last_insert_rowid();
    place(&tx, project, id, position)?;
    tx.commit().map_err(db::Error::Sqlite)?;
    select(&conn, id)
}

/// Renames and/or moves the section `id` (title already validated) and returns it, or
/// `None` if it is unknown or its project is not live.
pub fn update(
    db: &Db,
    id: i64,
    title: Option<&str>,
    position: Option<usize>,
) -> Result<Option<Section>, db::Error> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    let Some(section) = select(&tx, id)? else {
        return Ok(None);
    };
    if let Some(title) = title {
        tx.execute(
            "UPDATE sections SET title = ?2 WHERE id = ?1",
            params![id, title],
        )
        .map_err(db::Error::Sqlite)?;
    }
    if position.is_some() {
        place(&tx, section.project_id, id, position)?;
    }
    tx.commit().map_err(db::Error::Sqlite)?;
    select(&conn, id)
}

/// Deletes the section `id`; its tasks keep their project and lose the section. Returns
/// whether it existed in a live project.
pub fn delete(db: &Db, id: i64) -> Result<bool, db::Error> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    let Some(section) = select(&tx, id)? else {
        return Ok(false);
    };
    tx.execute("DELETE FROM sections WHERE id = ?1", [id])
        .map_err(db::Error::Sqlite)?;
    place(&tx, section.project_id, id, None)?;
    tx.commit().map_err(db::Error::Sqlite)?;
    Ok(true)
}

/// Moves section `id` of `project` to `position` (last when `None` or past the end; a
/// deleted `id` is simply absent), then renumbers the project's sections `0..n`.
fn place(
    tx: &Transaction,
    project: i64,
    id: i64,
    position: Option<usize>,
) -> Result<(), db::Error> {
    let mut ids: Vec<i64> = select_all(tx, project)?.into_iter().map(|s| s.id).collect();
    if let Some(at) = ids.iter().position(|&s| s == id) {
        ids.remove(at);
        let to = position.unwrap_or(ids.len()).min(ids.len());
        ids.insert(to, id);
    }
    for (pos, sid) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE sections SET position = ?2 WHERE id = ?1 AND position <> ?2",
            params![sid, pos as i64],
        )
        .map_err(db::Error::Sqlite)?;
    }
    Ok(())
}

fn select_all(conn: &Connection, project: i64) -> Result<Vec<Section>, db::Error> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, title, position FROM sections WHERE project_id = ?1 \
             ORDER BY position, id",
        )
        .map_err(db::Error::Sqlite)?;
    stmt.query_map([project], section)
        .map_err(db::Error::Sqlite)?
        .collect::<Result<_, _>>()
        .map_err(db::Error::Sqlite)
}

/// The section `id` if its project is live.
fn select(conn: &Connection, id: i64) -> Result<Option<Section>, db::Error> {
    conn.query_row(
        "SELECT s.id, s.project_id, s.title, s.position FROM sections s \
         JOIN objects o ON o.id = s.project_id \
         WHERE s.id = ?1 AND o.type = 'project' AND o.deleted_at IS NULL",
        [id],
        section,
    )
    .optional()
    .map_err(db::Error::Sqlite)
}

fn section(r: &rusqlite::Row) -> rusqlite::Result<Section> {
    Ok(Section {
        id: r.get(0)?,
        project_id: r.get(1)?,
        title: r.get(2)?,
        position: r.get(3)?,
    })
}
