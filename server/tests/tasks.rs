//! Tasks: `POST /api/tasks`, `GET`/`PATCH /api/tasks/{id}`, complete/reopen, the field
//! rules, the M2 schema and `tasks::store::get_many`.

mod common;
#[path = "tasks/rules.rs"]
mod rules;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, create_task, token};
use polykov_server::{db::Db, tasks::store};
use serde_json::{Value, json};

async fn post(db: &Db, auth: &str, path: &str, body: Option<Value>) -> Reply {
    call(db, Method::POST, path, Some(auth), body).await
}

async fn get(db: &Db, auth: &str, id: i64) -> Reply {
    let path = format!("/api/tasks/{id}");
    call(db, Method::GET, &path, Some(auth), None).await
}

async fn patch(db: &Db, auth: &str, id: i64, body: Value) -> Reply {
    let path = format!("/api/tasks/{id}");
    call(db, Method::PATCH, &path, Some(auth), Some(body)).await
}

async fn patch_task(db: &Db, auth: &str, id: i64, task: Value) -> Reply {
    patch(db, auth, id, json!({ "task": task })).await
}

fn assert_error(res: &Reply, status: StatusCode, error: &str) {
    assert_eq!(res.status, status, "{:?}", res.json());
    assert_eq!(res.json(), json!({ "error": error }));
}

fn defaults() -> Value {
    json!({
        "status": "open", "priority": 4, "scheduled": null, "deadline": null, "estimate": null,
        "someday": false, "parent_id": null, "project_id": null, "section_id": null,
        "area_id": null, "checklist": [], "recurrence": null, "completed": null
    })
}

fn add_section(dir: &DataDir, project: i64) -> i64 {
    let conn = dir.sql();
    conn.execute(
        "INSERT INTO sections (project_id, title, position) VALUES (?1, 's', 0)",
        [project],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[tokio::test]
async fn create_returns_201_with_object_and_task_fields() {
    let (_dir, db) = DataDir::new("task-create");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "Launch").await;
    let area = create_object(&db, &auth, "area", "Work").await;
    let body = json!({
        "title": "  Write copy ", "tags": ["#Work"], "properties": { "k": 1 }, "pinned": true,
        "task": {
            "priority": 1, "scheduled": "2026-11-02", "deadline": "2026-11-03T17:30",
            "estimate": 90, "someday": true, "project_id": project, "area_id": area,
            "checklist": [{ "text": " draft ", "done": true }, { "text": "edit" }]
        }
    });
    let res = post(&db, &auth, "/api/tasks", Some(body)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    let t = res.json();
    assert_eq!(t["type"], "task");
    assert_eq!(t["title"], "Write copy");
    assert_eq!(t["tags"], json!(["work"]));
    assert_eq!(t["properties"], json!({ "k": 1 }));
    assert_eq!(t["pinned"], true);
    let mut want = defaults();
    want["priority"] = json!(1);
    want["scheduled"] = json!("2026-11-02");
    want["deadline"] = json!("2026-11-03T17:30");
    want["estimate"] = json!(90);
    want["someday"] = json!(true);
    want["project_id"] = json!(project);
    want["area_id"] = json!(area);
    want["checklist"] =
        json!([{ "text": "draft", "done": true }, { "text": "edit", "done": false }]);
    assert_eq!(t["task"], want);
    assert_eq!(get(&db, &auth, t["id"].as_i64().unwrap()).await.json(), t);
}

#[tokio::test]
async fn create_with_a_broken_rule_writes_nothing() {
    let (dir, db) = DataDir::new("task-create-tx");
    let auth = token(&db).await;
    let body = json!({ "title": "x", "tags": ["a"], "task": { "project_id": 999 } });
    let res = post(&db, &auth, "/api/tasks", Some(body)).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "project must be a live project",
    );
    let n: i64 = dir
        .sql()
        .query_row("SELECT COUNT(*) FROM objects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
    let res = post(&db, &auth, "/api/tasks", Some(json!({ "title": "  " }))).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "title must be 1 to 1000 characters",
    );
}

#[tokio::test]
async fn generic_task_objects_read_with_defaults_and_no_row() {
    let (dir, db) = DataDir::new("task-generic");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "Plain").await;
    let res = get(&db, &auth, id).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["task"], defaults());
    let rows: i64 = dir
        .sql()
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
    let res = patch_task(&db, &auth, id, json!({ "priority": 2 })).await;
    assert_eq!(res.json()["task"]["priority"], 2);
}

