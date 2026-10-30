//! GTD lists: `GET /api/lists/{name}` (the list rules, filters, paging and errors) and `GET
//! /api/tasks/{id}/subtasks`.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, create_task, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

const TODAY: &str = "2026-11-01";

async fn get(db: &Db, auth: &str, path: &str) -> Reply {
    call(db, Method::GET, path, Some(auth), None).await
}

/// The ids of `list` on `TODAY` with the extra query `extra`, in order.
async fn ids(db: &Db, auth: &str, list: &str, extra: &str) -> Vec<i64> {
    let res = get(db, auth, &format!("/api/lists/{list}?today={TODAY}{extra}")).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    ids_of(&res.json()["tasks"])
}

fn ids_of(tasks: &Value) -> Vec<i64> {
    tasks
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .collect()
}

async fn trash(db: &Db, auth: &str, id: i64) {
    let path = format!("/api/objects/{id}");
    let res = call(db, Method::DELETE, &path, Some(auth), None).await;
    assert!(res.status.is_success(), "{}", res.status);
}

async fn task(db: &Db, auth: &str, fields: Value) -> i64 {
    create_task(db, auth, "t", fields).await
}

fn assert_error(res: &Reply, status: StatusCode, error: &str) {
    assert_eq!(res.status, status, "{:?}", res.json());
    assert_eq!(res.json(), json!({ "error": error }));
}

#[tokio::test]
async fn routes_require_auth() {
    let (_dir, db) = DataDir::new("lists-auth");
    for path in ["/api/lists/inbox", "/api/tasks/1/subtasks"] {
        let res = call(&db, Method::GET, path, None, None).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{path}");
    }
}

#[tokio::test]
async fn inbox_holds_unfiled_undated_tasks_newest_first() {
    let (_dir, db) = DataDir::new("lists-inbox");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "P").await;
    let area = create_object(&db, &auth, "area", "A").await;
    let first = task(&db, &auth, json!({})).await;
    let rowless = create_object(&db, &auth, "task", "made as an object").await;
    let parent = task(&db, &auth, json!({ "project_id": project })).await;
    for fields in [
        json!({ "project_id": project }),
        json!({ "area_id": area }),
        json!({ "parent_id": parent }),
        json!({ "scheduled": "2026-12-01" }),
        json!({ "deadline": "2026-12-01" }),
        json!({ "someday": true }),
    ] {
        task(&db, &auth, fields).await;
    }
    assert_eq!(ids(&db, &auth, "inbox", "").await, [rowless, first]);
}

#[tokio::test]
async fn today_holds_due_and_overdue_tasks_at_any_depth() {
    let (_dir, db) = DataDir::new("lists-today");
    let auth = token(&db).await;
    let late = task(
        &db,
        &auth,
        json!({ "scheduled": "2026-10-01", "priority": 2 }),
    )
    .await;
    let now = task(
        &db,
        &auth,
        json!({ "scheduled": "2026-11-01T09:00", "priority": 2 }),
    )
    .await;
    let due = task(
        &db,
        &auth,
        json!({ "deadline": "2026-11-01", "priority": 2 }),
    )
    .await;
    let p1 = task(&db, &auth, json!({ "scheduled": TODAY, "priority": 1 })).await;
    let child = task(&db, &auth, json!({ "parent_id": p1, "scheduled": TODAY })).await;
    task(
        &db,
        &auth,
        json!({ "scheduled": "2026-11-02", "deadline": "2026-11-09" }),
    )
    .await;
    task(&db, &auth, json!({})).await;
    // Priority, then deadline (nulls last), then scheduled time, then id.
    assert_eq!(
        ids(&db, &auth, "today", "").await,
        [p1, due, late, now, child]
    );
}

#[tokio::test]
async fn upcoming_holds_future_dated_tasks_by_date_then_priority() {
    // Same-day ties go by priority, whatever the time of day.
    let (_dir, db) = DataDir::new("lists-upcoming");
    let auth = token(&db).await;
    let later = task(&db, &auth, json!({ "scheduled": "2026-11-05" })).await;
    let due = task(&db, &auth, json!({ "deadline": "2026-11-03" })).await;
    let low = task(
        &db,
        &auth,
        json!({ "scheduled": "2026-11-02", "priority": 3 }),
    )
    .await;
    let parent = task(&db, &auth, json!({})).await;
    let high = task(
        &db,
        &auth,
        json!({ "scheduled": "2026-11-02T09:00", "priority": 1, "parent_id": parent }),
    )
    .await;
    task(
        &db,
        &auth,
        json!({ "scheduled": TODAY, "deadline": "2026-11-09" }),
    )
    .await;
    task(&db, &auth, json!({ "deadline": TODAY })).await;
    assert_eq!(
        ids(&db, &auth, "upcoming", "").await,
        [high, low, due, later]
    );
}

