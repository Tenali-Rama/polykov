//! The sort and group keys of every object a view query matches, and the view's sort order.
//! Only these small rows are read for all matches; full objects are loaded for the kept ones.

use std::cmp::Ordering;

use chrono::NaiveDate;
use rusqlite::{Connection, params_from_iter, types::Value};

use super::spec::Sort;
use crate::{
    db::{self, Db},
    http::ApiError,
    query::{self, FROM, Param, Query},
    search, smartsort,
    tasks::store::Fields,
};

/// What sorting and grouping need of one matching object. Task fields are `None` for other
/// types; a reference to a trashed project or area counts as none.
#[derive(Debug, Clone)]
pub struct Key {
    pub id: i64,
    pub r#type: String,
    pub title: String,
    /// Unix seconds.
    pub created: i64,
    pub updated: i64,
    pub completed: Option<i64>,
    pub status: Option<String>,
    pub priority: Option<i64>,
    pub scheduled: Option<String>,
    pub deadline: Option<String>,
    pub estimate: Option<i64>,
    pub project: Option<(i64, String)>,
    pub section: Option<(i64, String)>,
    /// The task's own area, else its project's; a project's own area.
    pub area: Option<(i64, String)>,
    /// The first of the sorted tags.
    pub tag: Option<String>,
}

/// The extra joins: the task's live project, its section, and the live area of the task, of
/// its project, or of the project object itself.
const JOINS: &str = "LEFT JOIN objects vp ON vp.id = t.project_id AND vp.type = 'project' \
     AND vp.deleted_at IS NULL \
     LEFT JOIN sections vs ON vs.id = t.section_id \
     LEFT JOIN projects vpa ON vpa.object_id = \
     COALESCE(vp.id, CASE WHEN objects.type = 'project' THEN objects.id END) \
     LEFT JOIN objects va ON va.id = COALESCE(t.area_id, vpa.area_id) AND va.type = 'area' \
     AND va.deleted_at IS NULL";

/// The keys of every live object matching `query` as of `today`, in no particular order.
pub fn select(db: &Db, query: &Query, today: NaiveDate) -> Result<Vec<Key>, ApiError> {
    let compiled = query::compile(query, today)?;
    let conn = db.conn()?;
    let args = bind(&conn, &compiled.params)?;
    let sql = format!(
        "SELECT objects.id, objects.type, objects.title, objects.created, objects.updated, \
         t.completed, \
         CASE WHEN objects.type = 'task' THEN COALESCE(t.status, 'open') END, \
         CASE WHEN objects.type = 'task' THEN COALESCE(t.priority, 4) END, \
         t.scheduled, t.deadline, t.estimate, vp.id, vp.title, vs.id, vs.title, va.id, va.title, \
         (SELECT MIN(tag) FROM object_tags WHERE object_id = objects.id) \
         FROM {FROM} {JOINS} WHERE {}",
        compiled.sql_where
    );
    let mut stmt = conn.prepare(&sql).map_err(db::Error::Sqlite)?;
    let pair = |r: &rusqlite::Row, i: usize| -> rusqlite::Result<Option<(i64, String)>> {
        Ok(r.get::<_, Option<i64>>(i)?
            .zip(r.get::<_, Option<String>>(i + 1)?))
    };
    let keys = stmt
        .query_map(params_from_iter(&args), |r| {
            Ok(Key {
                id: r.get(0)?,
                r#type: r.get(1)?,
                title: r.get(2)?,
                created: r.get(3)?,
                updated: r.get(4)?,
                completed: r.get(5)?,
                status: r.get(6)?,
                priority: r.get(7)?,
                scheduled: r.get(8)?,
                deadline: r.get(9)?,
                estimate: r.get(10)?,
                project: pair(r, 11)?,
                section: pair(r, 13)?,
                area: pair(r, 15)?,
                tag: r.get(17)?,
            })
        })
        .map_err(db::Error::Sqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db::Error::Sqlite)?;
    Ok(keys)
}

/// The SQL values of `params`, each text term bound as the JSON array of its matching ids,
/// as `query::run` binds them.
fn bind(conn: &Connection, params: &[Param]) -> Result<Vec<Value>, db::Error> {
    params
        .iter()
        .map(|p| match p {
            Param::Value(v) => Ok(v.clone()),
            Param::Search(text) => {
                let ids = search::matching_ids(conn, text)?;
                Ok(Value::Text(serde_json::Value::from(ids).to_string()))
            }
        })
        .collect()
}

/// Sorts `keys` by `sort` (ADR 0016): `smart` by smart score, highest first; `created`,
/// `updated` and `completed` newest first; the rest ascending. Unset values go last. Ties go
/// by id, ascending, except newest-first sorts, where they go by id descending.
pub fn sort(keys: &mut [Key], sort: Sort, today: NaiveDate) {
    match sort {
        Sort::Smart => {
            keys.sort_by_cached_key(|k| (Score(smart(k, today)), k.id));
        }
        Sort::Title => keys.sort_by_cached_key(|k| (k.title.to_lowercase(), k.id)),
        Sort::Priority => keys.sort_by(|a, b| asc(&a.priority, &b.priority).then(a.id.cmp(&b.id))),
        Sort::Deadline => keys.sort_by(|a, b| asc(&a.deadline, &b.deadline).then(a.id.cmp(&b.id))),
        Sort::Scheduled => {
            keys.sort_by(|a, b| asc(&a.scheduled, &b.scheduled).then(a.id.cmp(&b.id)))
        }
        Sort::Created => keys.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id))),
        Sort::Updated => keys.sort_by(|a, b| b.updated.cmp(&a.updated).then(b.id.cmp(&a.id))),
        Sort::Completed => {
            keys.sort_by(|a, b| desc(&a.completed, &b.completed).then(b.id.cmp(&a.id)))
        }
    }
}

/// The smart score of a task; other objects score as a task with the defaults.
fn smart(k: &Key, today: NaiveDate) -> f64 {
    let fields = Fields {
        status: k.status.clone().unwrap_or_default(),
        priority: k.priority.unwrap_or(4),
        scheduled: k.scheduled.clone(),
        deadline: k.deadline.clone(),
        estimate: k.estimate,
        someday: false,
        parent_id: None,
        project_id: k.project.as_ref().map(|p| p.0),
        section_id: k.section.as_ref().map(|s| s.0),
        area_id: k.area.as_ref().map(|a| a.0),
        checklist: Vec::new(),
        recurrence: None,
        completed: None,
    };
    smartsort::score(&fields, today)
}

/// A score ordered high to low.
#[derive(PartialEq)]
struct Score(f64);

impl Eq for Score {}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0)
    }
}

/// Ascending, `None` last.
fn asc<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (a, b) => a.is_none().cmp(&b.is_none()),
    }
}

/// Descending, `None` last.
fn desc<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.cmp(a),
        _ => asc(a, b),
    }
}

/// The date part of a `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM` task date.
pub fn date(value: Option<&str>) -> Option<NaiveDate> {
    value
        .and_then(|v| v.get(..10))
        .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_sorts_last_both_ways() {
        let mut v = vec![None, Some(2), Some(1), None];
        v.sort_by(asc);
        assert_eq!(v, [Some(1), Some(2), None, None]);
        v.sort_by(desc);
        assert_eq!(v, [Some(2), Some(1), None, None]);
    }

    #[test]
    fn date_reads_the_day_of_a_date_time() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 3);
        assert_eq!(date(Some("2026-11-03T17:30")), d);
        assert_eq!(date(Some("2026-11-03")), d);
        assert_eq!(date(Some("bad")), None);
        assert_eq!(date(None), None);
    }
}
