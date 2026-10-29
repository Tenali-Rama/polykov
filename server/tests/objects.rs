//! Generic objects: `POST /api/objects`, `GET`/`PATCH`/`DELETE /api/objects/{id}`, the M1
//! schema and `objects::store::summary`.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, token};
use polykov_server::{db::Db, objects::store};
use serde_json::{Value, json};

async fn post(db: &Db, auth: &str, body: Value) -> Reply {
    call(db, Method::POST, "/api/objects", Some(auth), Some(body)).await
}

async fn get(db: &Db, auth: &str, id: i64) -> Reply {
    let path = format!("/api/objects/{id}");
    call(db, Method::GET, &path, Some(auth), None).await
}

async fn patch(db: &Db, auth: &str, id: i64, body: Value) -> Reply {
    let path = format!("/api/objects/{id}");
    call(db, Method::PATCH, &path, Some(auth), Some(body)).await
}

async fn delete(db: &Db, auth: &str, id: i64) -> Reply {
    let path = format!("/api/objects/{id}");
    call(db, Method::DELETE, &path, Some(auth), None).await
}

fn assert_error(res: &Reply, status: StatusCode, error: &str) {
    assert_eq!(res.status, status);
    assert_eq!(res.json(), json!({ "error": error }));
}

fn is_rfc3339_utc(s: &str) -> bool {
    let b = s.as_bytes();
    s.len() == 20 && b[4] == b'-' && b[10] == b'T' && b[19] == b'Z'
}

