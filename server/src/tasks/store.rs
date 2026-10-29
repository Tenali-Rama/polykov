//! The `tasks` table over live `task` objects. An object without a `tasks` row reads with
//! the defaults; the row is written on the first task change.

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, types::Type};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use super::validate;
use crate::{
    db::{self, Db},
    http::ApiError,
    objects::store::{self as objects, Object},
    tags,
};

/// One checklist line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChecklistItem {
    pub text: String,
    #[serde(default)]
    pub done: bool,
}

/// The task fields of a task, as served under `"task"`. `completed` is RFC 3339 UTC.
#[derive(Debug, Clone, Serialize)]
pub struct Fields {
    pub status: String,
    pub priority: i64,
    pub scheduled: Option<String>,
    pub deadline: Option<String>,
    pub estimate: Option<i64>,
    pub someday: bool,
    pub parent_id: Option<i64>,
    pub project_id: Option<i64>,
    pub section_id: Option<i64>,
    pub area_id: Option<i64>,
    pub checklist: Vec<ChecklistItem>,
    pub recurrence: Option<Value>,
    pub completed: Option<String>,
}

/// A task: the object JSON plus `"task": Fields`.
#[derive(Debug, Serialize)]
pub struct Task {
    #[serde(flatten)]
    pub object: Object,
    pub task: Fields,
}

/// Changes to the editable task fields; `None` leaves a field as it is, and `Some(None)`
/// clears a nullable one. Also the `"task"` body of `POST`/`PATCH /api/tasks`. The store
/// checks every rule in `validate`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldChanges {
    #[serde(default, deserialize_with = "present")]
    pub priority: Option<i64>,
    #[serde(default, deserialize_with = "nullable")]
    pub scheduled: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub deadline: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub estimate: Option<Option<i64>>,
    #[serde(default, deserialize_with = "present")]
    pub someday: Option<bool>,
    #[serde(default, deserialize_with = "nullable")]
    pub parent_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub project_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub section_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub area_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "present")]
    pub checklist: Option<Vec<ChecklistItem>>,
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

impl FieldChanges {
    fn is_empty(&self) -> bool {
        self.priority.is_none()
            && self.scheduled.is_none()
            && self.deadline.is_none()
            && self.estimate.is_none()
            && self.someday.is_none()
            && self.parent_id.is_none()
            && self.project_id.is_none()
            && self.section_id.is_none()
            && self.area_id.is_none()
            && self.checklist.is_none()
    }
}

/// A new task. `title`, `properties` and `tags` must already pass the object rules.
#[derive(Debug, Default)]
pub struct New {
    pub title: String,
    pub properties: Map<String, Value>,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub task: FieldChanges,
}

/// Changes to a task. `object` must already pass the object rules.
#[derive(Debug, Default)]
pub struct Changes {
    pub object: objects::Changes,
    pub task: FieldChanges,
}

/// Stores `new` (object and task row in one transaction) and returns it. A broken task
/// rule is a `400`.
pub fn create(db: &Db, new: &New) -> Result<Task, ApiError> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    tx.execute(
        "INSERT INTO objects (type, title, properties, pinned) VALUES ('task', ?1, ?2, ?3)",
        params![
            new.title,
            Value::Object(new.properties.clone()).to_string(),
            new.pinned
        ],
    )
    .map_err(db::Error::Sqlite)?;
    let id = tx.last_insert_rowid();
    tags::store::replace(&tx, id, &new.tags)?;
    let current = select(&tx, id)?.ok_or_else(missing)?;
    write_fields(&tx, id, current.task, &new.task)?;
    tx.commit().map_err(db::Error::Sqlite)?;
    Ok(select(&conn, id)?.ok_or_else(missing)?)
}

/// The live task `id`, or `None` if it is unknown, in the trash or not a task.
pub fn get(db: &Db, id: i64) -> Result<Option<Task>, db::Error> {
    select(&*db.conn()?, id)
}

