//! Global search (spec "Organization"): `GET /api/search`, full text with fuzzy matching over
//! every live object, archived and completed included. Requires auth. Other modules filter
//! by text with [`store::matching_ids`].

pub mod fuzzy;
pub mod store;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    objects, projects, tasks,
};

pub use store::matching_ids;

/// Longest accepted query, in characters, after trimming.
pub const MAX_QUERY_CHARS: usize = 200;
/// Page size when `limit` is not given.
pub const DEFAULT_LIMIT: u32 = 20;
/// Largest accepted `limit`.
pub const MAX_LIMIT: u32 = 100;

/// The search route, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new().route("/api/search", get(search))
}

/// Query string of `GET /api/search`; every value is checked by [`parse`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    q: Option<String>,
    r#type: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

fn parse(p: Params) -> Result<store::Query, ApiError> {
    let text = p.q.as_deref().unwrap_or_default().trim();
    if text.is_empty() || text.chars().count() > MAX_QUERY_CHARS {
        return Err(ApiError::bad_request(format!(
            "q must be 1 to {MAX_QUERY_CHARS} characters"
        )));
    }
    if let Some(kind) = &p.r#type {
        objects::validate::object_type(kind)?;
    }
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
            .map_err(|_| ApiError::bad_request("offset must be a non-negative integer"))?,
    };
    Ok(store::Query {
        text: text.to_owned(),
        r#type: p.r#type,
        limit,
        offset,
    })
}

async fn search(
    State(db): State<Db>,
    _: Session,
    Query(params): Query<Params>,
) -> Result<Json<Value>, ApiError> {
    let query = parse(params)?;
    let body = blocking(move || {
        let page = store::search(&db, &query)?;
        let mut results = Vec::with_capacity(page.hits.len());
        for hit in page.hits {
            if let Some(object) = object_json(&db, hit.id)? {
                results
                    .push(json!({ "object": object, "snippet": hit.snippet, "score": hit.score }));
            }
        }
        Ok::<_, ApiError>(json!({ "results": results, "total": page.total }))
    })
    .await?;
    Ok(Json(body))
}

/// The live object `id` as its own routes serve it (a task or project with its fields), or
/// `None` if it was trashed meanwhile.
fn object_json(db: &Db, id: i64) -> Result<Option<Value>, ApiError> {
    let Some(object) = objects::store::get(db, id)? else {
        return Ok(None);
    };
    let value = match object.r#type.as_str() {
        "task" => tasks::store::get(db, id)?.map(serde_json::to_value),
        "project" => projects::store::get(db, id)?.map(serde_json::to_value),
        _ => Some(serde_json::to_value(object)),
    };
    value
        .transpose()
        .map_err(|e| ApiError::Internal(format!("search result {id}: {e}")))
}
