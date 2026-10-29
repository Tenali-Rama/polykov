//! Links and backlinks: `PUT`/`DELETE /api/objects/{id}/links/{to}` and `GET
//! /api/objects/{id}/links`.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

async fn link(db: &Db, auth: &str, id: i64, to: i64) -> Reply {
    let path = format!("/api/objects/{id}/links/{to}");
    call(db, Method::PUT, &path, Some(auth), None).await
}

async fn unlink(db: &Db, auth: &str, id: i64, to: i64) -> Reply {
    let path = format!("/api/objects/{id}/links/{to}");
    call(db, Method::DELETE, &path, Some(auth), None).await
}

async fn links(db: &Db, auth: &str, id: i64) -> Reply {
    let path = format!("/api/objects/{id}/links");
    call(db, Method::GET, &path, Some(auth), None).await
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

fn ids(side: &Value) -> Vec<i64> {
    side.as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn link_is_idempotent_and_listed_with_backlinks() {
    let (dir, db) = DataDir::new("links-basic");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "project", "Launch").await;
    let b = create_object(&db, &auth, "task", "Write copy").await;

    for _ in 0..2 {
        let res = link(&db, &auth, a, b).await;
        assert_eq!(res.status, StatusCode::NO_CONTENT);
        assert!(res.body.is_empty());
    }
    let rows: i64 = dir
        .sql()
        .query_row("SELECT count(*) FROM links", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1);

    let res = links(&db, &auth, a).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.json(),
        json!({ "links": [{ "id": b, "type": "task", "title": "Write copy" }], "backlinks": [] })
    );
    assert_eq!(
        links(&db, &auth, b).await.json(),
        json!({ "links": [], "backlinks": [{ "id": a, "type": "project", "title": "Launch" }] })
    );
}

#[tokio::test]
async fn both_sides_are_ordered_by_title() {
    let (_dir, db) = DataDir::new("links-order");
    let auth = token(&db).await;
    let hub = create_object(&db, &auth, "area", "Hub").await;
    let c = create_object(&db, &auth, "note", "Cherry").await;
    let a = create_object(&db, &auth, "note", "Apple").await;
    let b = create_object(&db, &auth, "note", "Banana").await;
    for id in [c, a, b] {
        link(&db, &auth, hub, id).await;
        link(&db, &auth, id, hub).await;
    }
    let body = links(&db, &auth, hub).await.json();
    assert_eq!(ids(&body["links"]), vec![a, b, c]);
    assert_eq!(ids(&body["backlinks"]), vec![a, b, c]);
}

#[tokio::test]
async fn self_links_are_rejected() {
    let (_dir, db) = DataDir::new("links-self");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "note", "a").await;
    assert_error(
        &link(&db, &auth, a, a).await,
        StatusCode::BAD_REQUEST,
        "cannot link an object to itself",
    );
}

#[tokio::test]
async fn unlink_removes_once() {
    let (_dir, db) = DataDir::new("links-unlink");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "note", "a").await;
    let b = create_object(&db, &auth, "note", "b").await;
    link(&db, &auth, a, b).await;
    assert_error(
        &unlink(&db, &auth, b, a).await,
        StatusCode::NOT_FOUND,
        "link not found",
    );
    let res = unlink(&db, &auth, a, b).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    assert_eq!(
        links(&db, &auth, a).await.json(),
        json!({ "links": [], "backlinks": [] })
    );
    assert_error(
        &unlink(&db, &auth, a, b).await,
        StatusCode::NOT_FOUND,
        "link not found",
    );
}

#[tokio::test]
async fn unknown_or_trashed_objects_are_404() {
    let (_dir, db) = DataDir::new("links-404");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "note", "a").await;
    let gone = create_object(&db, &auth, "note", "gone").await;
    link(&db, &auth, a, gone).await;
    trash(&db, &auth, gone).await;
    for (x, y) in [(a, 999), (999, a), (a, gone), (gone, a)] {
        for res in [link(&db, &auth, x, y).await, unlink(&db, &auth, x, y).await] {
            assert_error(&res, StatusCode::NOT_FOUND, "object not found");
        }
    }
    for id in [999, gone] {
        assert_error(
            &links(&db, &auth, id).await,
            StatusCode::NOT_FOUND,
            "object not found",
        );
    }
}

#[tokio::test]
async fn trashed_objects_are_hidden_until_restored() {
    let (dir, db) = DataDir::new("links-trash");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "note", "a").await;
    let b = create_object(&db, &auth, "note", "b").await;
    let c = create_object(&db, &auth, "note", "c").await;
    link(&db, &auth, a, b).await;
    link(&db, &auth, c, a).await;
    trash(&db, &auth, b).await;
    trash(&db, &auth, c).await;
    assert_eq!(
        links(&db, &auth, a).await.json(),
        json!({ "links": [], "backlinks": [] })
    );

    // Restore (task 017 adds the endpoint) only clears `deleted_at`.
    dir.sql()
        .execute("UPDATE objects SET deleted_at = NULL", [])
        .unwrap();
    let body = links(&db, &auth, a).await.json();
    assert_eq!(ids(&body["links"]), vec![b]);
    assert_eq!(ids(&body["backlinks"]), vec![c]);
}

#[tokio::test]
async fn every_route_requires_auth() {
    let (dir, db) = DataDir::new("links-auth");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "note", "a").await;
    let b = create_object(&db, &auth, "note", "b").await;
    link(&db, &auth, a, b).await;
    let pair = format!("/api/objects/{a}/links/{b}");
    let list = format!("/api/objects/{a}/links");
    let requests = [
        (Method::PUT, pair.as_str()),
        (Method::DELETE, pair.as_str()),
        (Method::GET, list.as_str()),
    ];
    for (method, path) in requests {
        for bad in [None, Some("0".repeat(64))] {
            let res = call(&db, method.clone(), path, bad.as_deref(), None).await;
            assert_error(&res, StatusCode::UNAUTHORIZED, "authentication required");
        }
    }
    let rows: i64 = dir
        .sql()
        .query_row("SELECT count(*) FROM links", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1);
}
