use std::fs;
use std::path::{Path, PathBuf};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use polykov_server::db::Db;
use polykov_server::web::NOT_BUILT;
use serde_json::{Value, json};
use tower::ServiceExt;

const INDEX: &str = "<!doctype html><title>fake polykov</title>";
const APP_JS: &str = "console.log('fake');";
const SECRET: &str = "secret = \"outside web_dir\"";

/// A temp root holding a data dir, a `config.toml` that must stay private, and a fake built
/// web UI in `web/`. Removed on drop.
struct Root(PathBuf);

impl Root {
    fn new(name: &str, with_web: bool) -> (Self, Db) {
        let root = std::env::temp_dir().join(format!(
            "polykov-web-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(root.join("config.toml"), SECRET).unwrap();
        if with_web {
            fs::create_dir_all(root.join("web/assets")).unwrap();
            fs::write(root.join("web/index.html"), INDEX).unwrap();
            fs::write(root.join("web/assets/app.js"), APP_JS).unwrap();
            fs::write(root.join("web/assets/app.css"), "body{}").unwrap();
        }
        let db = Db::open(&root.join("data")).unwrap();
        (Root(root), db)
    }

    fn web_dir(&self) -> PathBuf {
        self.0.join("web")
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Reply {
    status: StatusCode,
    content_type: String,
    body: String,
}

async fn get(db: &Db, web_dir: &Path, path: &str) -> Reply {
    let req = Request::get(path).body(Body::empty()).unwrap();
    let res = polykov_server::app::router(db.clone(), web_dir)
        .oneshot(req)
        .await
        .unwrap();
    let status = res.status();
    let content_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    Reply {
        status,
        content_type,
        body: String::from_utf8(body).unwrap(),
    }
}

#[tokio::test]
async fn root_serves_index_html() {
    let (root, db) = Root::new("index", true);
    let r = get(&db, &root.web_dir(), "/").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.content_type.starts_with("text/html"),
        "{}",
        r.content_type
    );
    assert_eq!(r.body, INDEX);
}

#[tokio::test]
async fn assets_are_served_with_their_content_type() {
    let (root, db) = Root::new("assets", true);
    let js = get(&db, &root.web_dir(), "/assets/app.js").await;
    assert_eq!(js.status, StatusCode::OK);
    assert!(
        js.content_type.starts_with("text/javascript"),
        "{}",
        js.content_type
    );
    assert_eq!(js.body, APP_JS);
    let css = get(&db, &root.web_dir(), "/assets/app.css").await;
    assert_eq!(css.status, StatusCode::OK);
    assert!(
        css.content_type.starts_with("text/css"),
        "{}",
        css.content_type
    );
}

#[tokio::test]
async fn unknown_non_api_paths_fall_back_to_index_html() {
    let (root, db) = Root::new("spa", true);
    for path in ["/notes/42", "/settings", "/assets/missing.js", "/api-docs"] {
        let r = get(&db, &root.web_dir(), path).await;
        assert_eq!(r.status, StatusCode::OK, "{path}");
        assert!(r.content_type.starts_with("text/html"), "{path}");
        assert_eq!(r.body, INDEX, "{path}");
    }
}

#[tokio::test]
async fn unknown_api_and_health_paths_return_json_404() {
    let (root, db) = Root::new("api404", true);
    for path in [
        "/api",
        "/api/",
        "/api/no-such",
        "/api/setup/extra",
        "/health/",
        "/health/extra",
    ] {
        let r = get(&db, &root.web_dir(), path).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{path}");
        let body: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(body, json!({ "error": "not found" }), "{path}");
    }
}

#[tokio::test]
async fn api_and_health_still_work_next_to_the_web_ui() {
    let (root, db) = Root::new("api", true);
    let health = get(&db, &root.web_dir(), "/health").await;
    assert_eq!(health.status, StatusCode::OK);
    assert!(health.content_type.starts_with("application/json"));
    let setup = get(&db, &root.web_dir(), "/api/setup").await;
    assert_eq!(setup.status, StatusCode::OK);
    assert_eq!(
        serde_json::from_str::<Value>(&setup.body).unwrap(),
        json!({ "needed": true })
    );
}

#[tokio::test]
async fn path_traversal_cannot_read_outside_web_dir() {
    let (root, db) = Root::new("traversal", true);
    let paths = [
        "/../config.toml",
        "/assets/../../config.toml",
        "/%2e%2e/config.toml",
        "/%2E%2E/config.toml",
        "/..%2fconfig.toml",
        "/assets/..%2f..%2fconfig.toml",
        "/%2e%2e%2f%2e%2e%2fconfig.toml",
        "/..%5cconfig.toml",
        "/%252e%252e/config.toml",
        "//config.toml",
    ];
    for path in paths {
        let r = get(&db, &root.web_dir(), path).await;
        assert!(
            !r.body.contains("outside web_dir"),
            "{path} leaked config.toml"
        );
        assert_eq!(r.status, StatusCode::OK, "{path}");
        assert_eq!(r.body, INDEX, "{path}");
    }
}

#[tokio::test]
async fn missing_web_dir_returns_503_for_web_paths_only() {
    let (root, db) = Root::new("missing", false);
    for path in ["/", "/notes/42", "/assets/app.js"] {
        let r = get(&db, &root.web_dir(), path).await;
        assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
        assert!(r.content_type.starts_with("text/plain"), "{path}");
        assert_eq!(r.body, NOT_BUILT, "{path}");
    }
    let health = get(&db, &root.web_dir(), "/health").await;
    assert_eq!(health.status, StatusCode::OK);
    let setup = get(&db, &root.web_dir(), "/api/setup").await;
    assert_eq!(setup.status, StatusCode::OK);
    let unknown = get(&db, &root.web_dir(), "/api/no-such").await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(
        serde_json::from_str::<Value>(&unknown.body).unwrap(),
        json!({ "error": "not found" })
    );
}

#[tokio::test]
async fn non_get_web_requests_return_405() {
    let (root, db) = Root::new("method", true);
    let req = Request::post("/notes/42").body(Body::empty()).unwrap();
    let res = polykov_server::app::router(db, &root.web_dir())
        .oneshot(req)
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
}
