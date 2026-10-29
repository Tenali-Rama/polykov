use std::fs;
use std::path::PathBuf;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use polykov_server::auth::session;
use polykov_server::db::{self, Db};
use rusqlite::Connection;
use serde_json::{Value, json};
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";

/// A fresh data dir with a migrated database, removed on drop.
struct DataDir(PathBuf);

impl DataDir {
    fn new(name: &str) -> (Self, Db) {
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
    fn sql(&self) -> Connection {
        Connection::open(self.0.join(db::FILE_NAME)).unwrap()
    }
}

impl Drop for DataDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Reply {
    status: StatusCode,
    cookie: Option<String>,
    body: Vec<u8>,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

async fn send(db: &Db, req: Request<Body>) -> Reply {
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

async fn get(db: &Db, path: &str) -> Reply {
    send(db, Request::get(path).body(Body::empty()).unwrap()).await
}

async fn post(db: &Db, path: &str, body: Value) -> Reply {
    let req = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    send(db, req).await
}

async fn setup(db: &Db, password: &str) -> Reply {
    post(db, "/api/setup", json!({ "password": password })).await
}

async fn login(db: &Db, password: &str) -> Reply {
    post(db, "/api/login", json!({ "password": password })).await
}

#[tokio::test]
async fn setup_is_needed_until_owner_exists() {
    let (_dir, db) = DataDir::new("needed");
    let before = get(&db, "/api/setup").await;
    assert_eq!(before.status, StatusCode::OK);
    assert_eq!(before.json(), json!({ "needed": true }));
    assert_eq!(setup(&db, PASSWORD).await.status, StatusCode::CREATED);
    let after = get(&db, "/api/setup").await;
    assert_eq!(after.status, StatusCode::OK);
    assert_eq!(after.json(), json!({ "needed": false }));
}

#[tokio::test]
async fn setup_stores_argon2id_hash_not_password() {
    let (dir, db) = DataDir::new("hash");
    assert_eq!(setup(&db, PASSWORD).await.status, StatusCode::CREATED);
    let hash: String = dir
        .sql()
        .query_row("SELECT password_hash FROM owner", [], |r| r.get(0))
        .unwrap();
    assert!(hash.starts_with("$argon2id$"), "{hash}");
    assert!(!hash.contains(PASSWORD));
}

#[tokio::test]
async fn second_setup_returns_409_and_keeps_first_password() {
    let (dir, db) = DataDir::new("twice");
    assert_eq!(setup(&db, PASSWORD).await.status, StatusCode::CREATED);
    let again = setup(&db, "another long password").await;
    assert_eq!(again.status, StatusCode::CONFLICT);
    let owners: i64 = dir
        .sql()
        .query_row("SELECT COUNT(*) FROM owner", [], |r| r.get(0))
        .unwrap();
    assert_eq!(owners, 1);
    assert_eq!(login(&db, PASSWORD).await.status, StatusCode::OK);
}

#[tokio::test]
async fn short_password_returns_400_and_creates_nothing() {
    let (_dir, db) = DataDir::new("short");
    // 11 characters, 22 bytes: the limit counts characters, not bytes.
    let short = setup(&db, "ééééééééééé").await;
    assert_eq!(short.status, StatusCode::BAD_REQUEST);
    assert!(short.json()["error"].as_str().unwrap().contains("12"));
    assert_eq!(
        get(&db, "/api/setup").await.json(),
        json!({ "needed": true })
    );
    assert_eq!(setup(&db, "123456789012").await.status, StatusCode::CREATED);
}

#[tokio::test]
async fn login_returns_token_and_session_cookie() {
    let (_dir, db) = DataDir::new("login");
    setup(&db, PASSWORD).await;
    let res = login(&db, PASSWORD).await;
    assert_eq!(res.status, StatusCode::OK);
    let token = res.json()["token"].as_str().unwrap().to_owned();
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    let cookie = res.cookie.unwrap();
    let mut parts = cookie.split("; ");
    assert_eq!(
        parts.next(),
        Some(format!("polykov_session={token}").as_str())
    );
    let attrs: Vec<&str> = parts.collect();
    for attr in ["HttpOnly", "SameSite=Strict", "Path=/"] {
        assert!(attrs.contains(&attr), "{cookie}");
    }
    assert!(session::is_valid(&db, &token).unwrap());
    assert_ne!(login(&db, PASSWORD).await.json()["token"], json!(token));
}

#[tokio::test]
async fn wrong_password_and_no_owner_return_bare_401() {
    let (_dir, db) = DataDir::new("401");
    let none = login(&db, PASSWORD).await;
    setup(&db, PASSWORD).await;
    let wrong = login(&db, "not the right password").await;
    for res in [none, wrong] {
        assert_eq!(res.status, StatusCode::UNAUTHORIZED);
        assert!(res.body.is_empty());
        assert!(res.cookie.is_none());
    }
}

#[tokio::test]
async fn sessions_store_only_a_hash_expiring_in_30_days() {
    let (dir, db) = DataDir::new("store");
    setup(&db, PASSWORD).await;
    let token = login(&db, PASSWORD).await.json()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let conn = dir.sql();
    let (hash, ttl): (String, i64) = conn
        .query_row(
            "SELECT token_hash, expires_at - created_at FROM sessions",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(hash, session::hash_token(&token));
    assert_eq!(ttl, 30 * 24 * 60 * 60);
    let dump: String = conn
        .query_row(
            "SELECT group_concat(token_hash || created_at || expires_at) FROM sessions",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!dump.contains(&token));
}

#[tokio::test]
async fn expired_or_unknown_session_is_not_valid() {
    let (dir, db) = DataDir::new("expired");
    let token = session::create(&db).unwrap();
    assert!(session::is_valid(&db, &token).unwrap());
    assert!(!session::is_valid(&db, "not-a-token").unwrap());
    dir.sql()
        .execute("UPDATE sessions SET expires_at = unixepoch() - 1", [])
        .unwrap();
    assert!(!session::is_valid(&db, &token).unwrap());
}
