//! The `projects` table over live `project` objects. An object without a `projects` row
//! reads with the defaults (`active`, nulls); the row is written on the first project change.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    db::{self, Db},
    http::ApiError,
    objects::store::{self as objects, Object},
    tasks::validate,
};

/// Project statuses, in the order the API names them.
pub const STATUSES: [&str; 3] = ["active", "someday", "done"];

/// The project fields of a project, as served under `"project"`. The counts are of the
/// live (not trashed) tasks filed under it.
#[derive(Debug, Clone, Serialize)]
pub struct Fields {
    pub status: String,
    pub area_id: Option<i64>,
    pub start: Option<String>,
    pub deadline: Option<String>,
    pub open_tasks: i64,
    pub done_tasks: i64,
}

/// A project: the object JSON plus `"project": Fields`.
#[derive(Debug, Serialize)]
pub struct Project {
    #[serde(flatten)]
    pub object: Object,
    pub project: Fields,
}

/// Changes to the editable project fields; `None` leaves a field as it is, and `Some(None)`
/// clears a nullable one. Also the body of `PATCH /api/projects/{id}`. The store checks
/// every rule.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Changes {
    #[serde(default, deserialize_with = "present")]
    pub status: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub area_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub start: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub deadline: Option<Option<String>>,
}

/// A present field must not be `null`.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// A present field may be `null`, which is `Some(None)`.
fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

impl Changes {
    fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.area_id.is_none()
            && self.start.is_none()
            && self.deadline.is_none()
    }
}

/// A validated list request; `None` matches anything.
#[derive(Debug)]
pub struct Filter {
    pub status: Option<String>,
    pub area_id: Option<i64>,
}

/// The live project `id`, or `None` if it is unknown, in the trash or not a project.
pub fn get(db: &Db, id: i64) -> Result<Option<Project>, db::Error> {
    select(&*db.conn()?, id)
}

/// Live, non-archived projects matching `filter`, by title (case-insensitive), then id.
pub fn list(db: &Db, filter: &Filter) -> Result<Vec<Project>, db::Error> {
    let conn = db.conn()?;
    let mut stmt = conn
        .prepare(&format!(
            "{} AND objects.archived = 0 AND (?1 IS NULL OR COALESCE(p.status, 'active') = ?1) \
             AND (?2 IS NULL OR p.area_id = ?2) ORDER BY objects.title COLLATE NOCASE, objects.id",
            query()
        ))
        .map_err(db::Error::Sqlite)?;
    stmt.query_map(params![filter.status, filter.area_id], project)
        .map_err(db::Error::Sqlite)?
        .collect::<Result<_, _>>()
        .map_err(db::Error::Sqlite)
}

/// Applies `changes` to the live project `id` and returns it, or `None` if there is no such
/// project. Any change upserts the row and sets the object's `updated` to now. A broken
/// rule is a `400`.
pub fn update(db: &Db, id: i64, changes: &Changes) -> Result<Option<Project>, ApiError> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    let Some(current) = select(&tx, id)? else {
        return Ok(None);
    };
    if !changes.is_empty() {
        let mut fields = current.project;
        if let Some(status) = &changes.status {
            if !STATUSES.contains(&status.as_str()) {
                return Err(ApiError::bad_request(
                    "status must be active, someday or done",
                ));
            }
            fields.status.clone_from(status);
        }
        if let Some(area) = changes.area_id {
            if let Some(area) = area {
                live_area(&tx, area)?;
            }
            fields.area_id = area;
        }
        for (name, change, field) in [
            ("start", &changes.start, &mut fields.start),
            ("deadline", &changes.deadline, &mut fields.deadline),
        ] {
            if let Some(value) = change {
                if let Some(date) = value {
                    validate::date(name, date)?;
                }
                field.clone_from(value);
            }
        }
        tx.execute(
            "INSERT INTO projects (object_id, status, area_id, start, deadline) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT (object_id) DO UPDATE SET status = excluded.status, \
             area_id = excluded.area_id, start = excluded.start, deadline = excluded.deadline",
            params![
                id,
                fields.status,
                fields.area_id,
                fields.start,
                fields.deadline
            ],
        )
        .map_err(db::Error::Sqlite)?;
        tx.execute(
            "UPDATE objects SET updated = unixepoch() WHERE id = ?1",
            [id],
        )
        .map_err(db::Error::Sqlite)?;
    }
    tx.commit().map_err(db::Error::Sqlite)?;
    Ok(select(&conn, id)?)
}

/// Whether `id` is a live project, read on `conn` (inside the caller's transaction).
pub(super) fn is_live(conn: &Connection, id: i64) -> Result<bool, db::Error> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM objects WHERE id = ?1 AND type = 'project' AND deleted_at IS NULL",
            [id],
            |_| Ok(()),
        )
        .optional()
        .map_err(db::Error::Sqlite)?
        .is_some())
}

fn live_area(conn: &Connection, id: i64) -> Result<(), ApiError> {
    let found = conn
        .query_row(
            "SELECT 1 FROM objects WHERE id = ?1 AND type = 'area' AND deleted_at IS NULL",
            [id],
            |_| Ok(()),
        )
        .optional()
        .map_err(db::Error::Sqlite)?;
    match found {
        Some(()) => Ok(()),
        None => Err(ApiError::bad_request("area must be a live area")),
    }
}

/// Every live project with its fields and task counts; callers add conditions with `AND`.
fn query() -> String {
    let count = |status: &str| {
        format!(
            "(SELECT COUNT(*) FROM tasks t JOIN objects o ON o.id = t.object_id \
             WHERE t.project_id = objects.id AND o.deleted_at IS NULL AND t.status = '{status}')"
        )
    };
    format!(
        "SELECT {}, COALESCE(p.status, 'active'), p.area_id, p.start, p.deadline, {}, {} \
         FROM objects LEFT JOIN projects p ON p.object_id = objects.id \
         WHERE objects.type = 'project' AND objects.deleted_at IS NULL",
        objects::columns(),
        count("open"),
        count("done")
    )
}

fn select(conn: &Connection, id: i64) -> Result<Option<Project>, db::Error> {
    conn.query_row(&format!("{} AND objects.id = ?1", query()), [id], project)
        .optional()
        .map_err(db::Error::Sqlite)
}

/// Reads one row of [`query`]: the object columns (0–8), then the project columns.
fn project(r: &Row) -> rusqlite::Result<Project> {
    Ok(Project {
        object: objects::object(r)?,
        project: Fields {
            status: r.get(9)?,
            area_id: r.get(10)?,
            start: r.get(11)?,
            deadline: r.get(12)?,
            open_tasks: r.get(13)?,
            done_tasks: r.get(14)?,
        },
    })
}
