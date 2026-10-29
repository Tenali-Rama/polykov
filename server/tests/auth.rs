mod common;

use axum::http::StatusCode;
use common::{DataDir, PASSWORD, get, login, setup};
use polykov_server::auth::session;
use serde_json::json;

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
