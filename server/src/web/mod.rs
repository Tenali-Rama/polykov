//! Serves the built web UI from `web_dir` at run time, with SPA fallback to `index.html`.

use std::path::Path;

use axum::{Router, http::StatusCode};
use tower_http::services::{ServeDir, ServeFile};

/// Plain-text body of the `503` sent when the web UI is not built.
pub const NOT_BUILT: &str = "web UI not built: no index.html in web_dir\n";

/// Builds the service for every path the API does not claim. Files under `web_dir` are
/// served as they are; anything else (including paths that try to leave `web_dir`) gets
/// `index.html`. If `web_dir/index.html` is missing, logs one warning and answers `503`.
pub fn service(web_dir: &Path) -> Router {
    let index = web_dir.join("index.html");
    if !index.is_file() {
        eprintln!(
            "polykov-server: warning: no web UI at {} (missing index.html); web paths return 503",
            web_dir.display()
        );
        return Router::new().fallback(not_built);
    }
    Router::new().fallback_service(ServeDir::new(web_dir).fallback(ServeFile::new(index)))
}

async fn not_built() -> (StatusCode, &'static str) {
    (StatusCode::SERVICE_UNAVAILABLE, NOT_BUILT)
}
