//! Comments on any object (spec §3): `POST`/`GET /api/objects/{id}/comments` and
//! `PATCH`/`DELETE /api/comments/{cid}`. Every route requires auth.

pub mod store;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    objects,
};
use store::Comment;

/// Maximum comment body length after trimming, in characters.
pub const MAX_BODY_CHARS: usize = 10_000;

/// The comment routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/objects/{id}/comments", get(list).post(create))
        .route("/api/comments/{cid}", patch(update).delete(remove))
}

/// Body of `POST /api/objects/{id}/comments` and `PATCH /api/comments/{cid}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyIn {
    body: String,
}

#[derive(Serialize)]
struct CommentList {
    comments: Vec<Comment>,
}

/// Returns the trimmed body if it is 1 to [`MAX_BODY_CHARS`] characters.
fn body(body: &str) -> Result<String, ApiError> {
    let body = body.trim();
    let len = body.chars().count();
    if len == 0 || len > MAX_BODY_CHARS {
        return Err(ApiError::bad_request(format!(
            "body must be 1 to {MAX_BODY_CHARS} characters"
        )));
    }
    Ok(body.to_owned())
}

/// Fails with `404 {"error":"object not found"}` unless `id` is a live object.
fn require_live(db: &Db, id: i64) -> Result<(), ApiError> {
    match objects::store::summary(db, id)? {
        Some(_) => Ok(()),
        None => Err(ApiError::not_found("object not found")),
    }
}

fn not_found() -> ApiError {
    ApiError::not_found("comment not found")
}

async fn create(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
    Json(input): Json<BodyIn>,
) -> Result<(StatusCode, Json<Comment>), ApiError> {
    let text = body(&input.body)?;
    let comment = blocking(move || {
        require_live(&db, id)?;
        Ok::<_, ApiError>(store::create(&db, id, &text)?)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

async fn list(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<CommentList>, ApiError> {
    let comments = blocking(move || {
        require_live(&db, id)?;
        Ok::<_, ApiError>(store::list(&db, id)?)
    })
    .await?;
    Ok(Json(CommentList { comments }))
}

async fn update(
    State(db): State<Db>,
    _: Session,
    Path(cid): Path<i64>,
    Json(input): Json<BodyIn>,
) -> Result<Json<Comment>, ApiError> {
    let text = body(&input.body)?;
    blocking(move || store::update(&db, cid, &text))
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

async fn remove(
    State(db): State<Db>,
    _: Session,
    Path(cid): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if blocking(move || store::delete(&db, cid)).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}
