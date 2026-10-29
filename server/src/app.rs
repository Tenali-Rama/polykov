//! HTTP router and handlers.

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

/// Builds the full HTTP router. Binds no port, so tests can drive it directly.
pub fn router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}
