//! `GET /api/views/{id}/items`: a view's matches, sorted and grouped for its layout.

use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    keys::{self, Key},
    spec::{self, Group, Layout},
};
use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    objects::store as objects,
    query::Item,
    tasks::{store as tasks, validate},
};

/// Most items one response carries, across all groups (owner may change).
pub const MAX_ITEMS: usize = 500;
/// Eisenhower: a task is important at this priority or higher (lower number).
pub const IMPORTANT_PRIORITY: i64 = 2;
/// Eisenhower: a task is urgent when overdue or its deadline or scheduled date is at most
/// this many days after today (owner may change).
pub const URGENT_DAYS: u64 = 2;

/// Query string of the items route.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Params {
    today: Option<String>,
}

/// One group of items, in the view's sort order.
#[derive(Serialize)]
struct Bucket {
    key: Value,
    label: String,
    items: Vec<Item>,
}

/// Where a group goes: empty-key groups last, then by number, then text, then id.
type Rank = (bool, i64, String, i64);

pub(super) async fn items(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
    Query(params): Query<Params>,
) -> Result<Response, ApiError> {
    let today = match params.today {
        None => validate::today(),
        Some(raw) => NaiveDate::parse_from_str(&raw, "%Y-%m-%d")
            .ok()
            .filter(|_| raw.len() == 10)
            .ok_or_else(|| ApiError::bad_request("today must be a date YYYY-MM-DD"))?,
    };
    blocking(move || build(&db, id, today)).await
}

fn build(db: &Db, id: i64, today: NaiveDate) -> Result<Response, ApiError> {
    let view = super::find(db, id)?.ok_or_else(super::not_found)?;
    let spec = match spec::read(&view.properties) {
        Ok(spec) => spec,
        Err(e) => return Ok(e.respond(StatusCode::UNPROCESSABLE_ENTITY)),
    };
    let mut keys = keys::select(db, &spec.query, today)?;
    keys::sort(&mut keys, spec.sort, today);
    let total = keys.len();
    keys.truncate(MAX_ITEMS);
    let groups = match (spec.layout, spec.group) {
        (Layout::Eisenhower, _) => eisenhower(db, &keys, today)?,
        (Layout::Kanban, Group::None) => grouped(db, &keys, Group::Status)?,
        (_, group) => grouped(db, &keys, group)?,
    };
    let body = json!({
        "view": view,
        "groups": groups,
        "total": total,
        "truncated": total > MAX_ITEMS,
    });
    Ok(Json(body).into_response())
}

/// The keys in groups by `by`, each group keeping their order.
fn grouped(db: &Db, keys: &[Key], by: Group) -> Result<Vec<Bucket>, ApiError> {
    let mut groups: BTreeMap<Rank, Bucket> = BTreeMap::new();
    for k in keys {
        let (rank, key, label) = place(k, by);
        let bucket = groups.entry(rank).or_insert_with(|| Bucket {
            key,
            label,
            items: Vec::new(),
        });
        bucket.items.extend(load(db, k)?);
    }
    Ok(groups.into_values().collect())
}

/// The group of `k` under `by`: its rank, key and label. No value is key `null`, label `none`.
fn place(k: &Key, by: Group) -> (Rank, Value, String) {
    let titled = |r: &Option<(i64, String)>| {
        r.as_ref().map(|(id, title)| {
            let rank = (false, 0, title.to_lowercase(), *id);
            (rank, json!(id), title.clone())
        })
    };
    let named = |name: &str| ((false, 0, name.to_owned(), 0), json!(name), name.to_owned());
    let placed = match by {
        Group::None => Some(((false, 0, String::new(), 0), Value::Null, "all".to_owned())),
        Group::Status => k.status.as_deref().map(|s| {
            let open = i64::from(s != "open");
            ((false, open, String::new(), 0), json!(s), s.to_owned())
        }),
        Group::Priority => k
            .priority
            .map(|p| ((false, p, String::new(), 0), json!(p), format!("P{p}"))),
        Group::Project => titled(&k.project),
        Group::Section => titled(&k.section),
        Group::Area => titled(&k.area),
        Group::Tag => k.tag.as_deref().map(named),
        Group::Type => Some(named(&k.r#type)),
    };
    placed.unwrap_or_else(|| ((true, 0, String::new(), 0), Value::Null, "none".to_owned()))
}

/// The four Eisenhower quadrants, always all four, in order.
fn eisenhower(db: &Db, keys: &[Key], today: NaiveDate) -> Result<Vec<Bucket>, ApiError> {
    let quadrants = [
        ("urgent_important", "urgent & important"),
        ("important", "important"),
        ("urgent", "urgent"),
        ("neither", "neither"),
    ];
    let mut groups: Vec<Bucket> = quadrants
        .iter()
        .map(|(key, label)| Bucket {
            key: json!(key),
            label: (*label).to_owned(),
            items: Vec::new(),
        })
        .collect();
    let horizon = today.checked_add_days(Days::new(URGENT_DAYS));
    for k in keys {
        let important = k.priority.is_some_and(|p| p <= IMPORTANT_PRIORITY);
        let soonest = [&k.deadline, &k.scheduled]
            .into_iter()
            .filter_map(|d| keys::date(d.as_deref()))
            .min();
        let urgent = soonest.is_some_and(|d| horizon.is_none_or(|h| d <= h));
        let quadrant = match (urgent, important) {
            (true, true) => 0,
            (false, true) => 1,
            (true, false) => 2,
            (false, false) => 3,
        };
        groups[quadrant].items.extend(load(db, k)?);
    }
    Ok(groups)
}

/// The full JSON of a key's object: the task JSON for a task. `None` if it went in between.
fn load(db: &Db, k: &Key) -> Result<Option<Item>, ApiError> {
    Ok(if k.r#type == "task" {
        tasks::get(db, k.id)?.map(|t| Item::Task(Box::new(t)))
    } else {
        objects::get(db, k.id)?.map(Item::Object)
    })
}
