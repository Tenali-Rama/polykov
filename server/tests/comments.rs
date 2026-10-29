//! Comments: `POST`/`GET /api/objects/{id}/comments` and `PATCH`/`DELETE
//! /api/comments/{cid}`.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

async fn add(db: &Db, auth: &str, id: i64, body: &str) -> Reply {
    let path = format!("/api/objects/{id}/comments");
    call(
        db,
        Method::POST,
        &path,
        Some(auth),
        Some(json!({ "body": body })),
    )
    .await
}

async fn list(db: &Db, auth: &str, id: i64) -> Reply {
    let path = format!("/api/objects/{id}/comments");
    call(db, Method::GET, &path, Some(auth), None).await
}

async fn edit(db: &Db, auth: &str, cid: i64, body: &str) -> Reply {
    let path = format!("/api/comments/{cid}");
    call(
        db,
        Method::PATCH,
        &path,
        Some(auth),
        Some(json!({ "body": body })),
    )
    .await
}

async fn delete(db: &Db, auth: &str, cid: i64) -> Reply {
    let path = format!("/api/comments/{cid}");
    call(db, Method::DELETE, &path, Some(auth), None).await
}

async fn trash(db: &Db, auth: &str, id: i64) {
    let path = format!("/api/objects/{id}");
    let res = call(db, Method::DELETE, &path, Some(auth), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
}

fn assert_error(res: &Reply, status: StatusCode, error: &str) {
    assert_eq!(res.status, status);
    assert_eq!(res.json(), json!({ "error": error }));
}

fn is_rfc3339_utc(s: &str) -> bool {
    let b = s.as_bytes();
    s.len() == 20 && b[4] == b'-' && b[10] == b'T' && b[19] == b'Z'
}

fn bodies(res: &Reply) -> Vec<Value> {
    res.json()["comments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["body"].clone())
        .collect()
}

#[tokio::test]
async fn create_returns_201_and_the_trimmed_comment() {
    let (_dir, db) = DataDir::new("comments-create");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let res = add(&db, &auth, id, "  looks good \n").await;
    assert_eq!(res.status, StatusCode::CREATED);
    let c = res.json();
    assert!(c["id"].as_i64().unwrap() > 0);
    assert_eq!(c["object_id"], id);
    assert_eq!(c["body"], "looks good");
    assert!(is_rfc3339_utc(c["created"].as_str().unwrap()));
    assert_eq!(c["created"], c["updated"]);
    assert_eq!(c.as_object().unwrap().len(), 5);
}

#[tokio::test]
async fn body_must_be_1_to_10000_chars() {
    let (_dir, db) = DataDir::new("comments-body");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let cid = add(&db, &auth, id, "ok").await.json()["id"]
        .as_i64()
        .unwrap();
    let max = "é".repeat(10_000);
    assert_eq!(add(&db, &auth, id, &max).await.status, StatusCode::CREATED);
    for bad in ["", "  \n ", &format!("{max}x")] {
        for res in [
            add(&db, &auth, id, bad).await,
            edit(&db, &auth, cid, bad).await,
        ] {
            assert_error(
                &res,
                StatusCode::BAD_REQUEST,
                "body must be 1 to 10000 characters",
            );
        }
    }
    assert_eq!(bodies(&list(&db, &auth, id).await)[0], "ok");
}

#[tokio::test]
async fn list_is_oldest_first() {
    let (dir, db) = DataDir::new("comments-list");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let other = create_object(&db, &auth, "task", "other").await;
    for body in ["first", "second", "third"] {
        add(&db, &auth, id, body).await;
    }
    add(&db, &auth, other, "elsewhere").await;
    // Make "third" the oldest by time, so order follows `created`, not id.
    dir.sql()
        .execute(
            "UPDATE comments SET created = created - 60 WHERE body = 'third'",
            [],
        )
        .unwrap();
    let res = list(&db, &auth, id).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(bodies(&res), vec!["third", "first", "second"]);
    assert_eq!(
        list(&db, &auth, create_object(&db, &auth, "note", "n").await)
            .await
            .json(),
        json!({ "comments": [] })
    );
}

#[tokio::test]
async fn patch_changes_body_and_bumps_updated() {
    let (dir, db) = DataDir::new("comments-patch");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let c = add(&db, &auth, id, "draft").await.json();
    let cid = c["id"].as_i64().unwrap();
    dir.sql()
        .execute(
            "UPDATE comments SET created = created - 60, updated = updated - 60",
            [],
        )
        .unwrap();
    let res = edit(&db, &auth, cid, " final ").await;
    assert_eq!(res.status, StatusCode::OK);
    let edited = res.json();
    assert_eq!(edited["id"], cid);
    assert_eq!(edited["body"], "final");
    assert!(edited["updated"].as_str() > edited["created"].as_str());
    assert_eq!(list(&db, &auth, id).await.json()["comments"][0], edited);
}

#[tokio::test]
async fn delete_removes_once() {
    let (_dir, db) = DataDir::new("comments-delete");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let cid = add(&db, &auth, id, "bye").await.json()["id"]
        .as_i64()
        .unwrap();
    let res = delete(&db, &auth, cid).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    assert_eq!(list(&db, &auth, id).await.json(), json!({ "comments": [] }));
    assert_error(
        &delete(&db, &auth, cid).await,
        StatusCode::NOT_FOUND,
        "comment not found",
    );
    assert_error(
        &edit(&db, &auth, cid, "x").await,
        StatusCode::NOT_FOUND,
        "comment not found",
    );
}

#[tokio::test]
async fn unknown_or_trashed_objects_are_404() {
    let (dir, db) = DataDir::new("comments-trash");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let cid = add(&db, &auth, id, "kept").await.json()["id"]
        .as_i64()
        .unwrap();
    trash(&db, &auth, id).await;
    for obj in [id, 999] {
        assert_error(
            &add(&db, &auth, obj, "x").await,
            StatusCode::NOT_FOUND,
            "object not found",
        );
        assert_error(
            &list(&db, &auth, obj).await,
            StatusCode::NOT_FOUND,
            "object not found",
        );
    }
    for c in [cid, 999] {
        assert_error(
            &edit(&db, &auth, c, "x").await,
            StatusCode::NOT_FOUND,
            "comment not found",
        );
        assert_error(
            &delete(&db, &auth, c).await,
            StatusCode::NOT_FOUND,
            "comment not found",
        );
    }

    // Restore (task 017 adds the endpoint) only clears `deleted_at`; the comment is intact.
    dir.sql()
        .execute("UPDATE objects SET deleted_at = NULL", [])
        .unwrap();
    assert_eq!(bodies(&list(&db, &auth, id).await), vec!["kept"]);
}

#[tokio::test]
async fn every_route_requires_auth() {
    let (_dir, db) = DataDir::new("comments-auth");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let cid = add(&db, &auth, id, "secret").await.json()["id"]
        .as_i64()
        .unwrap();
    let on_object = format!("/api/objects/{id}/comments");
    let one = format!("/api/comments/{cid}");
    let body = Some(json!({ "body": "x" }));
    let requests = [
        (Method::POST, on_object.as_str(), body.clone()),
        (Method::GET, on_object.as_str(), None),
        (Method::PATCH, one.as_str(), body),
        (Method::DELETE, one.as_str(), None),
    ];
    for (method, path, body) in requests {
        for bad in [None, Some("0".repeat(64))] {
            let res = call(&db, method.clone(), path, bad.as_deref(), body.clone()).await;
            assert_error(&res, StatusCode::UNAUTHORIZED, "authentication required");
        }
    }
    assert_eq!(bodies(&list(&db, &auth, id).await), vec!["secret"]);
}