#[tokio::test]
async fn patch_sets_and_clears_fields_and_bumps_updated() {
    let (dir, db) = DataDir::new("task-patch");
    let auth = token(&db).await;
    let id = create_task(
        &db,
        &auth,
        "t",
        json!({ "deadline": "2026-11-01", "estimate": 5 }),
    )
    .await;
    dir.sql()
        .execute("UPDATE objects SET updated = 0 WHERE id = ?1", [id])
        .unwrap();
    let res = patch(
        &db,
        &auth,
        id,
        json!({ "title": "renamed", "archived": true, "task": { "deadline": null, "estimate": null, "scheduled": "2026-12-01T08:00" } }),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    let t = res.json();
    assert_eq!(t["title"], "renamed");
    assert_eq!(t["archived"], true);
    assert_eq!(t["task"]["deadline"], Value::Null);
    assert_eq!(t["task"]["estimate"], Value::Null);
    assert_eq!(t["task"]["scheduled"], "2026-12-01T08:00");
    assert_ne!(t["updated"], "1970-01-01T00:00:00Z");

    dir.sql()
        .execute("UPDATE objects SET updated = 0 WHERE id = ?1", [id])
        .unwrap();
    let res = patch(&db, &auth, id, json!({})).await;
    assert_eq!(res.json()["updated"], "1970-01-01T00:00:00Z");
    let res = patch_task(&db, &auth, id, json!({ "someday": true })).await;
    assert_ne!(res.json()["updated"], "1970-01-01T00:00:00Z");
}

#[tokio::test]
async fn patch_rejects_status_completed_and_null_for_required_fields() {
    let (_dir, db) = DataDir::new("task-patch-422");
    let auth = token(&db).await;
    let id = create_task(&db, &auth, "t", json!({})).await;
    for task in [
        json!({ "status": "done" }),
        json!({ "completed": null }),
        json!({ "priority": null }),
        json!({ "checklist": null }),
        json!({ "recurrence": null }),
    ] {
        let res = patch_task(&db, &auth, id, task.clone()).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{task}");
    }
}

#[tokio::test]
async fn complete_and_reopen() {
    let (dir, db) = DataDir::new("task-complete");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "t").await;
    let res = post(&db, &auth, &format!("/api/tasks/{id}/complete"), None).await;
    assert_eq!(res.status, StatusCode::OK);
    let body = res.json();
    assert_eq!(body["next"], Value::Null);
    assert_eq!(body["task"]["id"], id);
    assert_eq!(body["task"]["task"]["status"], "done");
    assert_eq!(
        body["task"]["task"]["completed"].as_str().unwrap().len(),
        20
    );

    dir.sql()
        .execute(
            "UPDATE tasks SET completed = 86400 WHERE object_id = ?1",
            [id],
        )
        .unwrap();
    let res = post(&db, &auth, &format!("/api/tasks/{id}/complete"), None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.json()["task"]["task"]["completed"],
        "1970-01-02T00:00:00Z"
    );

    let res = post(&db, &auth, &format!("/api/tasks/{id}/reopen"), None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["task"]["status"], "open");
    assert_eq!(res.json()["task"]["completed"], Value::Null);
    let res = post(&db, &auth, &format!("/api/tasks/{id}/reopen"), None).await;
    assert_eq!(res.json()["task"]["status"], "open");
}