/// The live tasks among `ids`, in the given order; other ids are skipped.
pub fn get_many(db: &Db, ids: &[i64]) -> Result<Vec<Task>, db::Error> {
    let conn = db.conn()?;
    let mut tasks = Vec::with_capacity(ids.len());
    for &id in ids {
        tasks.extend(select(&conn, id)?);
    }
    Ok(tasks)
}

/// Applies `changes` to the live task `id` and returns it, or `None` if there is no such
/// task. Any change sets the object's `updated` to now. A broken task rule is a `400`.
pub fn update(db: &Db, id: i64, changes: &Changes) -> Result<Option<Task>, ApiError> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    let Some(current) = select(&tx, id)? else {
        return Ok(None);
    };
    let object = &changes.object;
    let object_changed = object.title.is_some()
        || object.properties.is_some()
        || object.pinned.is_some()
        || object.archived.is_some()
        || object.tags.is_some();
    if !changes.task.is_empty() {
        write_fields(&tx, id, current.task, &changes.task)?;
    }
    if object_changed || !changes.task.is_empty() {
        let properties = object
            .properties
            .as_ref()
            .map(|p| Value::Object(p.clone()).to_string());
        tx.execute(
            "UPDATE objects SET title = COALESCE(?2, title), \
             properties = COALESCE(?3, properties), pinned = COALESCE(?4, pinned), \
             archived = COALESCE(?5, archived), updated = unixepoch() WHERE id = ?1",
            params![id, object.title, properties, object.pinned, object.archived],
        )
        .map_err(db::Error::Sqlite)?;
        if let Some(tags) = &object.tags {
            tags::store::replace(&tx, id, tags)?;
        }
    }
    tx.commit().map_err(db::Error::Sqlite)?;
    Ok(select(&conn, id)?)
}

/// Marks the live task `id` done now and returns it, or `None` if there is no such task.
/// A task that is already done is left as it is.
pub fn complete(db: &Db, id: i64) -> Result<Option<Task>, db::Error> {
    set_status(
        db,
        id,
        "INSERT INTO tasks (object_id, status, completed) VALUES (?1, 'done', unixepoch()) \
         ON CONFLICT (object_id) DO UPDATE SET status = 'done', completed = unixepoch() \
         WHERE tasks.status <> 'done'",
    )
}

/// Marks the live task `id` open, clearing `completed`, and returns it, or `None` if there
/// is no such task. A task that is already open is left as it is.
pub fn reopen(db: &Db, id: i64) -> Result<Option<Task>, db::Error> {
    set_status(
        db,
        id,
        "UPDATE tasks SET status = 'open', completed = NULL \
         WHERE object_id = ?1 AND status <> 'open'",
    )
}

/// Runs the status write `sql` (bound to `?1 = id`) on a live task, bumping `updated` if it
/// changed a row.
fn set_status(db: &Db, id: i64, sql: &str) -> Result<Option<Task>, db::Error> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    if select(&tx, id)?.is_none() {
        return Ok(None);
    }
    if tx.execute(sql, [id]).map_err(db::Error::Sqlite)? > 0 {
        tx.execute(
            "UPDATE objects SET updated = unixepoch() WHERE id = ?1",
            [id],
        )
        .map_err(db::Error::Sqlite)?;
    }
    tx.commit().map_err(db::Error::Sqlite)?;
    select(&conn, id)
}

