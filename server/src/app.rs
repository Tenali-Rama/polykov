//! HTTP router and handlers.

use std::path::Path;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{any, get},
};
use serde_json::{Value, json};

use crate::{auth, db::Db, web};

/// Builds the full HTTP router over `db`, serving the web UI from `web_dir` for every path
/// the API does not claim. Binds no port, so tests can drive it directly.
pub fn router(db: Db, web_dir: &Path) -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(auth::routes())
        .merge(crate::objects::routes())
        .merge(crate::links::routes())
        .merge(crate::comments::routes())
        .merge(crate::attachments::routes())
        .route("/api", any(not_found))
        .route("/api/", any(not_found))
        .route("/api/{*rest}", any(not_found))
        .route("/health/", any(not_found))
        .route("/health/{*rest}", any(not_found))
        .with_state(db)
        .fallback_service(web::service(web_dir))
}

/// Unknown API paths get a JSON 404 rather than the web UI.
async fn not_found() -> (StatusCode, Json<Value>) {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" })))
}

async fn health(State(db): State<Db>) -> (StatusCode, Json<Value>) {
    let ok = matches!(
        tokio::task::spawn_blocking(move || db.check()).await,
        Ok(Ok(()))
    );
    let (code, status) = if ok {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "error")
    };
    let body = json!({ "status": status, "version": env!("CARGO_PKG_VERSION"), "db": status });
    (code, Json(body))
}