#[tokio::test]
async fn someday_holds_undated_top_level_someday_tasks() {
    let (_dir, db) = DataDir::new("lists-someday");
    let auth = token(&db).await;
    let a = task(&db, &auth, json!({ "someday": true, "priority": 3 })).await;
    let b = task(&db, &auth, json!({ "someday": true, "priority": 1 })).await;
    task(
        &db,
        &auth,
        json!({ "someday": true, "scheduled": "2026-12-01" }),
    )
    .await;
    task(&db, &auth, json!({ "someday": true, "parent_id": a })).await;
    let gone = task(&db, &auth, json!({})).await;
    let orphan = task(&db, &auth, json!({ "someday": true, "parent_id": gone })).await;
    trash(&db, &auth, gone).await;
    task(&db, &auth, json!({})).await;
    assert_eq!(ids(&db, &auth, "someday", "").await, [b, a, orphan]);
}

#[tokio::test]
async fn anytime_holds_filed_available_top_level_tasks() {
    let (_dir, db) = DataDir::new("lists-anytime");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "P").await;
    let area = create_object(&db, &auth, "area", "A").await;
    let filed = task(&db, &auth, json!({ "project_id": project })).await;
    let urgent = task(&db, &auth, json!({ "area_id": area, "priority": 1 })).await;
    let now = task(&db, &auth, json!({ "scheduled": TODAY })).await;
    let due = task(&db, &auth, json!({ "deadline": "2026-12-01" })).await;
    task(&db, &auth, json!({})).await;
    task(
        &db,
        &auth,
        json!({ "project_id": project, "someday": true }),
    )
    .await;
    task(
        &db,
        &auth,
        json!({ "project_id": project, "scheduled": "2026-11-02" }),
    )
    .await;
    task(
        &db,
        &auth,
        json!({ "project_id": project, "parent_id": filed }),
    )
    .await;
    assert_eq!(
        ids(&db, &auth, "anytime", "").await,
        [urgent, filed, now, due]
    );
}

#[tokio::test]
async fn only_live_unarchived_open_tasks_are_listed() {
    let (_dir, db) = DataDir::new("lists-open");
    let auth = token(&db).await;
    let open = task(&db, &auth, json!({})).await;
    let done = task(&db, &auth, json!({})).await;
    let archived = task(&db, &auth, json!({})).await;
    let trashed = task(&db, &auth, json!({})).await;
    task(&db, &auth, json!({ "parent_id": open })).await;
    let path = format!("/api/tasks/{done}/complete");
    call(&db, Method::POST, &path, Some(&auth), None).await;
    let path = format!("/api/tasks/{archived}");
    call(
        &db,
        Method::PATCH,
        &path,
        Some(&auth),
        Some(json!({ "archived": true })),
    )
    .await;
    trash(&db, &auth, trashed).await;
    assert_eq!(ids(&db, &auth, "all", "").await, [open]);
}

#[tokio::test]
async fn trashed_project_and_area_count_as_none() {
    let (_dir, db) = DataDir::new("lists-trashed-refs");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "P").await;
    let area = create_object(&db, &auth, "area", "A").await;
    let a = task(&db, &auth, json!({ "project_id": project })).await;
    let b = task(&db, &auth, json!({ "area_id": area })).await;
    assert_eq!(ids(&db, &auth, "inbox", "").await, Vec::<i64>::new());
    trash(&db, &auth, project).await;
    trash(&db, &auth, area).await;
    assert_eq!(ids(&db, &auth, "inbox", "").await, [b, a]);
    assert_eq!(ids(&db, &auth, "anytime", "").await, Vec::<i64>::new());
}

#[tokio::test]
async fn project_and_area_filters() {
    let (dir, db) = DataDir::new("lists-filters");
    let auth = token(&db).await;
    let area = create_object(&db, &auth, "area", "Work").await;
    let other_area = create_object(&db, &auth, "area", "Home").await;
    let project = create_object(&db, &auth, "project", "Launch").await;
    let other = create_object(&db, &auth, "project", "Other").await;
    dir.sql()
        .execute(
            "INSERT INTO projects (object_id, area_id) VALUES (?1, ?2)",
            [project, area],
        )
        .unwrap();
    let in_project = task(&db, &auth, json!({ "project_id": project })).await;
    let in_area = task(&db, &auth, json!({ "area_id": area })).await;
    let elsewhere = task(&db, &auth, json!({ "project_id": other })).await;
    task(&db, &auth, json!({ "area_id": other_area })).await;
    task(&db, &auth, json!({})).await;
    let project_view = ids(&db, &auth, "all", &format!("&project={project}")).await;
    assert_eq!(project_view, [in_project]);
    let area_view = ids(&db, &auth, "all", &format!("&area={area}")).await;
    assert_eq!(area_view, [in_project, in_area]);
    let other_view = ids(&db, &auth, "anytime", &format!("&project={other}")).await;
    assert_eq!(other_view, [elsewhere]);
    // A trashed area counts as none, also when reached through a project.
    trash(&db, &auth, area).await;
    let res = get(&db, &auth, &format!("/api/lists/all?area={area}")).await;
    assert_eq!(res.json(), json!({ "tasks": [], "total": 0 }));
}

