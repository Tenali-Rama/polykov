//! The `objects` table: create, read, update and move to the trash. Every read here sees
//! live objects only (`deleted_at IS NULL`).

use rusqlite::{Connection, OptionalExtension, Row, params, types::Type};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::db::{self, Db};

/// An object as served over the API. Times are RFC 3339 UTC.
#[derive(Debug, Serialize)]
pub struct Object {
    pub id: i64,
    pub r#type: String,
    pub title: String,
    pub created: String,
    pub updated: String,
    pub archived: bool,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub properties: Map<String, Value>,
}

/// Just enough of a live object for other modules to check a target and name it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub id: i64,
    pub r#type: String,
    pub title: String,
}

/// A validated new object.
#[derive(Debug)]
pub struct New {
    pub r#type: String,
    pub title: String,
    pub properties: Map<String, Value>,
    pub pinned: bool,
    pub archived: bool,
}

/// Validated changes to an object; `None` leaves a field as it is.
#[derive(Debug, Default)]
pub struct Changes {
    pub title: Option<String>,
    pub properties: Option<Map<String, Value>>,
    pub pinned: Option<bool>,
    pub archived: Option<bool>,
}

impl Changes {
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.properties.is_none()
            && self.pinned.is_none()
            && self.archived.is_none()
    }
}

const COLUMNS: &str = "id, type, title, strftime('%Y-%m-%dT%H:%M:%SZ', created, 'unixepoch'), \
     strftime('%Y-%m-%dT%H:%M:%SZ', updated, 'unixepoch'), archived, pinned, properties";

/// Stores `new` and returns it as created.
pub fn create(db: &Db, new: &New) -> Result<Object, db::Error> {
    let conn = db.conn()?;
    conn.execute(
        "INSERT INTO objects (type, title, properties, pinned, archived) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            new.r#type,
            new.title,
            Value::Object(new.properties.clone()).to_string(),
            new.pinned,
            new.archived
        ],
    )
    .map_err(db::Error::Sqlite)?;
    let id = conn.last_insert_rowid();
    select(&conn, id)?.ok_or(db::Error::Sqlite(rusqlite::Error::QueryReturnedNoRows))
}

/// The live object `id`, or `None` if it is unknown or in the trash.
pub fn get(db: &Db, id: i64) -> Result<Option<Object>, db::Error> {
    select(&*db.conn()?, id)
}

/// Applies `changes` to the live object `id` and returns it, or `None` if it is unknown or in
/// the trash. Any given field sets `updated` to now; no changes leave the object untouched.
pub fn update(db: &Db, id: i64, changes: &Changes) -> Result<Option<Object>, db::Error> {
    let conn = db.conn()?;
    if !changes.is_empty() {
        let properties = changes
            .properties
            .as_ref()
            .map(|p| Value::Object(p.clone()).to_string());
        conn.execute(
            "UPDATE objects SET title = COALESCE(?2, title), \
             properties = COALESCE(?3, properties), pinned = COALESCE(?4, pinned), \
             archived = COALESCE(?5, archived), updated = unixepoch() \
             WHERE id = ?1 AND deleted_at IS NULL",
            params![
                id,
                changes.title,
                properties,
                changes.pinned,
                changes.archived
            ],
        )
        .map_err(db::Error::Sqlite)?;
    }
    select(&conn, id)
}

/// Moves the live object `id` to the trash. Returns whether it was live.
pub fn trash(db: &Db, id: i64) -> Result<bool, db::Error> {
    let n = db
        .conn()?
        .execute(
            "UPDATE objects SET deleted_at = unixepoch() WHERE id = ?1 AND deleted_at IS NULL",
            [id],
        )
        .map_err(db::Error::Sqlite)?;
    Ok(n > 0)
}

/// The id, type and title of the live object `id`, or `None` if it is unknown or in the
/// trash. For modules that attach things to objects (links, comments, attachments).
pub fn summary(db: &Db, id: i64) -> Result<Option<Summary>, db::Error> {
    db.conn()?
        .query_row(
            "SELECT id, type, title FROM objects WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| {
                Ok(Summary {
                    id: r.get(0)?,
                    r#type: r.get(1)?,
                    title: r.get(2)?,
                })
            },
        )
        .optional()
        .map_err(db::Error::Sqlite)
}

fn select(conn: &Connection, id: i64) -> Result<Option<Object>, db::Error> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM objects WHERE id = ?1 AND deleted_at IS NULL"),
        [id],
        object,
    )
    .optional()
    .map_err(db::Error::Sqlite)
}

fn object(r: &Row) -> rusqlite::Result<Object> {
    let raw: String = r.get(7)?;
    let properties = serde_json::from_str(&raw)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(7, Type::Text, Box::new(e)))?;
    Ok(Object {
        id: r.get(0)?,
        r#type: r.get(1)?,
        title: r.get(2)?,
        created: r.get(3)?,
        updated: r.get(4)?,
        archived: r.get(5)?,
        pinned: r.get(6)?,
        tags: Vec::new(),
        properties,
    })
}
