//! Helpers shared by the integration tests: a throwaway data dir and one-shot requests
//! against `app::router`.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use polykov_server::db::{self, Db};
use rusqlite::Connection;
use serde_json::{Value, json};
use tower::ServiceExt;

pub const PASSWORD: &str = "correct horse battery";

/// A fresh data dir with a migrated database, removed on drop.
pub struct DataDir(PathBuf);

impl DataDir {
    pub fn new(name: &str) -> (Self, Db) {
        let dir = std::env::temp_dir().join(format!(
            "polykov-auth-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir).unwrap();
        (DataDir(dir), db)
    }

    /// A second connection to the same database, for inspecting tables directly.
    pub fn sql(&self) -> Connection {
        Connection::open(self.0.join(db::FILE_NAME)).unwrap()
    }
}

impl Drop for DataDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub struct Reply {
    pub status: StatusCode,
    pub cookie: Option<String>,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

pub async fn send(db: &Db, req: Request<Body>) -> Reply {
    let res = polykov_server::app::router(db.clone(), &std::env::temp_dir().join("polykov-no-web"))
        .oneshot(req)
        .await
        .unwrap();
    let status = res.status();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned());
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    Reply {
        status,
        cookie,
        body,
    }
}

pub async fn get(db: &Db, path: &str) -> Reply {
    send(db, Request::get(path).body(Body::empty()).unwrap()).await
}

pub async fn post(db: &Db, path: &str, body: Value) -> Reply {
    let req = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    send(db, req).await
}

pub async fn setup(db: &Db, password: &str) -> Reply {
    post(db, "/api/setup", json!({ "password": password })).await
}

pub async fn login(db: &Db, password: &str) -> Reply {
    post(db, "/api/login", json!({ "password": password })).await
}

/// Sets up the owner and logs in; returns the session token.
pub async fn token(db: &Db) -> String {
    setup(db, PASSWORD).await;
    login(db, PASSWORD).await.json()["token"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Sends `method path`, with `token` as a bearer token and `body` as JSON when given.
pub async fn call(
    db: &Db,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Reply {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(body) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    send(db, req.body(body).unwrap()).await
}

/// Creates an object through the API and returns its id.
pub async fn create_object(db: &Db, token: &str, kind: &str, title: &str) -> i64 {
    let body = json!({ "type": kind, "title": title });
    let res = call(db, Method::POST, "/api/objects", Some(token), Some(body)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    res.json()["id"].as_i64().unwrap()
}

/// Creates a task through `POST /api/tasks` with `task` as its task fields (`json!({})` for
/// the defaults) and returns its id.
pub async fn create_task(db: &Db, token: &str, title: &str, task: Value) -> i64 {
    let body = json!({ "title": title, "task": task });
    let res = call(db, Method::POST, "/api/tasks", Some(token), Some(body)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    res.json()["id"].as_i64().unwrap()
}