/// Checks `changes` against the task rules, merges them into `fields` and upserts the row.
fn write_fields(
    tx: &Transaction,
    id: i64,
    mut fields: Fields,
    changes: &FieldChanges,
) -> Result<(), ApiError> {
    if let Some(p) = changes.priority {
        fields.priority = validate::priority(p)?;
    }
    for (name, change, field) in [
        ("scheduled", &changes.scheduled, &mut fields.scheduled),
        ("deadline", &changes.deadline, &mut fields.deadline),
    ] {
        if let Some(value) = change {
            if let Some(date) = value {
                validate::date(name, date)?;
            }
            field.clone_from(value);
        }
    }
    if let Some(estimate) = changes.estimate {
        fields.estimate = estimate.map(validate::estimate).transpose()?;
    }
    if let Some(someday) = changes.someday {
        fields.someday = someday;
    }
    if let Some(parent) = changes.parent_id {
        if let Some(parent) = parent {
            validate::parent(tx, id, parent)?;
        }
        fields.parent_id = parent;
    }
    if let Some(project) = changes.project_id {
        if let Some(project) = project {
            validate::live(tx, "project", project)?;
        }
        if project != fields.project_id {
            fields.section_id = None;
        }
        fields.project_id = project;
    }
    if let Some(section) = changes.section_id {
        if let Some(section) = section {
            validate::section(tx, section, fields.project_id)?;
        }
        fields.section_id = section;
    }
    if let Some(area) = changes.area_id {
        if let Some(area) = area {
            validate::live(tx, "area", area)?;
        }
        fields.area_id = area;
    }
    if let Some(checklist) = &changes.checklist {
        fields.checklist = validate::checklist(checklist)?;
    }
    let checklist = serde_json::to_string(&fields.checklist)
        .map_err(|e| ApiError::Internal(format!("checklist encoding failed: {e}")))?;
    tx.execute(
        "INSERT INTO tasks (object_id, priority, scheduled, deadline, estimate, someday, \
         parent_id, project_id, section_id, area_id, checklist) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
         ON CONFLICT (object_id) DO UPDATE SET priority = excluded.priority, \
         scheduled = excluded.scheduled, deadline = excluded.deadline, \
         estimate = excluded.estimate, someday = excluded.someday, \
         parent_id = excluded.parent_id, project_id = excluded.project_id, \
         section_id = excluded.section_id, area_id = excluded.area_id, \
         checklist = excluded.checklist",
        params![
            id,
            fields.priority,
            fields.scheduled,
            fields.deadline,
            fields.estimate,
            fields.someday,
            fields.parent_id,
            fields.project_id,
            fields.section_id,
            fields.area_id,
            checklist
        ],
    )
    .map_err(db::Error::Sqlite)?;
    Ok(())
}

fn missing() -> db::Error {
    db::Error::Sqlite(rusqlite::Error::QueryReturnedNoRows)
}

fn select(conn: &Connection, id: i64) -> Result<Option<Task>, db::Error> {
    conn.query_row(
        &format!(
            "SELECT {}, COALESCE(t.status, 'open'), COALESCE(t.priority, 4), t.scheduled, \
             t.deadline, t.estimate, COALESCE(t.someday, 0), t.parent_id, t.project_id, \
             t.section_id, t.area_id, COALESCE(t.checklist, '[]'), t.recurrence, \
             strftime('%Y-%m-%dT%H:%M:%SZ', t.completed, 'unixepoch') \
             FROM objects LEFT JOIN tasks t ON t.object_id = objects.id \
             WHERE objects.id = ?1 AND objects.type = 'task' AND objects.deleted_at IS NULL",
            objects::columns()
        ),
        [id],
        task,
    )
    .optional()
    .map_err(db::Error::Sqlite)
}

/// Reads one row of [`select`]: the object columns (0–8), then the task columns.
fn task(r: &Row) -> rusqlite::Result<Task> {
    let recurrence: Option<String> = r.get(20)?;
    Ok(Task {
        object: objects::object(r)?,
        task: Fields {
            status: r.get(9)?,
            priority: r.get(10)?,
            scheduled: r.get(11)?,
            deadline: r.get(12)?,
            estimate: r.get(13)?,
            someday: r.get(14)?,
            parent_id: r.get(15)?,
            project_id: r.get(16)?,
            section_id: r.get(17)?,
            area_id: r.get(18)?,
            checklist: json(19, &r.get::<_, String>(19)?)?,
            recurrence: recurrence.map(|raw| json(20, &raw)).transpose()?,
            completed: r.get(21)?,
        },
    })
}

fn json<T: serde::de::DeserializeOwned>(i: usize, raw: &str) -> rusqlite::Result<T> {
    serde_json::from_str(raw)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(i, Type::Text, Box::new(e)))
}
