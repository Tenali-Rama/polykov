//! `GET /api/objects`: filters, tag folders, ordering, paging and its 10k-object budget.

mod common;

use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use common::{DataDir, call, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

/// Creates an object through the API and returns its id.
async fn create(db: &Db, auth: &str, body: Value) -> i64 {
    let res = call(db, Method::POST, "/api/objects", Some(auth), Some(body)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    res.json()["id"].as_i64().unwrap()
}

/// The ids listed by `GET /api/objects?<query>`, and its `total`.
async fn ids(db: &Db, auth: &str, query: &str) -> (Vec<i64>, i64) {
    let res = call(
        db,
        Method::GET,
        &format!("/api/objects?{query}"),
        Some(auth),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{query}: {:?}", res.json());
    let body = res.json();
    let ids = body["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_i64().unwrap())
        .collect();
    (ids, body["total"].as_i64().unwrap())
}

#[tokio::test]
async fn lists_live_unarchived_objects_by_default() {
    let (_dir, db) = DataDir::new("list-default");
    let auth = token(&db).await;
    assert_eq!(ids(&db, &auth, "").await, (vec![], 0));

    let note = create(
        &db,
        &auth,
        json!({ "type": "note", "title": "n", "tags": ["B", "a"] }),
    )
    .await;
    let archived = create(
        &db,
        &auth,
        json!({ "type": "note", "title": "a", "archived": true }),
    )
    .await;
    let trashed = create(&db, &auth, json!({ "type": "note", "title": "t" })).await;
    let path = format!("/api/objects/{trashed}");
    call(&db, Method::DELETE, &path, Some(&auth), None).await;

    let res = call(&db, Method::GET, "/api/objects", Some(&auth), None).await;
    let body = res.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["objects"][0]["id"], note);
    assert_eq!(body["objects"][0]["tags"], json!(["a", "b"]));

    assert_eq!(ids(&db, &auth, "archived=true").await, (vec![archived], 1));
    assert_eq!(ids(&db, &auth, "archived=false").await, (vec![note], 1));
    let (mut all, total) = ids(&db, &auth, "archived=any").await;
    all.sort();
    assert_eq!(
        (all, total),
        (vec![note, archived], 2),
        "trash never listed"
    );
}

#[tokio::test]
async fn filters_by_type_and_pinned() {
    let (_dir, db) = DataDir::new("list-filters");
    let auth = token(&db).await;
    let task = create(
        &db,
        &auth,
        json!({ "type": "task", "title": "t", "pinned": true }),
    )
    .await;
    let note = create(&db, &auth, json!({ "type": "note", "title": "n" })).await;
    assert_eq!(ids(&db, &auth, "type=task").await, (vec![task], 1));
    assert_eq!(ids(&db, &auth, "type=note").await, (vec![note], 1));
    assert_eq!(ids(&db, &auth, "type=habit").await, (vec![], 0));
    assert_eq!(ids(&db, &auth, "pinned=true").await, (vec![task], 1));
    assert_eq!(ids(&db, &auth, "pinned=false").await, (vec![note], 1));
    assert_eq!(ids(&db, &auth, "type=task&pinned=false").await, (vec![], 0));
}

#[tokio::test]
async fn tag_filter_matches_the_folder_not_a_prefix() {
    let (_dir, db) = DataDir::new("list-tag");
    let auth = token(&db).await;
    let work = create(
        &db,
        &auth,
        json!({ "type": "note", "title": "w", "tags": ["work"] }),
    )
    .await;
    let deep = create(
        &db,
        &auth,
        json!({ "type": "note", "title": "d", "tags": ["work/clienta/design"] }),
    )
    .await;
    create(
        &db,
        &auth,
        json!({ "type": "note", "title": "s", "tags": ["workshop"] }),
    )
    .await;
    create(
        &db,
        &auth,
        json!({ "type": "note", "title": "x", "tags": ["work0", "work-x"] }),
    )
    .await;

    let (mut got, total) = ids(&db, &auth, "tag=work").await;
    got.sort();
    assert_eq!((got, total), (vec![work, deep], 2));
    assert_eq!(
        ids(&db, &auth, "tag=%23Work%2FClientA").await,
        (vec![deep], 1)
    );
    assert_eq!(
        ids(&db, &auth, "tag=work/clienta/design").await,
        (vec![deep], 1)
    );
    assert_eq!(ids(&db, &auth, "tag=work/client").await, (vec![], 0));
}

#[tokio::test]
async fn orders_pinned_first_then_updated_then_id() {
    let (dir, db) = DataDir::new("list-order");
    let auth = token(&db).await;
    let mut made = Vec::new();
    for _ in 0..4 {
        made.push(create(&db, &auth, json!({ "type": "note", "title": "n" })).await);
    }
    let [a, b, c, d] = made[..] else {
        unreachable!()
    };
    let sql = dir.sql();
    sql.execute_batch(&format!(
        "UPDATE objects SET updated = 100; UPDATE objects SET updated = 200 WHERE id = {a};
         UPDATE objects SET pinned = 1, updated = 50 WHERE id = {d};"
    ))
    .unwrap();
    // d is pinned; a is newest; b and c tie on updated, so the higher id comes first.
    assert_eq!(ids(&db, &auth, "").await, (vec![d, a, c, b], 4));
}

#[tokio::test]
async fn pages_with_limit_and_offset() {
    let (_dir, db) = DataDir::new("list-paging");
    let auth = token(&db).await;
    let mut made = Vec::new();
    for _ in 0..5 {
        made.push(create(&db, &auth, json!({ "type": "note", "title": "n" })).await);
    }
    made.reverse(); // same `updated` second, so id desc
    assert_eq!(ids(&db, &auth, "limit=2").await, (made[..2].to_vec(), 5));
    assert_eq!(
        ids(&db, &auth, "limit=2&offset=2").await,
        (made[2..4].to_vec(), 5)
    );
    assert_eq!(
        ids(&db, &auth, "limit=2&offset=4").await,
        (made[4..].to_vec(), 5)
    );
    assert_eq!(ids(&db, &auth, "offset=10").await, (vec![], 5));
    assert_eq!(ids(&db, &auth, "limit=1000").await.0.len(), 5);
}

#[tokio::test]
async fn bad_parameters_get_400() {
    let (_dir, db) = DataDir::new("list-bad");
    let auth = token(&db).await;
    let cases = [
        ("type=thing", None),
        ("tag=a%20b", Some("invalid tag: a b")),
        ("pinned=yes", Some("pinned must be true or false")),
        ("pinned=any", Some("pinned must be true or false")),
        ("archived=all", Some("archived must be false, true or any")),
        ("limit=0", Some("limit must be 1 to 1000")),
        ("limit=1001", Some("limit must be 1 to 1000")),
        ("limit=ten", Some("limit must be 1 to 1000")),
        ("offset=-1", Some("offset must be a non-negative integer")),
        ("colour=red", None),
    ];
    for (query, error) in cases {
        let path = format!("/api/objects?{query}");
        let res = call(&db, Method::GET, &path, Some(&auth), None).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{query}");
        if let Some(error) = error {
            assert_eq!(res.json(), json!({ "error": error }), "{query}");
        }
    }
    let res = call(&db, Method::GET, "/api/objects", None, None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn tag_filter_over_10k_objects_is_fast() {
    let (dir, db) = DataDir::new("list-10k");
    let auth = token(&db).await;
    let mut sql = dir.sql();
    let tx = sql.transaction().unwrap();
    for i in 0..10_000 {
        tx.execute("INSERT INTO objects (type, title) VALUES ('note', 'n')", [])
            .unwrap();
        let id = tx.last_insert_rowid();
        let tag = if i % 100 == 0 {
            "x".to_owned()
        } else {
            format!("t{}/x", i % 50)
        };
        tx.execute("INSERT INTO object_tags VALUES (?1, ?2)", (id, tag))
            .unwrap();
    }
    tx.commit().unwrap();

    let start = Instant::now();
    let (got, total) = ids(&db, &auth, "tag=x").await;
    let took = start.elapsed();
    assert_eq!((got.len(), total), (100, 100));
    if !cfg!(debug_assertions) {
        assert!(took < Duration::from_millis(100), "took {took:?}");
    }
}