#[tokio::test]
async fn create_returns_201_and_the_object() {
    let (_dir, db) = DataDir::new("obj-create");
    let auth = token(&db).await;
    let res = post(
        &db,
        &auth,
        json!({ "type": "note", "title": "  Groceries  ", "properties": { "aisle": 3, "list": ["milk", null] }, "pinned": true }),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let obj = res.json();
    assert!(obj["id"].as_i64().unwrap() > 0);
    assert_eq!(obj["type"], "note");
    assert_eq!(obj["title"], "Groceries");
    assert_eq!(obj["pinned"], true);
    assert_eq!(obj["archived"], false);
    assert_eq!(obj["tags"], json!([]));
    assert_eq!(
        obj["properties"],
        json!({ "aisle": 3, "list": ["milk", null] })
    );
    assert!(is_rfc3339_utc(obj["created"].as_str().unwrap()));
    assert_eq!(obj["created"], obj["updated"]);
    assert_eq!(
        get(&db, &auth, obj["id"].as_i64().unwrap()).await.json(),
        obj
    );
}

#[tokio::test]
async fn create_defaults_properties_and_flags() {
    let (_dir, db) = DataDir::new("obj-defaults");
    let auth = token(&db).await;
    let res = post(&db, &auth, json!({ "type": "task", "title": "x" })).await;
    let obj = res.json();
    assert_eq!(obj["properties"], json!({}));
    assert_eq!(obj["pinned"], false);
    assert_eq!(obj["archived"], false);
}

#[tokio::test]
async fn create_rejects_invalid_input() {
    let (_dir, db) = DataDir::new("obj-invalid");
    let auth = token(&db).await;
    let ok = |extra: Value| {
        let mut body = json!({ "type": "note", "title": "t" });
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        body
    };
    let types = "type must be one of: task, project, area, goal, milestone, habit, note, \
                 notebook, journal_entry, daily_note, review, event, time_entry, view, forecast_record";
    let title = "title must be 1 to 1000 characters";
    let props = "properties must be an object";
    let cases = [
        (ok(json!({ "type": "widget" })), types.to_owned()),
        (ok(json!({ "type": "Note" })), types.to_owned()),
        (ok(json!({ "title": "" })), title.to_owned()),
        (ok(json!({ "title": " \t\n " })), title.to_owned()),
        (ok(json!({ "title": "é".repeat(1001) })), title.to_owned()),
        (ok(json!({ "properties": [] })), props.to_owned()),
        (ok(json!({ "properties": "a" })), props.to_owned()),
        (ok(json!({ "properties": null })), props.to_owned()),
        (
            ok(json!({ "properties": { "": 1 } })),
            "property names must be 1 to 64 characters".to_owned(),
        ),
        (
            ok(json!({ "properties": { "k".repeat(65): 1 } })),
            "property names must be 1 to 64 characters".to_owned(),
        ),
        (
            ok(json!({ "properties": { "a": { "b": 1 } } })),
            r#"property "a" must be a string, number, bool, null or an array of those"#.to_owned(),
        ),
        (
            ok(json!({ "properties": { "a": [[1]] } })),
            r#"property "a" must be a string, number, bool, null or an array of those"#.to_owned(),
        ),
        (
            ok(json!({ "properties": { "a": [{}] } })),
            r#"property "a" must be a string, number, bool, null or an array of those"#.to_owned(),
        ),
    ];
    for (body, error) in cases {
        let res = post(&db, &auth, body.clone()).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(res.json(), json!({ "error": error }), "{body}");
    }
    let many: serde_json::Map<String, Value> =
        (0..101).map(|i| (format!("k{i}"), json!(i))).collect();
    let res = post(&db, &auth, ok(json!({ "properties": many }))).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "properties must have at most 100 keys",
    );
    // Nothing was stored.
    assert_eq!(get(&db, &auth, 1).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_accepts_limits() {
    let (_dir, db) = DataDir::new("obj-limits");
    let auth = token(&db).await;
    let props: serde_json::Map<String, Value> = (0..100)
        .map(|i| (format!("{i:0>64}"), json!(true)))
        .collect();
    let body = json!({ "type": "forecast_record", "title": "é".repeat(1000), "properties": props });
    assert_eq!(post(&db, &auth, body).await.status, StatusCode::CREATED);
}

#[tokio::test]
async fn create_rejects_unknown_fields_and_missing_type() {
    let (_dir, db) = DataDir::new("obj-unknown");
    let auth = token(&db).await;
    let res = post(
        &db,
        &auth,
        json!({ "type": "note", "title": "t", "colour": 1 }),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = post(&db, &auth, json!({ "title": "t" })).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn get_unknown_or_bad_id() {
    let (_dir, db) = DataDir::new("obj-get-404");
    let auth = token(&db).await;
    assert_error(
        &get(&db, &auth, 42).await,
        StatusCode::NOT_FOUND,
        "object not found",
    );
    let res = call(&db, Method::GET, "/api/objects/abc", Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn patch_changes_given_fields_and_bumps_updated() {
    let (dir, db) = DataDir::new("obj-patch");
    let auth = token(&db).await;
    let res = post(
        &db,
        &auth,
        json!({ "type": "task", "title": "a", "properties": { "x": 1 } }),
    )
    .await;
    let id = res.json()["id"].as_i64().unwrap();
    let backdate = || {
        dir.sql()
            .execute(
                "UPDATE objects SET created = 1000, updated = 1000 WHERE id = ?1",
                [id],
            )
            .unwrap()
    };
    backdate();

    let obj = patch(&db, &auth, id, json!({})).await;
    assert_eq!(obj.status, StatusCode::OK);
    assert_eq!(obj.json()["updated"], "1970-01-01T00:16:40Z");
    assert_eq!(obj.json()["title"], "a");

    let res = patch(
        &db,
        &auth,
        id,
        json!({ "title": " b ", "properties": { "y": "z" }, "archived": true }),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let obj = res.json();
    assert_eq!(obj["title"], "b");
    assert_eq!(obj["properties"], json!({ "y": "z" }));
    assert_eq!(obj["archived"], true);
    assert_eq!(obj["pinned"], false);
    assert_eq!(obj["type"], "task");
    assert_eq!(obj["created"], "1970-01-01T00:16:40Z");
    assert_ne!(obj["updated"], "1970-01-01T00:16:40Z");
    assert_eq!(get(&db, &auth, id).await.json(), obj);

    backdate();
    let obj = patch(&db, &auth, id, json!({ "pinned": true }))
        .await
        .json();
    assert_eq!(obj["pinned"], true);
    assert_eq!(obj["title"], "b");
    assert_ne!(obj["updated"], "1970-01-01T00:16:40Z");
}

#[tokio::test]
async fn patch_validates_like_create() {
    let (_dir, db) = DataDir::new("obj-patch-invalid");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "keep").await;
    let res = patch(&db, &auth, id, json!({ "title": "  " })).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "title must be 1 to 1000 characters",
    );
    let res = patch(&db, &auth, id, json!({ "properties": { "a": { "b": 1 } } })).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    let res = patch(&db, &auth, id, json!({ "properties": null })).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "properties must be an object",
    );
    // `type` cannot be changed, and `null` is not "absent".
    assert_eq!(
        patch(&db, &auth, id, json!({ "type": "task" }))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        patch(&db, &auth, id, json!({ "title": null })).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(get(&db, &auth, id).await.json()["title"], "keep");
}

#[tokio::test]
async fn delete_moves_to_trash() {
    let (dir, db) = DataDir::new("obj-delete");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "bye").await;
    let res = delete(&db, &auth, id).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    let deleted_at: Option<i64> = dir
        .sql()
        .query_row("SELECT deleted_at FROM objects WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(deleted_at.is_some());

    assert_error(
        &delete(&db, &auth, id).await,
        StatusCode::NOT_FOUND,
        "object not found",
    );
    assert_error(
        &get(&db, &auth, id).await,
        StatusCode::NOT_FOUND,
        "object not found",
    );
    let res = patch(&db, &auth, id, json!({ "title": "back" })).await;
    assert_error(&res, StatusCode::NOT_FOUND, "object not found");
    assert_error(
        &patch(&db, &auth, id, json!({})).await,
        StatusCode::NOT_FOUND,
        "object not found",
    );
    assert_error(
        &delete(&db, &auth, 999).await,
        StatusCode::NOT_FOUND,
        "object not found",
    );
}

#[tokio::test]
async fn every_route_requires_auth() {
    let (_dir, db) = DataDir::new("obj-auth");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "secret").await;
    let path = format!("/api/objects/{id}");
    let body = json!({ "type": "note", "title": "x" });
    let requests = [
        (Method::POST, "/api/objects", Some(body)),
        (Method::GET, path.as_str(), None),
        (Method::PATCH, path.as_str(), Some(json!({ "title": "y" }))),
        (Method::DELETE, path.as_str(), None),
    ];
    for (method, path, body) in requests {
        for bad in [None, Some("0".repeat(64))] {
            let res = call(&db, method.clone(), path, bad.as_deref(), body.clone()).await;
            assert_error(&res, StatusCode::UNAUTHORIZED, "authentication required");
        }
    }
    assert_eq!(get(&db, &auth, id).await.json()["title"], "secret");
}

#[tokio::test]
async fn ids_are_never_reused() {
    let (dir, db) = DataDir::new("obj-ids");
    let auth = token(&db).await;
    let first = create_object(&db, &auth, "note", "a").await;
    dir.sql()
        .execute("DELETE FROM objects WHERE id = ?1", [first])
        .unwrap();
    let second = create_object(&db, &auth, "note", "b").await;
    assert!(second > first);
}

#[tokio::test]
async fn summary_sees_live_objects_only() {
    let (_dir, db) = DataDir::new("obj-summary");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "Launch").await;
    let expected = store::Summary {
        id,
        r#type: "project".into(),
        title: "Launch".into(),
    };
    assert_eq!(store::summary(&db, id).unwrap(), Some(expected));
    assert_eq!(store::summary(&db, id + 1).unwrap(), None);
    delete(&db, &auth, id).await;
    assert_eq!(store::summary(&db, id).unwrap(), None);
}

#[tokio::test]
async fn side_tables_cascade_and_links_reject_self() {
    let (dir, db) = DataDir::new("obj-schema");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "note", "a").await;
    let b = create_object(&db, &auth, "task", "b").await;
    let sql = dir.sql();
    sql.execute_batch("PRAGMA foreign_keys = ON").unwrap();
    let self_link = sql.execute("INSERT INTO links (src_id, dst_id) VALUES (?1, ?1)", [a]);
    assert!(self_link.is_err());
    let orphan = sql.execute(
        "INSERT INTO comments (object_id, body) VALUES (999, 'x')",
        [],
    );
    assert!(orphan.is_err());
    sql.execute_batch(&format!(
        "INSERT INTO object_tags VALUES ({a}, 'work');
         INSERT INTO links (src_id, dst_id) VALUES ({a}, {b}), ({b}, {a});
         INSERT INTO comments (object_id, body) VALUES ({a}, 'hi');
         INSERT INTO attachments (object_id, name, mime, size) VALUES ({a}, 'f.txt', 'text/plain', 3);
         DELETE FROM objects WHERE id = {a};"
    ))
    .unwrap();
    for table in ["object_tags", "links", "comments", "attachments"] {
        let n: i64 = sql
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "{table}");
    }
    assert_eq!(get(&db, &auth, b).await.status, StatusCode::OK);
}
