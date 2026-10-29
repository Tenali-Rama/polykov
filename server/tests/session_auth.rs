//! The `auth::Session` extractor through `GET /api/me` and `POST /api/logout`.

mod common;

use axum::{
    body::Body,
    http::{HeaderName, Method, Request, StatusCode, header},
};
use common::{DataDir, PASSWORD, Reply, get, login, post, send, setup};
use polykov_server::db::Db;
use serde_json::json;

/// Sets up the owner and logs in, returning the session token.
async fn session_token(db: &Db) -> String {
    setup(db, PASSWORD).await;
    new_login(db).await
}

async fn new_login(db: &Db) -> String {
    login(db, PASSWORD).await.json()["token"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn with_header(db: &Db, method: Method, path: &str, name: HeaderName, value: &str) -> Reply {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header(name, value)
        .body(Body::empty())
        .unwrap();
    send(db, req).await
}

async fn me_bearer(db: &Db, token: &str) -> Reply {
    let auth = format!("Bearer {token}");
    with_header(db, Method::GET, "/api/me", header::AUTHORIZATION, &auth).await
}

async fn me_cookie(db: &Db, cookie: &str) -> Reply {
    with_header(db, Method::GET, "/api/me", header::COOKIE, cookie).await
}

fn assert_unauthorized(res: &Reply) {
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.json(), json!({ "error": "authentication required" }));
}

#[tokio::test]
async fn me_accepts_bearer_token() {
    let (dir, db) = DataDir::new("me-bearer");
    let token = session_token(&db).await;
    let res = me_bearer(&db, &token).await;
    assert_eq!(res.status, StatusCode::OK);
    let expected: String = dir
        .sql()
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', expires_at, 'unixepoch') FROM sessions",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        res.json(),
        json!({ "owner": true, "session_expires": expected })
    );
    // RFC 3339 UTC, e.g. 2026-11-28T09:15:00Z.
    assert_eq!(expected.len(), 20);
    assert!(expected.ends_with('Z') && expected.as_bytes()[10] == b'T');
}

#[tokio::test]
async fn me_accepts_session_cookie_among_others() {
    let (_dir, db) = DataDir::new("me-cookie");
    let token = session_token(&db).await;
    let res = me_cookie(&db, &format!("theme=dark; polykov_session={token}; x=1")).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["owner"], json!(true));
}

#[tokio::test]
async fn me_without_credentials_returns_401_json() {
    let (_dir, db) = DataDir::new("me-missing");
    session_token(&db).await;
    assert_unauthorized(&get(&db, "/api/me").await);
    assert_unauthorized(&me_cookie(&db, "polykov_session=").await);
    assert_unauthorized(&me_bearer(&db, "").await);
}

#[tokio::test]
async fn me_with_wrong_token_returns_401() {
    let (_dir, db) = DataDir::new("me-wrong");
    let token = session_token(&db).await;
    assert_unauthorized(&me_bearer(&db, &"0".repeat(64)).await);
    assert_unauthorized(&me_cookie(&db, "polykov_session=nope").await);
    let basic = format!("Basic {token}");
    let res = with_header(&db, Method::GET, "/api/me", header::AUTHORIZATION, &basic).await;
    assert_unauthorized(&res);
}

#[tokio::test]
async fn bad_bearer_header_does_not_fall_back_to_cookie() {
    let (_dir, db) = DataDir::new("me-precedence");
    let token = session_token(&db).await;
    let req = Request::get("/api/me")
        .header(header::AUTHORIZATION, "Bearer wrong")
        .header(header::COOKIE, format!("polykov_session={token}"))
        .body(Body::empty())
        .unwrap();
    assert_unauthorized(&send(&db, req).await);
}

#[tokio::test]
async fn me_with_expired_session_returns_401() {
    let (dir, db) = DataDir::new("me-expired");
    let token = session_token(&db).await;
    assert_eq!(me_bearer(&db, &token).await.status, StatusCode::OK);
    dir.sql()
        .execute("UPDATE sessions SET expires_at = unixepoch() - 1", [])
        .unwrap();
    assert_unauthorized(&me_bearer(&db, &token).await);
    assert_unauthorized(&me_cookie(&db, &format!("polykov_session={token}")).await);
}

#[tokio::test]
async fn logout_deletes_only_its_session_and_clears_cookie() {
    let (dir, db) = DataDir::new("logout");
    let token = session_token(&db).await;
    let other = new_login(&db).await;
    let cookie = format!("polykov_session={token}");
    let res = with_header(&db, Method::POST, "/api/logout", header::COOKIE, &cookie).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    let cleared = res.cookie.unwrap();
    assert!(cleared.starts_with("polykov_session=;"), "{cleared}");
    for attr in ["HttpOnly", "SameSite=Strict", "Path=/", "Max-Age=0"] {
        assert!(cleared.split("; ").any(|a| a == attr), "{cleared}");
    }
    assert_unauthorized(&me_bearer(&db, &token).await);
    let left: i64 = dir
        .sql()
        .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 1);
    assert_eq!(me_bearer(&db, &other).await.status, StatusCode::OK);
}

#[tokio::test]
async fn logout_requires_auth() {
    let (dir, db) = DataDir::new("logout-401");
    session_token(&db).await;
    let res = post(&db, "/api/logout", json!({})).await;
    assert_unauthorized(&res);
    assert!(res.cookie.is_none());
    let left: i64 = dir
        .sql()
        .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 1);
}

#[tokio::test]
async fn health_setup_and_login_stay_public() {
    let (_dir, db) = DataDir::new("public");
    assert_eq!(get(&db, "/health").await.status, StatusCode::OK);
    assert_eq!(get(&db, "/api/setup").await.status, StatusCode::OK);
    assert_eq!(setup(&db, PASSWORD).await.status, StatusCode::CREATED);
    assert_eq!(login(&db, PASSWORD).await.status, StatusCode::OK);
}
