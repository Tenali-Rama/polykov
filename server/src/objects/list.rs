//! `GET /api/objects`: live objects filtered by type, tag folder, pinned and archived, pinned
//! first, then most recently updated.

use axum::{
    Json,
    extract::{Query, State},
};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{
    store::{self, Object},
    validate,
};
use crate::{
    auth::Session,
    db::{self, Db},
    http::{ApiError, blocking},
    tags,
};

/// Page size when `limit` is not given.
pub const DEFAULT_LIMIT: u32 = 100;
/// Largest accepted `limit`.
pub const MAX_LIMIT: u32 = 1000;

/// Query string of `GET /api/objects`; every value is checked by [`Filter::parse`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Params {
    r#type: Option<String>,
    tag: Option<String>,
    pinned: Option<String>,
    archived: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

/// A validated list request; `None` matches anything.
#[derive(Debug)]
struct Filter {
    r#type: Option<String>,
    tag: Option<String>,
    pinned: Option<bool>,
    archived: Option<bool>,
    limit: u32,
    offset: i64,
}

impl Filter {
    fn parse(p: Params) -> Result<Self, ApiError> {
        if let Some(kind) = &p.r#type {
            validate::object_type(kind)?;
        }
        let flag = |value: Option<String>, name: &str, any: bool| match value.as_deref() {
            None if any => Ok(Some(false)),
            None => Ok(None),
            Some("true") => Ok(Some(true)),
            Some("false") => Ok(Some(false)),
            Some("any") if any => Ok(None),
            Some(_) if any => Err(ApiError::bad_request(format!(
                "{name} must be false, true or any"
            ))),
            Some(_) => Err(ApiError::bad_request(format!(
                "{name} must be true or false"
            ))),
        };
        let limit = match p.limit {
            None => DEFAULT_LIMIT,
            Some(raw) => raw
                .parse()
                .ok()
                .filter(|n| (1..=MAX_LIMIT).contains(n))
                .ok_or_else(|| ApiError::bad_request(format!("limit must be 1 to {MAX_LIMIT}")))?,
        };
        let offset =
            match p.offset {
                None => 0,
                Some(raw) => raw.parse().ok().filter(|&n: &i64| n >= 0).ok_or_else(|| {
                    ApiError::bad_request("offset must be a non-negative integer")
                })?,
            };
        Ok(Filter {
            r#type: p.r#type,
            tag: p.tag.as_deref().map(tags::normalize).transpose()?,
            pinned: flag(p.pinned, "pinned", false)?,
            archived: flag(p.archived, "archived", true)?,
            limit,
            offset,
        })
    }
}

/// One page of the list and the number of matches across all pages.
#[derive(Debug, Serialize)]
pub(super) struct Page {
    objects: Vec<Object>,
    total: i64,
}

pub(super) async fn list(
    State(db): State<Db>,
    _: Session,
    Query(params): Query<Params>,
) -> Result<Json<Page>, ApiError> {
    let filter = Filter::parse(params)?;
    Ok(Json(blocking(move || select(&db, &filter)).await?))
}

fn select(db: &Db, f: &Filter) -> Result<Page, db::Error> {
    let conn = db.conn()?;
    let filter = format!(
        "FROM objects WHERE deleted_at IS NULL AND (?1 IS NULL OR type = ?1) \
         AND (?2 IS NULL OR pinned = ?2) AND (?3 IS NULL OR archived = ?3) \
         AND (?4 IS NULL OR {})",
        tags::store::under(4)
    );
    let args = params![f.r#type, f.pinned, f.archived, f.tag];
    let total = conn
        .query_row(&format!("SELECT COUNT(*) {filter}"), args, |r| r.get(0))
        .map_err(db::Error::Sqlite)?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {} {filter} ORDER BY pinned DESC, updated DESC, id DESC \
             LIMIT ?5 OFFSET ?6",
            store::columns()
        ))
        .map_err(db::Error::Sqlite)?;
    let objects = stmt
        .query_map(
            params![f.r#type, f.pinned, f.archived, f.tag, f.limit, f.offset],
            store::object,
        )
        .map_err(db::Error::Sqlite)?
        .collect::<Result<_, _>>()
        .map_err(db::Error::Sqlite)?;
    Ok(Page { objects, total })
}
