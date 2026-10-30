//! `GET /api/lists/{name}?sort=`: smart order versus each list's own order, paging and the
//! bad-value error.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_task, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

const TODAY: &str = "2026-11-01";

async fn get(db: &Db, auth: &str, query: &str) -> Reply {
    let path = format!("/api/lists/today?today={TODAY}{query}");
    call(db, Method::GET, &path, Some(auth), None).await
}

/// The ids and `total` of the Today list with the extra query `query`.
async fn today(db: &Db, auth: &str, query: &str) -> (Vec<i64>, i64) {
    let res = get(db, auth, query).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    let body = res.json();
    let ids = body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .collect();
    (ids, body["total"].as_i64().unwrap())
}

/// Seeds a Today list whose smart order differs from its default one; returns the ids
/// in smart order and in default order.
async fn seed(db: &Db, auth: &str) -> (Vec<i64>, Vec<i64>) {
    let mut ids = vec![];
    for fields in [
        // 30: P1, no deadline.
        json!({ "scheduled": TODAY, "priority": 1 }),
        // 50: P3, overdue.
        json!({ "scheduled": TODAY, "priority": 3, "deadline": "2026-10-30" }),
        // 30 + 4: P4, due today, 24-minute estimate.
        json!({ "deadline": TODAY, "estimate": 24 }),
        // 30: P1, no deadline; ties the first by score, after it by id.
        json!({ "scheduled": "2026-10-20", "priority": 1 }),
        // 20 + 1: P2, 96-minute estimate.
        json!({ "scheduled": TODAY, "priority": 2, "estimate": 96 }),
    ] {
        ids.push(create_task(db, auth, "t", fields).await);
    }
    // Not in Today.
    create_task(db, auth, "later", json!({ "scheduled": "2026-11-05" })).await;
    let [p1, overdue, due, p1_late, p2] = ids[..] else {
        unreachable!()
    };
    (
        vec![overdue, due, p1, p1_late, p2],
        vec![p1_late, p1, p2, overdue, due],
    )
}

#[tokio::test]
async fn smart_sort_reorders_the_today_list() {
    let (_dir, db) = DataDir::new("smartsort-today");
    let auth = token(&db).await;
    let (smart, default) = seed(&db, &auth).await;
    assert_eq!(today(&db, &auth, "").await, (default.clone(), 5));
    assert_eq!(today(&db, &auth, "&sort=default").await, (default, 5));
    assert_eq!(today(&db, &auth, "&sort=smart").await, (smart, 5));
}

#[tokio::test]
async fn smart_sort_pages_after_ranking() {
    let (_dir, db) = DataDir::new("smartsort-paging");
    let auth = token(&db).await;
    let (smart, _) = seed(&db, &auth).await;
    let page = today(&db, &auth, "&sort=smart&limit=2&offset=1").await;
    assert_eq!(page, (smart[1..3].to_vec(), 5));
    let past = today(&db, &auth, "&sort=smart&offset=9").await;
    assert_eq!(past, (vec![], 5));
}

#[tokio::test]
async fn other_sort_values_are_rejected() {
    let (_dir, db) = DataDir::new("smartsort-bad");
    let auth = token(&db).await;
    for bad in ["priority", "", "SMART"] {
        let res = get(&db, &auth, &format!("&sort={bad}")).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{bad}");
        let error: Value = res.json();
        assert_eq!(error, json!({ "error": "sort must be default or smart" }));
    }
}
