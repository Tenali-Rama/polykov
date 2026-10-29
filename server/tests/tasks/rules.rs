//! The task field rules: values, parent, project, area and section.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use super::common::{DataDir, call, create_object, create_task, token};
use super::{add_section, assert_error, patch_task, post};

#[tokio::test]
async fn value_rules_are_400() {
    let (_dir, db) = DataDir::new("task-rules");
    let auth = token(&db).await;
    let id = create_task(&db, &auth, "t", json!({})).await;
    let date = "must be a date YYYY-MM-DD or a local time YYYY-MM-DDTHH:MM";
    let long = "x".repeat(501);
    let many: Vec<Value> = (0..101).map(|i| json!({ "text": i.to_string() })).collect();
    let cases = [
        (
            json!({ "priority": 0 }),
            "priority must be 1 to 4".to_owned(),
        ),
        (
            json!({ "priority": 5 }),
            "priority must be 1 to 4".to_owned(),
        ),
        (
            json!({ "scheduled": "2026-02-30" }),
            format!("scheduled {date}"),
        ),
        (
            json!({ "scheduled": "2026-11-01T09:00Z" }),
            format!("scheduled {date}"),
        ),
        (
            json!({ "deadline": "tomorrow" }),
            format!("deadline {date}"),
        ),
        (
            json!({ "estimate": 0 }),
            "estimate must be 1 to 10080 minutes".to_owned(),
        ),
        (
            json!({ "estimate": 10081 }),
            "estimate must be 1 to 10080 minutes".to_owned(),
        ),
        (
            json!({ "checklist": many }),
            "checklist must have at most 100 items".to_owned(),
        ),
        (
            json!({ "checklist": [{ "text": "  " }] }),
            "checklist text must be 1 to 500 characters".to_owned(),
        ),
        (
            json!({ "checklist": [{ "text": long }] }),
            "checklist text must be 1 to 500 characters".to_owned(),
        ),
    ];
    for (task, error) in cases {
        let res = patch_task(&db, &auth, id, task.clone()).await;
        assert_error(&res, StatusCode::BAD_REQUEST, &error);
        let res = post(
            &db,
            &auth,
            "/api/tasks",
            Some(json!({ "title": "n", "task": task })),
        )
        .await;
        assert_error(&res, StatusCode::BAD_REQUEST, &error);
    }
    let ok =
        json!({ "priority": 1, "estimate": 10080, "checklist": [{ "text": "x".repeat(500) }] });
    assert_eq!(patch_task(&db, &auth, id, ok).await.status, StatusCode::OK);
}

#[tokio::test]
async fn parent_must_be_a_live_task_and_not_a_descendant() {
    let (_dir, db) = DataDir::new("task-parent");
    let auth = token(&db).await;
    let a = create_task(&db, &auth, "a", json!({})).await;
    let b = create_task(&db, &auth, "b", json!({ "parent_id": a })).await;
    let c = create_task(&db, &auth, "c", json!({ "parent_id": b })).await;
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
    for bad in [note, gone, 999] {
        let res = patch_task(&db, &auth, a, json!({ "parent_id": bad })).await;
        assert_error(&res, StatusCode::BAD_REQUEST, "parent must be a live task");
    }
    for bad in [a, c] {
        let res = patch_task(&db, &auth, a, json!({ "parent_id": bad })).await;
        assert_error(
            &res,
            StatusCode::BAD_REQUEST,
            "a task cannot be its own ancestor",
        );
    }
    let res = patch_task(&db, &auth, c, json!({ "parent_id": a })).await;
    assert_eq!(res.json()["task"]["parent_id"], a);
    let res = patch_task(&db, &auth, c, json!({ "parent_id": null })).await;
    assert_eq!(res.json()["task"]["parent_id"], Value::Null);
}

#[tokio::test]
async fn project_and_area_must_be_live_and_of_their_type() {
    let (_dir, db) = DataDir::new("task-refs");
    let auth = token(&db).await;
    let id = create_task(&db, &auth, "t", json!({})).await;
    let note = create_object(&db, &auth, "note", "n").await;
    let area = create_object(&db, &auth, "area", "a").await;
    let res = patch_task(&db, &auth, id, json!({ "project_id": note })).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "project must be a live project",
    );
    let res = patch_task(&db, &auth, id, json!({ "project_id": area })).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "project must be a live project",
    );
    let res = patch_task(&db, &auth, id, json!({ "area_id": note })).await;
    assert_error(&res, StatusCode::BAD_REQUEST, "area must be a live area");
    let res = patch_task(&db, &auth, id, json!({ "area_id": area })).await;
    assert_eq!(res.json()["task"]["area_id"], area);
}

#[tokio::test]
async fn section_must_belong_to_the_project_and_clears_on_project_change() {
    let (dir, db) = DataDir::new("task-section");
    let auth = token(&db).await;
    let p1 = create_object(&db, &auth, "project", "p1").await;
    let p2 = create_object(&db, &auth, "project", "p2").await;
    let (s1, s2) = (add_section(&dir, p1), add_section(&dir, p2));
    let id = create_task(
        &db,
        &auth,
        "t",
        json!({ "project_id": p1, "section_id": s1 }),
    )
    .await;
    let msg = "section is not in this project";
    let res = patch_task(&db, &auth, id, json!({ "section_id": s2 })).await;
    assert_error(&res, StatusCode::BAD_REQUEST, msg);
    let res = patch_task(&db, &auth, id, json!({ "section_id": 999 })).await;
    assert_error(&res, StatusCode::BAD_REQUEST, msg);
    let res = patch_task(&db, &auth, id, json!({ "project_id": p1 })).await;
    assert_eq!(
        res.json()["task"]["section_id"],
        s1,
        "same project keeps the section"
    );
    let res = patch_task(&db, &auth, id, json!({ "project_id": p2 })).await;
    assert_eq!(res.json()["task"]["section_id"], Value::Null);
    let res = patch_task(
        &db,
        &auth,
        id,
        json!({ "project_id": p1, "section_id": s1 }),
    )
    .await;
    assert_eq!(res.json()["task"]["section_id"], s1);
    let res = patch_task(
        &db,
        &auth,
        id,
        json!({ "project_id": null, "section_id": s1 }),
    )
    .await;
    assert_error(&res, StatusCode::BAD_REQUEST, msg);
}
