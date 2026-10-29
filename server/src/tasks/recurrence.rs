//! Recurring tasks: the `recurrence` field and, on complete, the next occurrence (ADR 0013).

use chrono::NaiveDate;
use rusqlite::{Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    store::{self, ChecklistItem, Task},
    validate,
};
use crate::{
    db::{self, Db},
    http::ApiError,
    recur::{self, Rule},
    tags,
};

/// Which date the next occurrence counts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    /// The task's scheduled date, else its deadline, else the completion day.
    Scheduled,
    /// The completion day.
    Completion,
}

/// The `recurrence` task field: `{"rule":"FREQ=WEEKLY;BYDAY=SU","from":"scheduled"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recurrence {
    pub rule: String,
    pub from: Anchor,
}

/// Checks the rule parses and returns `recurrence` with it in canonical form.
pub fn validate(recurrence: &Recurrence) -> Result<Recurrence, ApiError> {
    Ok(Recurrence {
        rule: parse(&recurrence.rule)?.to_string(),
        from: recurrence.from,
    })
}

fn parse(rule: &str) -> Result<Rule, ApiError> {
    Rule::parse(rule).map_err(|e| ApiError::bad_request(format!("invalid recurrence: {e}")))
}

/// Marks the live task `id` done now and returns it with its next occurrence, or `None` if
/// there is no such task. A recurring task gets its next occurrence created in the same
/// transaction, and its own `recurrence` cleared. A task already done is left as it is.
pub fn complete(
    db: &Db,
    id: i64,
    today: NaiveDate,
) -> Result<Option<(Task, Option<Task>)>, ApiError> {
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(db::Error::Sqlite)?;
    let Some(current) = store::select(&tx, id)? else {
        return Ok(None);
    };
    if current.task.status == "done" {
        return Ok(Some((current, None)));
    }
    let next = current
        .task
        .recurrence
        .as_ref()
        .map(|recurrence| spawn(&tx, &current, recurrence, today))
        .transpose()?;
    tx.execute(
        "INSERT INTO tasks (object_id, status, completed) VALUES (?1, 'done', unixepoch()) \
         ON CONFLICT (object_id) DO UPDATE SET status = 'done', completed = unixepoch(), \
         recurrence = NULL",
        [id],
    )
    .map_err(db::Error::Sqlite)?;
    tx.execute(
        "UPDATE objects SET updated = unixepoch() WHERE id = ?1",
        [id],
    )
    .map_err(db::Error::Sqlite)?;
    tx.commit().map_err(db::Error::Sqlite)?;
    let done = store::select(&conn, id)?.ok_or_else(store::missing)?;
    let next = match next {
        Some(next) => Some(store::select(&conn, next)?.ok_or_else(store::missing)?),
        None => None,
    };
    Ok(Some((done, next)))
}

/// Creates the occurrence after `task` and returns its id.
fn spawn(
    tx: &Transaction,
    task: &Task,
    recurrence: &Recurrence,
    today: NaiveDate,
) -> Result<i64, ApiError> {
    let rule = parse(&recurrence.rule)?;
    let fields = &task.task;
    let scheduled = fields.scheduled.as_deref().map(split).transpose()?;
    let deadline = fields.deadline.as_deref().map(split).transpose()?;
    let base = scheduled.or(deadline).map_or(today, |(date, _)| date);
    let anchor = match recurrence.from {
        Anchor::Scheduled => base,
        Anchor::Completion => today,
    };
    let date = recur::next(&rule, anchor);
    let time = scheduled.map_or("", |(_, time)| time);
    let next_scheduled = join("scheduled", Some(date), time)?;
    let next_deadline = deadline
        .map(|(day, time)| join("deadline", day.checked_add_signed(date - base), time))
        .transpose()?;
    let checklist: Vec<ChecklistItem> = fields
        .checklist
        .iter()
        .map(|item| ChecklistItem {
            text: item.text.clone(),
            done: false,
        })
        .collect();
    let encode = |what: &str, value: Result<String, serde_json::Error>| {
        value.map_err(|e| ApiError::Internal(format!("{what} encoding failed: {e}")))
    };
    let checklist = encode("checklist", serde_json::to_string(&checklist))?;
    let recurrence = encode("recurrence", serde_json::to_string(recurrence))?;
    tx.execute(
        "INSERT INTO objects (type, title, properties) VALUES ('task', ?1, ?2)",
        params![
            task.object.title,
            Value::Object(task.object.properties.clone()).to_string()
        ],
    )
    .map_err(db::Error::Sqlite)?;
    let id = tx.last_insert_rowid();
    tags::store::replace(tx, id, &task.object.tags)?;
    tx.execute(
        "INSERT INTO tasks (object_id, priority, scheduled, deadline, estimate, someday, \
         parent_id, project_id, section_id, area_id, checklist, recurrence) \
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            fields.priority,
            next_scheduled,
            next_deadline,
            fields.estimate,
            fields.parent_id,
            fields.project_id,
            fields.section_id,
            fields.area_id,
            checklist,
            recurrence
        ],
    )
    .map_err(db::Error::Sqlite)?;
    Ok(id)
}

/// Splits a stored task date into its day and its time suffix (`""` or `"THH:MM"`).
fn split(value: &str) -> Result<(NaiveDate, &str), ApiError> {
    let (day, time) = value.split_at_checked(10).unwrap_or((value, ""));
    NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .map(|day| (day, time))
        .map_err(|e| ApiError::Internal(format!("stored task date {value:?} is broken: {e}")))
}

/// `day` plus the time suffix, if it is still a valid task date (the series can run past
/// year 9999).
fn join(field: &str, day: Option<NaiveDate>, time: &str) -> Result<String, ApiError> {
    let out_of_range =
        || ApiError::bad_request(format!("invalid recurrence: next {field} is out of range"));
    let value = format!("{}{time}", day.ok_or_else(out_of_range)?.format("%Y-%m-%d"));
    validate::date(field, &value).map_err(|_| out_of_range())?;
    Ok(value)
}
