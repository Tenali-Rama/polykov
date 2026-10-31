//! GTD lists (spec "Tasks (GTD)", "Capture"): `GET /api/lists/{name}`, `GET
//! /api/lists/{name}/items` and `GET /api/tasks/{id}/subtasks`. Every route requires auth.

mod layout;
pub mod store;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    tasks::{self, validate},
};
use store::{List, Page, Sort};

/// Page size when `limit` is not given.
pub const DEFAULT_LIMIT: u32 = 100;
/// Largest accepted `limit`.
pub const MAX_LIMIT: u32 = 1000;

/// The list routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/lists/{name}", get(list))
        .route("/api/lists/{name}/items", get(layout::items))
        .route("/api/tasks/{id}/subtasks", get(subtasks))
}

/// Query string of `GET /api/lists/{name}`; every value is checked by [`parse`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    today: Option<String>,
    project: Option<String>,
    area: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
    sort: Option<String>,
}

fn parse(list: List, p: Params) -> Result<store::Query, ApiError> {
    let today = match p.today {
        None => validate::today().format("%Y-%m-%d").to_string(),
        Some(raw) if raw.len() == 10 && NaiveDate::parse_from_str(&raw, "%Y-%m-%d").is_ok() => raw,
        Some(_) => return Err(ApiError::bad_request("today must be a date YYYY-MM-DD")),
    };
    let id = |raw: Option<String>, name: &str| {
        raw.map(|raw| {
            raw.parse().ok().filter(|&n: &i64| n > 0).ok_or_else(|| {
                ApiError::bad_request(format!("{name} must be a positive integer id"))
            })
        })
        .transpose()
    };
    let limit = match p.limit {
        None => DEFAULT_LIMIT,
        Some(raw) => raw
            .parse()
            .ok()
            .filter(|n| (1..=MAX_LIMIT).contains(n))
            .ok_or_else(|| ApiError::bad_request(format!("limit must be 1 to {MAX_LIMIT}")))?,
    };
    let offset = match p.offset {
        None => 0,
        Some(raw) => raw
            .parse()
            .ok()
            .filter(|&n: &i64| n >= 0)
            .ok_or_else(|| ApiError::bad_request("offset must be a non-negative integer"))?,
    };
    let sort = match p.sort.as_deref() {
        None | Some("default") => Sort::Default,
        Some("smart") => Sort::Smart,
        Some(_) => return Err(ApiError::bad_request("sort must be default or smart")),
    };
    Ok(store::Query {
        list,
        today,
        sort,
        project: id(p.project, "project")?,
        area: id(p.area, "area")?,
        limit,
        offset,
    })
}

async fn list(
    State(db): State<Db>,
    _: Session,
    Path(name): Path<String>,
    Query(params): Query<Params>,
) -> Result<Json<Page>, ApiError> {
    let list = List::parse(&name).ok_or_else(|| ApiError::not_found("unknown list"))?;
    let query = parse(list, params)?;
    Ok(Json(blocking(move || store::list(&db, &query)).await?))
}

async fn subtasks(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let rows = blocking(move || match tasks::store::get(&db, id)? {
        Some(_) => store::subtasks(&db, id).map(Some),
        None => Ok(None),
    })
    .await?
    .ok_or_else(|| ApiError::not_found("task not found"))?;
    Ok(Json(json!({ "tasks": rows })))
}
