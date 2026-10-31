//! `GET /api/lists/{name}/items`: GTD lists grouped for the Kanban and Eisenhower layouts.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, create_task, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

const TODAY: &str = "2026-11-01";

async fn get(db: &Db, auth: &str, path: &str) -> Reply {
    call(db, Method::GET, path, Some(auth), None).await
}

/// The groups of `list` on `TODAY` with the extra query `extra`, each as `[key, label, ids]`.
async fn groups(db: &Db, auth: &str, list: &str, extra: &str) -> Vec<Value> {
    let path = format!("/api/lists/{list}/items?today={TODAY}{extra}");
    let res = get(db, auth, &path).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    let body = res.json();
    assert_eq!(body["truncated"], json!(false));
    body["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| {
            let ids: Vec<i64> = g["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t["id"].as_i64().unwrap())
                .collect();
            json!([g["key"], g["label"], ids])
        })
        .collect()
}

async fn task(db: &Db, auth: &str, fields: Value) -> i64 {
    create_task(db, auth, "t", fields).await
}

#[tokio::test]
async fn requires_auth() {
    let (_dir, db) = DataDir::new("list-layouts-auth");
    let res = call(&db, Method::GET, "/api/lists/inbox/items", None, None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn today_as_kanban_by_priority() {
    let (_dir, db) = DataDir::new("list-layouts-today");
    let auth = token(&db).await;
    let low = task(&db, &auth, json!({ "priority": 3, "scheduled": TODAY })).await;
    let late = task(
        &db,
        &auth,
        json!({ "priority": 1, "deadline": "2026-10-30" }),
    )
    .await;
    let due = task(
        &db,
        &auth,
        json!({ "priority": 1, "scheduled": "2026-10-31" }),
    )
    .await;
    task(
        &db,
        &auth,
        json!({ "priority": 1, "scheduled": "2026-11-05" }),
    )
    .await;

    let by_priority = groups(&db, &auth, "today", "&layout=kanban&group=priority").await;
    assert_eq!(
        by_priority,
        [json!([1, "P1", [late, due]]), json!([3, "P3", [low]])]
    );
    // Kanban without a group groups by status; only open tasks are listed.
    let by_status = groups(&db, &auth, "today", "&layout=kanban").await;
    assert_eq!(by_status, [json!(["open", "open", [late, due, low]])]);
    // `list` (the default layout) with no group is one group in the list's order.
    let list = groups(&db, &auth, "today", "").await;
    assert_eq!(list, [json!([null, "all", [late, due, low]])]);
    // Smart order: the overdue P1 first.
    let smart = groups(&db, &auth, "today", "&sort=smart").await;
    assert_eq!(smart[0][2][0], json!(late));
}

#[tokio::test]
async fn project_all_as_kanban_by_section() {
    let (_dir, db) = DataDir::new("list-layouts-project");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "P").await;
    let other = create_object(&db, &auth, "project", "Q").await;
    let section = |title: &'static str| {
        let (db, auth) = (&db, &auth);
        async move {
            let path = format!("/api/projects/{project}/sections");
            let res = call(
                db,
                Method::POST,
                &path,
                Some(auth),
                Some(json!({ "title": title })),
            )
            .await;
            assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
            res.json()["id"].as_i64().unwrap()
        }
    };
    let (build, plan) = (section("Build").await, section("Plan").await);
    let loose = task(&db, &auth, json!({ "project_id": project })).await;
    let planned = task(
        &db,
        &auth,
        json!({ "project_id": project, "section_id": plan }),
    )
    .await;
    let built = task(
        &db,
        &auth,
        json!({ "project_id": project, "section_id": build }),
    )
    .await;
    task(&db, &auth, json!({ "project_id": other })).await;

    let extra = format!("&project={project}&layout=kanban&group=section");
    assert_eq!(
        groups(&db, &auth, "all", &extra).await,
        [
            json!([build, "Build", [built]]),
            json!([plan, "Plan", [planned]]),
            json!([null, "none", [loose]])
        ]
    );
    let res = get(
        &db,
        &auth,
        &format!("/api/lists/all/items?project={project}"),
    )
    .await;
    assert_eq!(res.json()["total"], json!(3));
}

#[tokio::test]
async fn inbox_as_eisenhower_has_four_groups() {
    let (_dir, db) = DataDir::new("list-layouts-inbox");
    let auth = token(&db).await;
    let key = task(&db, &auth, json!({ "priority": 2 })).await;
    let chore = task(&db, &auth, json!({ "priority": 3 })).await;
    let project = create_object(&db, &auth, "project", "P").await;
    task(&db, &auth, json!({ "project_id": project })).await;

    // Inbox tasks have no dates, so none is urgent; `group` is ignored.
    assert_eq!(
        groups(&db, &auth, "inbox", "&layout=eisenhower&group=tag").await,
        [
            json!(["urgent_important", "urgent & important", []]),
            json!(["important", "important", [key]]),
            json!(["urgent", "urgent", []]),
            json!(["neither", "neither", [chore]])
        ]
    );
}

#[tokio::test]
async fn bad_requests_name_the_problem() {
    let (_dir, db) = DataDir::new("list-layouts-errors");
    let auth = token(&db).await;
    let cases = [
        ("nope/items", StatusCode::NOT_FOUND, "unknown list"),
        (
            "inbox/items?layout=calendar",
            StatusCode::BAD_REQUEST,
            "layout must be one of: list, kanban, eisenhower",
        ),
        (
            "inbox/items?group=colour",
            StatusCode::BAD_REQUEST,
            "group must be one of: none, status, priority, project, section, area, tag, type",
        ),
        (
            "inbox/items?today=2026-11-1",
            StatusCode::BAD_REQUEST,
            "today must be a date YYYY-MM-DD",
        ),
        (
            "inbox/items?sort=title",
            StatusCode::BAD_REQUEST,
            "sort must be default or smart",
        ),
        (
            "all/items?project=0",
            StatusCode::BAD_REQUEST,
            "project must be a positive integer id",
        ),
        (
            "all/items?area=x",
            StatusCode::BAD_REQUEST,
            "area must be a positive integer id",
        ),
    ];
    for (path, status, error) in cases {
        let res = get(&db, &auth, &format!("/api/lists/{path}")).await;
        assert_eq!(res.status, status, "{path}");
        assert_eq!(res.json(), json!({ "error": error }), "{path}");
    }
    let res = get(&db, &auth, "/api/lists/inbox/items?limit=5").await;
    assert_eq!(
        res.status,
        StatusCode::BAD_REQUEST,
        "paging is not accepted"
    );
}
