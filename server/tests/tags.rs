//! Hierarchical tags: normalizing and rejecting tags on `POST`/`PATCH /api/objects`, and the
//! folder counts of `GET /api/tags`.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

async fn post(db: &Db, auth: &str, body: Value) -> Reply {
    call(db, Method::POST, "/api/objects", Some(auth), Some(body)).await
}

async fn patch(db: &Db, auth: &str, id: i64, body: Value) -> Reply {
    let path = format!("/api/objects/{id}");
    call(db, Method::PATCH, &path, Some(auth), Some(body)).await
}

/// Creates a note with `tags` and returns its id.
async fn tagged(db: &Db, auth: &str, tags: Value) -> i64 {
    let res = post(
        db,
        auth,
        json!({ "type": "note", "title": "n", "tags": tags }),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    res.json()["id"].as_i64().unwrap()
}

async fn tag_counts(db: &Db, auth: &str) -> Value {
    let res = call(db, Method::GET, "/api/tags", Some(auth), None).await;
    assert_eq!(res.status, StatusCode::OK);
    res.json()
}

#[tokio::test]
async fn tags_are_normalized_deduplicated_and_sorted() {
    let (_dir, db) = DataDir::new("tags-normalize");
    let auth = token(&db).await;
    let res = post(
        &db,
        &auth,
        json!({ "type": "note", "title": "n", "tags": ["  #Work/ClientA ", "home", "work/clienta", "#HOME"] }),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let obj = res.json();
    assert_eq!(obj["tags"], json!(["home", "work/clienta"]));
    let id = obj["id"].as_i64().unwrap();
    let path = format!("/api/objects/{id}");
    let got = call(&db, Method::GET, &path, Some(&auth), None)
        .await
        .json();
    assert_eq!(got["tags"], json!(["home", "work/clienta"]));
}

#[tokio::test]
async fn invalid_tags_get_400_and_store_nothing() {
    let (dir, db) = DataDir::new("tags-invalid");
    let auth = token(&db).await;
    let long = "a".repeat(101);
    for bad in [
        "",
        "#",
        "a//b",
        "a/",
        "/a",
        "two words",
        "a,b",
        "a/#b",
        long.as_str(),
    ] {
        let res = post(
            &db,
            &auth,
            json!({ "type": "note", "title": "n", "tags": ["ok", bad] }),
        )
        .await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{bad:?}");
        assert_eq!(
            res.json(),
            json!({ "error": format!("invalid tag: {bad}") })
        );
    }
    let n: i64 = dir
        .sql()
        .query_row("SELECT COUNT(*) FROM objects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);

    let fifty: Vec<String> = (0..50).map(|i| format!("t{i}")).collect();
    tagged(&db, &auth, json!(fifty)).await;
    let mut fifty_one = fifty.clone();
    fifty_one.push("t50".into());
    let res = post(
        &db,
        &auth,
        json!({ "type": "note", "title": "n", "tags": fifty_one }),
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        res.json(),
        json!({ "error": "an object can have at most 50 tags" })
    );
    // Duplicates collapse before the limit is checked.
    fifty_one[50] = "T0".into();
    tagged(&db, &auth, json!(fifty_one)).await;
}

#[tokio::test]
async fn patch_replaces_the_tag_set_and_bumps_updated() {
    let (dir, db) = DataDir::new("tags-patch");
    let auth = token(&db).await;
    let id = tagged(&db, &auth, json!(["a", "b"])).await;
    dir.sql()
        .execute("UPDATE objects SET updated = 0 WHERE id = ?1", [id])
        .unwrap();

    let res = patch(&db, &auth, id, json!({ "tags": ["c", "#B"] })).await;
    assert_eq!(res.status, StatusCode::OK);
    let obj = res.json();
    assert_eq!(obj["tags"], json!(["b", "c"]));
    assert_ne!(obj["updated"], "1970-01-01T00:00:00Z");

    let res = patch(&db, &auth, id, json!({ "title": "renamed" })).await;
    assert_eq!(
        res.json()["tags"],
        json!(["b", "c"]),
        "other fields keep the tags"
    );

    let res = patch(&db, &auth, id, json!({ "tags": [] })).await;
    assert_eq!(res.json()["tags"], json!([]));

    let res = patch(&db, &auth, id, json!({ "tags": ["ok", "a b"] })).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json(), json!({ "error": "invalid tag: a b" }));
    let res = patch(&db, &auth, id, json!({ "tags": null })).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn patch_of_a_trashed_object_leaves_its_tags() {
    let (dir, db) = DataDir::new("tags-patch-trash");
    let auth = token(&db).await;
    let id = tagged(&db, &auth, json!(["a"])).await;
    let path = format!("/api/objects/{id}");
    call(&db, Method::DELETE, &path, Some(&auth), None).await;
    let res = patch(&db, &auth, id, json!({ "tags": ["b"] })).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let tag: String = dir
        .sql()
        .query_row(
            "SELECT tag FROM object_tags WHERE object_id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tag, "a");
}

#[tokio::test]
async fn tag_counts_include_parent_folders() {
    let (_dir, db) = DataDir::new("tags-counts");
    let auth = token(&db).await;
    assert_eq!(tag_counts(&db, &auth).await, json!({ "tags": [] }));

    tagged(&db, &auth, json!(["work"])).await;
    tagged(&db, &auth, json!(["work/clienta/design", "work/clientb"])).await;
    tagged(&db, &auth, json!(["workshop"])).await;
    let archived = tagged(&db, &auth, json!(["work/clienta", "old"])).await;
    patch(&db, &auth, archived, json!({ "archived": true })).await;
    let trashed = tagged(&db, &auth, json!(["work", "gone"])).await;
    let path = format!("/api/objects/{trashed}");
    call(&db, Method::DELETE, &path, Some(&auth), None).await;

    assert_eq!(
        tag_counts(&db, &auth).await,
        json!({ "tags": [
            { "tag": "work", "count": 2 },
            { "tag": "work/clienta", "count": 1 },
            { "tag": "work/clienta/design", "count": 1 },
            { "tag": "work/clientb", "count": 1 },
            { "tag": "workshop", "count": 1 },
        ] })
    );
}

#[tokio::test]
async fn tag_routes_require_auth() {
    let (_dir, db) = DataDir::new("tags-auth");
    token(&db).await;
    let res = call(&db, Method::GET, "/api/tags", None, None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}
