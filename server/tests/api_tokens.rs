//! API tokens: `POST`/`GET /api/tokens`, `DELETE /api/tokens/{id}`, and using a token.

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{DataDir, PASSWORD, Reply, login, send, setup};
use polykov_server::db::Db;
use serde_json::{Value, json};

async fn session_token(db: &Db) -> String {
    setup(db, PASSWORD).await;
    login(db, PASSWORD).await.json()["token"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn call(
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

async fn create(db: &Db, auth: &str, name: &str) -> Reply {
    call(
        db,
        Method::POST,
        "/api/tokens",
        Some(auth),
        Some(json!({ "name": name })),
    )
    .await
}

async fn list(db: &Db, auth: &str) -> Value {
    let res = call(db, Method::GET, "/api/tokens", Some(auth), None).await;
    assert_eq!(res.status, StatusCode::OK);
    res.json()
}

#[tokio::test]
async fn create_returns_token_once_and_stores_only_its_hash() {
    let (dir, db) = DataDir::new("tokens-create");
    let auth = session_token(&db).await;
    let res = create(&db, &auth, "backup script").await;
    assert_eq!(res.status, StatusCode::CREATED);
    let body = res.json();
    assert_eq!(body["name"], json!("backup script"));
    assert!(body["id"].is_i64());
    let token = body["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));

    let row: String = dir
        .sql()
        .query_row(
            "SELECT id || '|' || name || '|' || token_hash || '|' || created_at FROM api_tokens",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let hash = polykov_server::auth::session::hash_token(token);
    assert!(row.contains(&hash));
    assert!(!row.contains(token));
}

#[tokio::test]
async fn token_authenticates_and_updates_last_used() {
    let (_dir, db) = DataDir::new("tokens-use");
    let auth = session_token(&db).await;
    let token = create(&db, &auth, "ci").await.json()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(list(&db, &auth).await[0]["last_used"], Value::Null);

    let me = call(&db, Method::GET, "/api/me", Some(&token), None).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json(), json!({ "owner": true, "session_expires": null }));

    let listed = list(&db, &token).await;
    let last_used = listed[0]["last_used"].as_str().unwrap();
    assert_eq!(last_used.len(), 20);
    assert!(last_used.ends_with('Z'));
}

#[tokio::test]
async fn list_shows_metadata_but_never_the_token_or_hash() {
    let (dir, db) = DataDir::new("tokens-list");
    let auth = session_token(&db).await;
    let first = create(&db, &auth, "one").await.json();
    create(&db, &auth, "two").await;
    let hash: String = dir
        .sql()
        .query_row(
            "SELECT token_hash FROM api_tokens WHERE name = 'one'",
            [],
            |r| r.get(0),
        )
        .unwrap();

    let listed = list(&db, &auth).await;
    let items = listed.as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], first["id"]);
    assert_eq!(items[0]["name"], json!("one"));
    assert_eq!(items[1]["name"], json!("two"));
    let mut keys: Vec<_> = items[0].as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["created", "id", "last_used", "name"]);
    assert_eq!(items[0]["created"].as_str().unwrap().len(), 20);
    let text = listed.to_string();
    assert!(!text.contains(first["token"].as_str().unwrap()) && !text.contains(&hash));
}

#[tokio::test]
async fn revoked_token_gets_401_and_unknown_id_404() {
    let (_dir, db) = DataDir::new("tokens-revoke");
    let auth = session_token(&db).await;
    let body = create(&db, &auth, "phone").await.json();
    let token = body["token"].as_str().unwrap();
    let path = format!("/api/tokens/{}", body["id"]);

    let res = call(&db, Method::DELETE, &path, Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    let me = call(&db, Method::GET, "/api/me", Some(token), None).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
    assert_eq!(list(&db, &auth).await, json!([]));

    let again = call(&db, Method::DELETE, &path, Some(&auth), None).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    assert_eq!(again.json(), json!({ "error": "token not found" }));
}

#[tokio::test]
async fn token_can_revoke_itself() {
    let (_dir, db) = DataDir::new("tokens-self");
    let auth = session_token(&db).await;
    let body = create(&db, &auth, "self").await.json();
    let token = body["token"].as_str().unwrap();
    let path = format!("/api/tokens/{}", body["id"]);
    let res = call(&db, Method::DELETE, &path, Some(token), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let res = call(&db, Method::GET, "/api/tokens", Some(token), None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn all_endpoints_require_auth() {
    let (_dir, db) = DataDir::new("tokens-unauth");
    let auth = session_token(&db).await;
    let id = create(&db, &auth, "keep").await.json()["id"].clone();
    let path = format!("/api/tokens/{id}");
    let name = Some(json!({ "name": "x" }));
    for token in [None, Some("0".repeat(64).as_str())] {
        for (method, path, body) in [
            (Method::POST, "/api/tokens", name.clone()),
            (Method::GET, "/api/tokens", None),
            (Method::DELETE, path.as_str(), None),
        ] {
            let res = call(&db, method.clone(), path, token, body).await;
            assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{method} {path}");
            assert_eq!(res.json(), json!({ "error": "authentication required" }));
        }
    }
    // Nothing was created or revoked.
    let listed = list(&db, &auth).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn bad_names_get_400() {
    let (dir, db) = DataDir::new("tokens-names");
    let auth = session_token(&db).await;
    for name in [String::new(), "   ".into(), "a".repeat(65), "é".repeat(65)] {
        let res = create(&db, &auth, &name).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{name:?}");
        assert_eq!(
            res.json(),
            json!({ "error": "name must be 1 to 64 characters" })
        );
    }
    // 64 characters is fine even when it is 128 bytes; surrounding spaces are trimmed.
    let res = create(&db, &auth, &"é".repeat(64)).await;
    assert_eq!(res.status, StatusCode::CREATED);
    let res = create(&db, &auth, "  padded  ").await;
    assert_eq!(res.json()["name"], json!("padded"));
    let count: i64 = dir
        .sql()
        .query_row("SELECT count(*) FROM api_tokens", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn logout_with_api_token_does_not_revoke_it() {
    let (_dir, db) = DataDir::new("tokens-logout");
    let auth = session_token(&db).await;
    let token = create(&db, &auth, "cli").await.json()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let res = call(&db, Method::POST, "/api/logout", Some(&token), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let me = call(&db, Method::GET, "/api/me", Some(&token), None).await;
    assert_eq!(me.status, StatusCode::OK);
}
