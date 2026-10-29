//! The trash: `GET`/`DELETE /api/trash`, `POST /api/trash/{id}/restore`,
//! `DELETE /api/trash/{id}` and `trash::purge_expired`.

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{DataDir, Reply, call, create_object, send, token};
use polykov_server::{db::Db, trash};
use serde_json::{Value, json};

const DAY: i64 = 24 * 60 * 60;

fn assert_error(res: &Reply, status: StatusCode, error: &str) {
    assert_eq!(res.status, status);
    assert_eq!(res.json(), json!({ "error": error }));
}

async fn delete(db: &Db, auth: &str, path: &str) -> Reply {
    call(db, Method::DELETE, path, Some(auth), None).await
}

async fn list(db: &Db, auth: &str) -> Value {
    let res = call(db, Method::GET, "/api/trash", Some(auth), None).await;
    assert_eq!(res.status, StatusCode::OK);
    res.json()
}

/// Sets object `id`'s tags.
async fn tag(db: &Db, auth: &str, id: i64, tags: Value) {
    let path = format!("/api/objects/{id}");
    let res = call(
        db,
        Method::PATCH,
        &path,
        Some(auth),
        Some(json!({ "tags": tags })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
}

/// Uploads a one-byte file to object `id` and returns the attachment id.
async fn attach(db: &Db, auth: &str, id: i64) -> i64 {
    let req = Request::post(format!("/api/objects/{id}/attachments?name=a.txt"))
        .header(header::AUTHORIZATION, format!("Bearer {auth}"))
        .body(Body::from("a"))
        .unwrap();
    let res = send(db, req).await;
    assert_eq!(res.status, StatusCode::CREATED);
    res.json()["id"].as_i64().unwrap()
}

/// Sets object `id`'s `deleted_at` to `secs` (Unix time) directly.
fn deleted_at(dir: &DataDir, id: i64, secs: i64) {
    let sql = "UPDATE objects SET deleted_at = ?2 WHERE id = ?1";
    dir.sql().execute(sql, [id, secs]).unwrap();
}

fn count(dir: &DataDir, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    dir.sql().query_row(&sql, [], |r| r.get(0)).unwrap()
}

fn now() -> i64 {
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since.unwrap().as_secs() as i64
}

#[tokio::test]
async fn every_route_requires_auth() {
    let (_dir, db) = DataDir::new("trash-auth");
    for (method, path) in [
        (Method::GET, "/api/trash"),
        (Method::DELETE, "/api/trash"),
        (Method::POST, "/api/trash/1/restore"),
        (Method::DELETE, "/api/trash/1"),
    ] {
        let res = call(&db, method, path, None, None).await;
        assert_error(&res, StatusCode::UNAUTHORIZED, "authentication required");
    }
}

#[tokio::test]
async fn lists_trashed_objects_newest_deleted_first() {
    let (dir, db) = DataDir::new("trash-list");
    let auth = token(&db).await;
    let old = create_object(&db, &auth, "note", "old").await;
    let new = create_object(&db, &auth, "task", "new").await;
    create_object(&db, &auth, "note", "live").await;
    assert_eq!(list(&db, &auth).await, json!({ "objects": [] }));
    delete(&db, &auth, &format!("/api/objects/{new}")).await;
    delete(&db, &auth, &format!("/api/objects/{old}")).await;
    // 2026-10-01T00:00:00Z and a day later.
    deleted_at(&dir, old, 1_790_812_800);
    deleted_at(&dir, new, 1_790_812_800 + DAY);

    let objects = list(&db, &auth).await["objects"].clone();
    assert_eq!(objects.as_array().unwrap().len(), 2, "live object listed");
    assert_eq!(objects[0]["id"], new);
    assert_eq!(objects[0]["type"], "task");
    assert_eq!(objects[0]["title"], "new");
    assert_eq!(objects[0]["deleted"], "2026-10-02T00:00:00Z");
    assert_eq!(objects[0]["purge_at"], "2026-11-01T00:00:00Z");
    assert_eq!(objects[1]["id"], old);
    assert_eq!(objects[1]["deleted"], "2026-10-01T00:00:00Z");
    assert_eq!(objects[1]["purge_at"], "2026-10-31T00:00:00Z");
    for key in [
        "created",
        "updated",
        "archived",
        "pinned",
        "tags",
        "properties",
    ] {
        assert!(objects[0].get(key).is_some(), "missing {key}");
    }
}

#[tokio::test]
async fn restore_brings_back_the_object_with_links_and_comments() {
    let (_dir, db) = DataDir::new("trash-restore");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "back").await;
    let other = create_object(&db, &auth, "note", "other").await;
    tag(&db, &auth, id, json!(["work/a", "home"])).await;
    let link = format!("/api/objects/{id}/links/{other}");
    call(&db, Method::PUT, &link, Some(&auth), None).await;
    let comments = format!("/api/objects/{id}/comments");
    let body = Some(json!({ "body": "keep me" }));
    call(&db, Method::POST, &comments, Some(&auth), body).await;
    delete(&db, &auth, &format!("/api/objects/{id}")).await;
    let trashed = list(&db, &auth).await;
    assert_eq!(trashed["objects"][0]["tags"], json!(["home", "work/a"]));

    let restore = format!("/api/trash/{id}/restore");
    let res = call(&db, Method::POST, &restore, Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["id"], id);
    assert_eq!(res.json()["title"], "back");
    assert_eq!(res.json()["tags"], json!(["home", "work/a"]));
    assert!(res.json().get("deleted").is_none());

    let res = call(
        &db,
        Method::GET,
        &format!("/api/objects/{id}"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let links = format!("/api/objects/{id}/links");
    let res = call(&db, Method::GET, &links, Some(&auth), None).await;
    assert_eq!(res.json()["links"][0]["id"], other);
    let res = call(&db, Method::GET, &comments, Some(&auth), None).await;
    assert_eq!(res.json()["comments"][0]["body"], "keep me");
    assert_eq!(list(&db, &auth).await, json!({ "objects": [] }));

    // Now live, so no longer in the trash; unknown ids neither.
    let res = call(&db, Method::POST, &restore, Some(&auth), None).await;
    assert_error(&res, StatusCode::NOT_FOUND, "not in trash");
    let res = call(
        &db,
        Method::POST,
        "/api/trash/999/restore",
        Some(&auth),
        None,
    )
    .await;
    assert_error(&res, StatusCode::NOT_FOUND, "not in trash");
}

#[tokio::test]
async fn permanent_delete_removes_row_side_tables_and_files() {
    let (dir, db) = DataDir::new("trash-purge");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "gone").await;
    let other = create_object(&db, &auth, "note", "other").await;
    tag(&db, &auth, id, json!(["work"])).await;
    let aid = attach(&db, &auth, id).await;
    let file = db.data_dir().join("attachments").join(aid.to_string());
    assert!(file.exists());
    let link = format!("/api/objects/{other}/links/{id}");
    call(&db, Method::PUT, &link, Some(&auth), None).await;
    let body = Some(json!({ "body": "bye" }));
    let comments = format!("/api/objects/{id}/comments");
    call(&db, Method::POST, &comments, Some(&auth), body).await;

    let path = format!("/api/trash/{id}");
    let res = delete(&db, &auth, &path).await;
    assert_error(&res, StatusCode::NOT_FOUND, "not in trash");
    assert!(file.exists(), "live object's file removed");

    delete(&db, &auth, &format!("/api/objects/{id}")).await;
    let res = delete(&db, &auth, &path).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    assert!(!file.exists(), "attachment file left on disk");
    assert_eq!(count(&dir, "objects"), 1);
    for table in ["object_tags", "attachments", "links", "comments"] {
        assert_eq!(count(&dir, table), 0, "{table} rows left");
    }
    let res = delete(&db, &auth, &path).await;
    assert_error(&res, StatusCode::NOT_FOUND, "not in trash");
}

#[tokio::test]
async fn empty_purges_only_the_trash() {
    let (dir, db) = DataDir::new("trash-empty");
    let auth = token(&db).await;
    let res = delete(&db, &auth, "/api/trash").await;
    assert_eq!(
        (res.status, res.json()),
        (StatusCode::OK, json!({ "purged": 0 }))
    );

    let live = create_object(&db, &auth, "note", "live").await;
    let live_file = attach(&db, &auth, live).await;
    for title in ["a", "b"] {
        let id = create_object(&db, &auth, "note", title).await;
        attach(&db, &auth, id).await;
        delete(&db, &auth, &format!("/api/objects/{id}")).await;
    }
    let res = delete(&db, &auth, "/api/trash").await;
    assert_eq!(
        (res.status, res.json()),
        (StatusCode::OK, json!({ "purged": 2 }))
    );
    assert_eq!(list(&db, &auth).await, json!({ "objects": [] }));
    assert_eq!(count(&dir, "objects"), 1);
    assert_eq!(count(&dir, "attachments"), 1);
    let attachments = db.data_dir().join("attachments");
    assert_eq!(std::fs::read_dir(&attachments).unwrap().count(), 1);
    assert!(attachments.join(live_file.to_string()).exists());
}

#[tokio::test]
async fn purge_expired_removes_objects_trashed_over_30_days_ago() {
    let (dir, db) = DataDir::new("trash-expired");
    let auth = token(&db).await;
    let now = now();
    let expired = create_object(&db, &auth, "note", "31 days").await;
    let kept = create_object(&db, &auth, "note", "29 days").await;
    let live = create_object(&db, &auth, "note", "live").await;
    let file = db.data_dir().join("attachments");
    let expired_file = file.join(attach(&db, &auth, expired).await.to_string());
    let live_file = file.join(attach(&db, &auth, live).await.to_string());
    deleted_at(&dir, expired, now - 31 * DAY);
    deleted_at(&dir, kept, now - 29 * DAY);

    assert_eq!(trash::purge_expired(&db, now).unwrap(), 1);
    assert!(!expired_file.exists());
    assert!(live_file.exists());
    let objects = list(&db, &auth).await["objects"].clone();
    assert_eq!(objects.as_array().unwrap().len(), 1);
    assert_eq!(objects[0]["id"], kept);
    let res = call(
        &db,
        Method::GET,
        &format!("/api/objects/{live}"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    // A live object stays even when it is far older than the retention.
    assert_eq!(trash::purge_expired(&db, now + 365 * DAY).unwrap(), 1);
    assert_eq!(count(&dir, "objects"), 1);
    assert!(live_file.exists());
}
