//! `GET /api/query`: the objects matching a view query. Requires auth.

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;

use super::{compile::run, parse};
use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    tasks::validate,
};

/// Page size when `limit` is not given.
pub const DEFAULT_LIMIT: u32 = 100;
/// Largest accepted `limit`.
pub const MAX_LIMIT: u32 = 1000;
/// Longest accepted `q`, in characters.
pub const MAX_QUERY_CHARS: usize = 1000;

/// The query route, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new().route("/api/query", get(query))
}

/// Query string of `GET /api/query`; every value is checked in [`query`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    q: Option<String>,
    today: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

/// `today`, `limit` and `offset`, checked with the messages of the lists API.
fn paging(p: &Params) -> Result<(NaiveDate, u32, i64), ApiError> {
    let today = match &p.today {
        None => validate::today(),
        Some(raw) => NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .ok()
            .filter(|_| raw.len() == 10)
            .ok_or_else(|| ApiError::bad_request("today must be a date YYYY-MM-DD"))?,
    };
    let limit = match &p.limit {
        None => DEFAULT_LIMIT,
        Some(raw) => raw
            .parse()
            .ok()
            .filter(|n| (1..=MAX_LIMIT).contains(n))
            .ok_or_else(|| ApiError::bad_request(format!("limit must be 1 to {MAX_LIMIT}")))?,
    };
    let offset = match &p.offset {
        None => 0,
        Some(raw) => raw
            .parse()
            .ok()
            .filter(|&n: &i64| n >= 0)
            .ok_or_else(|| ApiError::bad_request("offset must be a non-negative integer"))?,
    };
    Ok((today, limit, offset))
}

async fn query(
    State(db): State<Db>,
    _: Session,
    Query(params): Query<Params>,
) -> Result<Response, ApiError> {
    let (today, limit, offset) = paging(&params)?;
    let text = params.q.unwrap_or_default();
    if text.chars().count() > MAX_QUERY_CHARS {
        return Err(ApiError::bad_request(format!(
            "q must be at most {MAX_QUERY_CHARS} characters"
        )));
    }
    let query = match parse(&text) {
        Ok(query) => query,
        Err(e) => {
            let body = json!({ "error": e.message, "at": e.offset });
            return Ok((StatusCode::BAD_REQUEST, Json(body)).into_response());
        }
    };
    let page = blocking(move || run(&db, &query, today, limit, offset)).await?;
    Ok(Json(page).into_response())
}
