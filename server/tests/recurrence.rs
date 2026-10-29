//! Recurring tasks: the `recurrence` field and the next occurrence made by
//! `POST /api/tasks/{id}/complete[?today=YYYY-MM-DD]`.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, create_task, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

async fn complete(db: &Db, auth: &str, id: i64, today: &str) -> Reply {
    let path = format!("/api/tasks/{id}/complete?today={today}");
    call(db, Method::POST, &path, Some(auth), None).await
}

/// Completes `id` on `today` and returns `(done, next)` task JSON.
async fn complete_ok(db: &Db, auth: &str, id: i64, today: &str) -> (Value, Value) {
    let res = complete(db, auth, id, today).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    let body = res.json();
    (body["task"].clone(), body["next"].clone())
}

fn weekly(from: &str) -> Value {
    json!({ "rule": "FREQ=WEEKLY", "from": from })
}

#[tokio::test]
async fn recurrence_is_stored_canonical_and_can_be_cleared() {
    let (dir, db) = DataDir::new("recur-field");
    let auth = token(&db).await;
    let body = json!({ "title": "t", "task": { "recurrence": {
        "rule": "byday=su,mo;freq=weekly", "from": "completion"
    } } });
    let res = call(&db, Method::POST, "/api/tasks", Some(&auth), Some(body)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    let want = json!({ "rule": "FREQ=WEEKLY;BYDAY=MO,SU", "from": "completion" });
    assert_eq!(res.json()["task"]["recurrence"], want);
    let id = res.json()["id"].as_i64().unwrap();
    let stored: String = dir
        .sql()
        .query_row(
            "SELECT recurrence FROM tasks WHERE object_id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&stored).unwrap(), want);

    let path = format!("/api/tasks/{id}");
    let patch = json!({ "task": { "recurrence": weekly("scheduled") } });
    let res = call(&db, Method::PATCH, &path, Some(&auth), Some(patch)).await;
    assert_eq!(res.json()["task"]["recurrence"], weekly("scheduled"));
    let patch = json!({ "task": { "recurrence": null } });
    let res = call(&db, Method::PATCH, &path, Some(&auth), Some(patch)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["task"]["recurrence"], Value::Null);
}

#[tokio::test]
async fn bad_rules_are_400_and_bad_shapes_422() {
    let (_dir, db) = DataDir::new("recur-400");
    let auth = token(&db).await;
    let id = create_task(&db, &auth, "t", json!({})).await;
    let path = format!("/api/tasks/{id}");
    for (rule, reason) in [
        (
            "FREQ=HOURLY",
            "FREQ must be DAILY, WEEKLY, MONTHLY or YEARLY",
        ),
        ("FREQ=DAILY;BYDAY=MO", "BYDAY needs FREQ=WEEKLY"),
        ("FREQ=DAILY;INTERVAL=0", "INTERVAL must be 1 to 365"),
    ] {
        let task = json!({ "recurrence": { "rule": rule, "from": "scheduled" } });
        let res = call(
            &db,
            Method::POST,
            "/api/tasks",
            Some(&auth),
            Some(json!({
                "title": "t", "task": task.clone()
            })),
        )
        .await;
        let error = json!({ "error": format!("invalid recurrence: {reason}") });
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{rule}");
        assert_eq!(res.json(), error);
        let res = call(
            &db,
            Method::PATCH,
            &path,
            Some(&auth),
            Some(json!({ "task": task })),
        )
        .await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{rule}");
        assert_eq!(res.json(), error);
    }
    for recurrence in [
        json!({ "rule": "FREQ=DAILY", "from": "deadline" }),
        json!({ "rule": "FREQ=DAILY" }),
        json!("FREQ=DAILY"),
    ] {
        let body = json!({ "task": { "recurrence": recurrence } });
        let res = call(&db, Method::PATCH, &path, Some(&auth), Some(body)).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{recurrence}");
    }
}

#[tokio::test]
async fn non_recurring_complete_has_no_next() {
    let (_dir, db) = DataDir::new("recur-none");
    let auth = token(&db).await;
    let id = create_task(&db, &auth, "t", json!({ "scheduled": "2026-11-01" })).await;
    let (done, next) = complete_ok(&db, &auth, id, "2026-11-02").await;
    assert_eq!(done["task"]["status"], "done");
    assert_eq!(next, Value::Null);
}

#[tokio::test]
async fn from_scheduled_copies_the_task_and_resets_it() {
    let (_dir, db) = DataDir::new("recur-scheduled");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "Home").await;
    let area = create_object(&db, &auth, "area", "Life").await;
    let parent = create_task(&db, &auth, "Chores", json!({})).await;
    let body = json!({
        "title": "Water plants", "tags": ["home"], "properties": { "room": "kitchen" },
        "task": {
            "priority": 2, "scheduled": "2026-11-01T09:30", "deadline": "2026-11-03T18:00",
            "estimate": 15, "someday": true, "parent_id": parent, "project_id": project,
            "area_id": area,
            "checklist": [{ "text": "ficus", "done": true }, { "text": "fern" }],
            "recurrence": { "rule": "FREQ=WEEKLY;BYDAY=SU", "from": "scheduled" }
        }
    });
    let res = call(&db, Method::POST, "/api/tasks", Some(&auth), Some(body)).await;
    let id = res.json()["id"].as_i64().unwrap();

    // Completed late: the next one still counts from the scheduled Sunday.
    let (done, next) = complete_ok(&db, &auth, id, "2026-11-05").await;
    assert_eq!(done["id"], id);
    assert_eq!(done["task"]["status"], "done");
    assert_eq!(done["task"]["recurrence"], Value::Null);
    assert_ne!(next["id"], id);
    assert_eq!(next["title"], "Water plants");
    assert_eq!(next["tags"], json!(["home"]));
    assert_eq!(next["properties"], json!({ "room": "kitchen" }));
    assert_eq!(
        next["task"],
        json!({
            "status": "open", "priority": 2, "scheduled": "2026-11-08T09:30",
            "deadline": "2026-11-10T18:00", "estimate": 15, "someday": false,
            "parent_id": parent, "project_id": project, "section_id": null, "area_id": area,
            "checklist": [{ "text": "ficus", "done": false }, { "text": "fern", "done": false }],
            "recurrence": { "rule": "FREQ=WEEKLY;BYDAY=SU", "from": "scheduled" },
            "completed": null
        })
    );
    let path = format!("/api/tasks/{}", next["id"]);
    let res = call(&db, Method::GET, &path, Some(&auth), None).await;
    assert_eq!(res.json(), next);
}

#[tokio::test]
async fn from_completion_counts_from_today_and_shifts_the_deadline() {
    let (_dir, db) = DataDir::new("recur-completion");
    let auth = token(&db).await;
    let task = json!({
        "scheduled": "2026-11-01", "deadline": "2026-11-03",
        "recurrence": { "rule": "FREQ=DAILY;INTERVAL=3", "from": "completion" }
    });
    let id = create_task(&db, &auth, "t", task).await;
    let (_, next) = complete_ok(&db, &auth, id, "2026-11-10").await;
    assert_eq!(next["task"]["scheduled"], "2026-11-13");
    assert_eq!(next["task"]["deadline"], "2026-11-15");
}

#[tokio::test]
async fn anchor_falls_back_to_deadline_then_today() {
    let (_dir, db) = DataDir::new("recur-anchor");
    let auth = token(&db).await;
    let task = json!({ "deadline": "2026-11-03T17:00", "recurrence": weekly("scheduled") });
    let id = create_task(&db, &auth, "t", task).await;
    let (_, next) = complete_ok(&db, &auth, id, "2026-11-20").await;
    assert_eq!(next["task"]["scheduled"], "2026-11-10");
    assert_eq!(next["task"]["deadline"], "2026-11-10T17:00");

    let id = create_task(
        &db,
        &auth,
        "t",
        json!({ "recurrence": weekly("scheduled") }),
    )
    .await;
    let (_, next) = complete_ok(&db, &auth, id, "2026-11-20").await;
    assert_eq!(next["task"]["scheduled"], "2026-11-27");
    assert_eq!(next["task"]["deadline"], Value::Null);

    let id = create_task(
        &db,
        &auth,
        "t",
        json!({ "recurrence": weekly("completion") }),
    )
    .await;
    let (_, next) = complete_ok(&db, &auth, id, "2026-12-30").await;
    assert_eq!(next["task"]["scheduled"], "2027-01-06");
}

#[tokio::test]
async fn reopening_and_recompleting_does_not_fork_the_series() {
    let (_dir, db) = DataDir::new("recur-reopen");
    let auth = token(&db).await;
    let task = json!({ "scheduled": "2026-11-01", "recurrence": weekly("scheduled") });
    let id = create_task(&db, &auth, "t", task).await;
    let (_, next) = complete_ok(&db, &auth, id, "2026-11-01").await;
    assert!(next.is_object());
    let (_, again) = complete_ok(&db, &auth, id, "2026-11-01").await;
    assert_eq!(again, Value::Null, "already done");
    let path = format!("/api/tasks/{id}/reopen");
    call(&db, Method::POST, &path, Some(&auth), None).await;
    let (_, again) = complete_ok(&db, &auth, id, "2026-11-01").await;
    assert_eq!(again, Value::Null, "reopened");
}

#[tokio::test]
async fn bad_today_is_400() {
    let (_dir, db) = DataDir::new("recur-today");
    let auth = token(&db).await;
    let id = create_task(
        &db,
        &auth,
        "t",
        json!({ "recurrence": weekly("completion") }),
    )
    .await;
    for today in ["2026-02-30", "2026-1-01", "2026-11-01T09:00", "x"] {
        let res = complete(&db, &auth, id, today).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{today}");
        assert_eq!(
            res.json(),
            json!({ "error": "today must be a date YYYY-MM-DD" })
        );
    }
    let res = call(
        &db,
        Method::GET,
        &format!("/api/tasks/{id}"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.json()["task"]["status"], "open");
}

#[tokio::test]
async fn next_past_year_9999_is_400_and_leaves_the_task_open() {
    let (_dir, db) = DataDir::new("recur-range");
    let auth = token(&db).await;
    let task = json!({
        "scheduled": "9999-12-30",
        "recurrence": { "rule": "FREQ=YEARLY", "from": "scheduled" }
    });
    let id = create_task(&db, &auth, "t", task).await;
    let res = complete(&db, &auth, id, "2026-11-01").await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        res.json(),
        json!({ "error": "invalid recurrence: next scheduled is out of range" })
    );
    let path = format!("/api/tasks/{id}");
    let res = call(&db, Method::GET, &path, Some(&auth), None).await;
    assert_eq!(res.json()["task"]["status"], "open");
}
