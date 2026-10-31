//! `GET /api/lists/{name}/items`: a GTD list grouped for a layout, in the shape of view items
//! (ADR 0022). The grouping is the views' own (`views::items::group`).

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Params as ListParams, parse, store};
use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    query::Item,
    tasks::store::Task,
    views::{
        items::{self, MAX_ITEMS},
        keys::{self, Key},
        spec::{self, GROUPS, Group, LAYOUTS, Layout},
    },
};

/// Query string of the items route: the list filters plus `layout` and `group`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Params {
    today: Option<String>,
    project: Option<String>,
    area: Option<String>,
    sort: Option<String>,
    layout: Option<String>,
    group: Option<String>,
}

pub(super) async fn items(
    State(db): State<Db>,
    _: Session,
    Path(name): Path<String>,
    Query(p): Query<Params>,
) -> Result<Json<Value>, ApiError> {
    let list = store::List::parse(&name).ok_or_else(|| ApiError::not_found("unknown list"))?;
    let layout = pick("layout", p.layout, "list", &LAYOUTS)?;
    let group = pick("group", p.group, "none", &GROUPS)?;
    let mut query = parse(
        list,
        ListParams {
            today: p.today,
            project: p.project,
            area: p.area,
            limit: None,
            offset: None,
            sort: p.sort,
        },
    )?;
    query.limit = MAX_ITEMS as u32;
    Ok(Json(
        blocking(move || build(&db, &query, layout, group)).await?,
    ))
}

/// The option `value` (else `default`) among `options`, as in a view's settings.
fn pick<T: Copy>(
    field: &str,
    value: Option<String>,
    default: &str,
    options: &[(&str, T)],
) -> Result<T, ApiError> {
    spec::pick(field, value.as_deref().unwrap_or(default), options)
        .map_err(|e| ApiError::bad_request(e.message))
}

/// The first [`MAX_ITEMS`] tasks of `query`, grouped for `layout` and `group`.
fn build(db: &Db, query: &store::Query, layout: Layout, group: Group) -> Result<Value, ApiError> {
    let page = store::list(db, query)?;
    // `parse` checked `today`.
    let today = NaiveDate::parse_from_str(&query.today, "%Y-%m-%d").unwrap_or_default();
    let ids: Vec<i64> = page.tasks.iter().map(|r| r.task.object.id).collect();
    let keys = keys::of(db, &ids)?;
    let mut tasks: HashMap<i64, Task> = page
        .tasks
        .into_iter()
        .map(|r| (r.task.object.id, r.task))
        .collect();
    let load = |k: &Key| Ok(tasks.remove(&k.id).map(|t| Item::Task(Box::new(t))));
    let groups = items::group(&keys, layout, group, today, load)?;
    Ok(json!({
        "groups": groups,
        "total": page.total,
        "truncated": page.total > MAX_ITEMS as i64,
    }))
}