#[tokio::test]
async fn pages_report_the_total() {
    let (_dir, db) = DataDir::new("lists-paging");
    let auth = token(&db).await;
    let mut all = Vec::new();
    for _ in 0..5 {
        all.push(task(&db, &auth, json!({})).await);
    }
    let res = get(&db, &auth, "/api/lists/all?limit=2&offset=1").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["total"], 5);
    assert_eq!(ids_of(&res.json()["tasks"]), all[1..3]);
    let res = get(&db, &auth, "/api/lists/all?limit=2&offset=10").await;
    assert_eq!(res.json(), json!({ "tasks": [], "total": 5 }));
}

#[tokio::test]
async fn today_defaults_to_the_server_utc_date() {
    let (_dir, db) = DataDir::new("lists-default-today");
    let auth = token(&db).await;
    let utc = chrono::Utc::now().date_naive();
    let today = task(&db, &auth, json!({ "scheduled": utc.to_string() })).await;
    let tomorrow = utc.succ_opt().unwrap().to_string();
    task(&db, &auth, json!({ "scheduled": tomorrow })).await;
    let res = get(&db, &auth, "/api/lists/today").await;
    assert_eq!(ids_of(&res.json()["tasks"]), [today]);
}

#[tokio::test]
async fn rows_carry_subtask_counts() {
    let (_dir, db) = DataDir::new("lists-counts");
    let auth = token(&db).await;
    let parent = task(&db, &auth, json!({})).await;
    let done = task(&db, &auth, json!({ "parent_id": parent })).await;
    task(&db, &auth, json!({ "parent_id": parent })).await;
    let trashed = task(&db, &auth, json!({ "parent_id": parent })).await;
    call(
        &db,
        Method::POST,
        &format!("/api/tasks/{done}/complete"),
        Some(&auth),
        None,
    )
    .await;
    trash(&db, &auth, trashed).await;
    let res = get(&db, &auth, "/api/lists/inbox").await;
    let row = &res.json()["tasks"][0];
    assert_eq!(row["id"], parent);
    assert_eq!(row["task"]["status"], "open");
    assert_eq!(row["subtasks"], json!({ "open": 1, "total": 2 }));
}

#[tokio::test]
async fn bad_requests_are_rejected() {
    let (_dir, db) = DataDir::new("lists-errors");
    let auth = token(&db).await;
    let res = get(&db, &auth, "/api/lists/logbook").await;
    assert_error(&res, StatusCode::NOT_FOUND, "unknown list");
    for (query, error) in [
        ("today=2026-13-01", "today must be a date YYYY-MM-DD"),
        ("today=2026-11-01T09:00", "today must be a date YYYY-MM-DD"),
        ("limit=0", "limit must be 1 to 1000"),
        ("limit=1001", "limit must be 1 to 1000"),
        ("offset=-1", "offset must be a non-negative integer"),
        ("project=abc", "project must be a positive integer id"),
        ("area=0", "area must be a positive integer id"),
    ] {
        let res = get(&db, &auth, &format!("/api/lists/today?{query}")).await;
        assert_error(&res, StatusCode::BAD_REQUEST, error);
    }
}

#[tokio::test]
async fn subtasks_lists_direct_children_open_first() {
    let (_dir, db) = DataDir::new("lists-subtasks");
    let auth = token(&db).await;
    let parent = task(&db, &auth, json!({})).await;
    let a = task(&db, &auth, json!({ "parent_id": parent })).await;
    let b = task(&db, &auth, json!({ "parent_id": parent })).await;
    let c = task(&db, &auth, json!({ "parent_id": parent })).await;
    let grandchild = task(&db, &auth, json!({ "parent_id": c })).await;
    let trashed = task(&db, &auth, json!({ "parent_id": parent })).await;
    call(
        &db,
        Method::POST,
        &format!("/api/tasks/{a}/complete"),
        Some(&auth),
        None,
    )
    .await;
    trash(&db, &auth, trashed).await;
    let res = get(&db, &auth, &format!("/api/tasks/{parent}/subtasks")).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    let tasks = &res.json()["tasks"];
    assert_eq!(ids_of(tasks), [b, c, a]);
    assert_eq!(tasks[1]["subtasks"], json!({ "open": 1, "total": 1 }));
    let res = get(&db, &auth, &format!("/api/tasks/{grandchild}/subtasks")).await;
    assert_eq!(res.json(), json!({ "tasks": [] }));
}

#[tokio::test]
async fn subtasks_of_a_non_task_is_404() {
    let (_dir, db) = DataDir::new("lists-subtasks-404");
    let auth = token(&db).await;
    let note = create_object(&db, &auth, "note", "N").await;
    let gone = task(&db, &auth, json!({})).await;
    trash(&db, &auth, gone).await;
    for id in [note, gone, 9999] {
        let res = get(&db, &auth, &format!("/api/tasks/{id}/subtasks")).await;
        assert_error(&res, StatusCode::NOT_FOUND, "task not found");
    }
}
