//! Links between objects (spec §3): `PUT`/`DELETE /api/objects/{id}/links/{to}` and `GET
//! /api/objects/{id}/links`, which also computes backlinks. Every route requires auth.

pub mod store;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    objects,
};
use store::Links;

/// The link routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/objects/{id}/links", get(list))
        .route("/api/objects/{id}/links/{to}", put(add).delete(remove))
}

/// Fails with `404 {"error":"object not found"}` unless every id in `ids` is a live object.
fn require_live(db: &Db, ids: &[i64]) -> Result<(), ApiError> {
    for &id in ids {
        if objects::store::summary(db, id)?.is_none() {
            return Err(ApiError::not_found("object not found"));
        }
    }
    Ok(())
}

async fn add(
    State(db): State<Db>,
    _: Session,
    Path((id, to)): Path<(i64, i64)>,
) -> Result<StatusCode, ApiError> {
    if id == to {
        return Err(ApiError::bad_request("cannot link an object to itself"));
    }
    blocking(move || {
        require_live(&db, &[id, to])?;
        Ok::<_, ApiError>(store::add(&db, id, to)?)
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    State(db): State<Db>,
    _: Session,
    Path((id, to)): Path<(i64, i64)>,
) -> Result<StatusCode, ApiError> {
    let removed = blocking(move || {
        require_live(&db, &[id, to])?;
        Ok::<_, ApiError>(store::remove(&db, id, to)?)
    })
    .await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("link not found"))
    }
}

async fn list(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Links>, ApiError> {
    let links = blocking(move || {
        require_live(&db, &[id])?;
        Ok::<_, ApiError>(store::list(&db, id)?)
    })
    .await?;
    Ok(Json(links))
}
