//! HTTP router and handlers.

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde_json::{Value, json};

use crate::db::Db;

/// Builds the full HTTP router over `db`. Binds no port, so tests can drive it directly.
pub fn router(db: Db) -> Router {
    Router::new().route("/health", get(health)).with_state(db)
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