#[tokio::test]
async fn unknown_trashed_or_non_task_ids_are_404() {
    let (_dir, db) = DataDir::new("task-404");
    let auth = token(&db).await;
    let note = create_object(&db, &auth, "note", "n").await;
    let gone = create_task(&db, &auth, "gone", json!({})).await;
    call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{gone}"),
        Some(&auth),
        None,
    )
    .await;
    for id in [note, gone, 999] {
        assert_error(
            &get(&db, &auth, id).await,
            StatusCode::NOT_FOUND,
            "task not found",
        );
        let res = patch_task(&db, &auth, id, json!({ "priority": 1 })).await;
        assert_error(&res, StatusCode::NOT_FOUND, "task not found");
        for action in ["complete", "reopen"] {
            let res = post(&db, &auth, &format!("/api/tasks/{id}/{action}"), None).await;
            assert_error(&res, StatusCode::NOT_FOUND, "task not found");
        }
    }
}

#[tokio::test]
async fn every_route_requires_auth() {
    let (_dir, db) = DataDir::new("task-401");
    let auth = token(&db).await;
    let id = create_task(&db, &auth, "t", json!({})).await;
    let routes = [
        (
            Method::POST,
            "/api/tasks".to_owned(),
            Some(json!({ "title": "x" })),
        ),
        (Method::GET, format!("/api/tasks/{id}"), None),
        (Method::PATCH, format!("/api/tasks/{id}"), Some(json!({}))),
        (Method::POST, format!("/api/tasks/{id}/complete"), None),
        (Method::POST, format!("/api/tasks/{id}/reopen"), None),
    ];
    for (method, path, body) in routes {
        let res = call(&db, method, &path, None, body).await;
        assert_error(&res, StatusCode::UNAUTHORIZED, "authentication required");
    }
}

#[tokio::test]
async fn get_many_keeps_order_and_skips_non_tasks() {
    let (_dir, db) = DataDir::new("task-many");
    let auth = token(&db).await;
    let a = create_task(&db, &auth, "a", json!({})).await;
    let b = create_task(&db, &auth, "b", json!({})).await;
    let note = create_object(&db, &auth, "note", "n").await;
    let ids: Vec<i64> = store::get_many(&db, &[b, note, 999, a])
        .unwrap()
        .iter()
        .map(|t| t.object.id)
        .collect();
    assert_eq!(ids, vec![b, a]);
}

#[tokio::test]
async fn schema_cascades_and_sets_null_on_purge() {
    let (dir, db) = DataDir::new("task-schema");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "p").await;
    let section = add_section(&dir, project);
    let parent = create_task(&db, &auth, "parent", json!({})).await;
    let child = create_task(
        &db,
        &auth,
        "child",
        json!({ "parent_id": parent, "project_id": project, "section_id": section }),
    )
    .await;
    let conn = dir.sql();
    conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
    conn.execute(
        "DELETE FROM objects WHERE id IN (?1, ?2)",
        [parent, project],
    )
    .unwrap();
    let t = get(&db, &auth, child).await.json()["task"].clone();
    assert_eq!(
        (&t["parent_id"], &t["project_id"], &t["section_id"]),
        (&Value::Null, &Value::Null, &Value::Null)
    );
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
    assert_eq!(count("SELECT COUNT(*) FROM tasks"), 1);
    assert_eq!(count("SELECT COUNT(*) FROM sections"), 0);
    let indexes = count(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name IN \
         ('tasks_parent', 'tasks_project', 'tasks_area', 'tasks_scheduled', 'tasks_deadline', \
         'sections_project')",
    );
    assert_eq!(indexes, 6);
    conn.execute(
        "INSERT INTO objects (type, title) VALUES ('project', 'q')",
        [],
    )
    .unwrap();
    let q = conn.last_insert_rowid();
    conn.execute("INSERT INTO projects (object_id) VALUES (?1)", [q])
        .unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM projects WHERE object_id = ?1",
            [q],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "active");
}
