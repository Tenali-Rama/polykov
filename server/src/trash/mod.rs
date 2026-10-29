//! The trash (spec §3): `GET`/`DELETE /api/trash`, `POST /api/trash/{id}/restore` and
//! `DELETE /api/trash/{id}`. Every route requires auth. [`spawn_purger`] deletes objects for
//! good 30 days after they were trashed.

pub mod store;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use serde_json::{Value, json};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    objects::store::Object,
};
pub use store::purge_expired;

/// How often [`spawn_purger`] runs [`purge_expired`] after the run at start-up.
pub const PURGE_EVERY: Duration = Duration::from_secs(60 * 60);

/// The trash routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/trash", get(list).delete(empty))
        .route("/api/trash/{id}", delete(purge))
        .route("/api/trash/{id}/restore", post(restore))
}

/// Starts a thread that runs [`purge_expired`] now and then every [`PURGE_EVERY`]. A failed
/// run logs one line and is retried at the next tick.
pub fn spawn_purger(db: Db) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("trash-purger".to_owned())
        .spawn(move || {
            loop {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64);
                if let Err(e) = purge_expired(&db, now) {
                    eprintln!("polykov-server: trash purge failed: {e}");
                }
                std::thread::sleep(PURGE_EVERY);
            }
        })
        .map(drop)
}

fn not_in_trash() -> ApiError {
    ApiError::not_found("not in trash")
}

async fn list(State(db): State<Db>, _: Session) -> Result<Json<Value>, ApiError> {
    let objects = blocking(move || store::list(&db)).await?;
    Ok(Json(json!({ "objects": objects })))
}

async fn restore(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Object>, ApiError> {
    blocking(move || store::restore(&db, id))
        .await?
        .map(Json)
        .ok_or_else(not_in_trash)
}

async fn purge(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if blocking(move || store::purge(&db, id)).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_in_trash())
    }
}

async fn empty(State(db): State<Db>, _: Session) -> Result<Json<Value>, ApiError> {
    let purged = blocking(move || store::empty(&db)).await?;
    Ok(Json(json!({ "purged": purged })))
}
