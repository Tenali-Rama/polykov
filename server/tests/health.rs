use std::fs;
use std::path::PathBuf;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use polykov_server::db::{self, Db};
use serde_json::{Value, json};
use tower::ServiceExt;

/// A fresh data dir with a migrated database, removed on drop.
struct DataDir(PathBuf);

impl DataDir {
    fn new(name: &str) -> (Self, Db) {
        let dir = std::env::temp_dir().join(format!(
            "polykov-health-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir).unwrap();
        (DataDir(dir), db)
    }
}

impl Drop for DataDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn get(db: Db, path: &str) -> (StatusCode, Vec<u8>) {
    let req = Request::get(path).body(Body::empty()).unwrap();
    let res = polykov_server::app::router(db).oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, body)
}

#[tokio::test]
async fn health_returns_ok_version_and_db() {
    let (_dir, db) = DataDir::new("ok");
    let (status, body) = get(db, "/health").await;
    assert_eq!(status, StatusCode::OK);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION"), "db": "ok" })
    );
}

#[tokio::test]
async fn health_returns_503_when_db_query_fails() {
    let (dir, db) = DataDir::new("broken");
    rusqlite::Connection::open(dir.0.join(db::FILE_NAME))
        .unwrap()
        .execute_batch("DROP TABLE schema_version;")
        .unwrap();
    let (status, body) = get(db, "/health").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "status": "error", "version": env!("CARGO_PKG_VERSION"), "db": "error" })
    );
}

#[tokio::test]
async fn unknown_path_returns_404() {
    let (_dir, db) = DataDir::new("404");
    let (status, _) = get(db, "/no-such-path").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
