//! Projects: `GET /api/projects`, `GET`/`PATCH /api/projects/{id}` and the section routes
//! (section behaviour in `projects/sections.rs`).

mod common;
#[path = "projects/sections.rs"]
mod sections;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, create_task, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

async fn get(db: &Db, auth: &str, path: &str) -> Reply {
    call(db, Method::GET, path, Some(auth), None).await
}

async fn patch(db: &Db, auth: &str, path: &str, body: Value) -> Reply {
    call(db, Method::PATCH, path, Some(auth), Some(body)).await
}

async fn post(db: &Db, auth: &str, path: &str, body: Value) -> Reply {
    call(db, Method::POST, path, Some(auth), Some(body)).await
}

fn assert_error(res: &Reply, status: StatusCode, error: &str) {
    assert_eq!(res.status, status, "{:?}", res.json());
    assert_eq!(res.json(), json!({ "error": error }));
}

/// Titles of the projects listed at `query`.
async fn titles(db: &Db, auth: &str, query: &str) -> Vec<String> {
    let res = get(db, auth, &format!("/api/projects{query}")).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    res.json()["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["title"].as_str().unwrap().to_owned())
        .collect()
}

async fn add_section(db: &Db, auth: &str, project: i64, body: Value) -> Reply {
    post(db, auth, &format!("/api/projects/{project}/sections"), body).await
}

#[tokio::test]
async fn a_project_without_a_row_reads_with_defaults() {
    let (dir, db) = DataDir::new("project-defaults");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "Launch").await;
    let res = get(&db, &auth, &format!("/api/projects/{id}")).await;
    assert_eq!(res.status, StatusCode::OK);
    let p = res.json();
    assert_eq!(p["type"], "project");
    assert_eq!(p["title"], "Launch");
    assert_eq!(
        p["project"],
        json!({ "status": "active", "area_id": null, "start": null, "deadline": null,
                "open_tasks": 0, "done_tasks": 0 })
    );
    let rows: i64 = dir
        .sql()
        .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn counts_cover_live_open_and_done_tasks() {
    let (_dir, db) = DataDir::new("project-counts");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "P").await;
    let other = create_object(&db, &auth, "project", "Q").await;
    for title in ["a", "b", "c"] {
        create_task(&db, &auth, title, json!({ "project_id": id })).await;
    }
    let done = create_task(&db, &auth, "d", json!({ "project_id": id })).await;
    post(
        &db,
        &auth,
        &format!("/api/tasks/{done}/complete"),
        json!({}),
    )
    .await;
    let trashed = create_task(&db, &auth, "e", json!({ "project_id": id })).await;
    let path = format!("/api/objects/{trashed}");
    call(&db, Method::DELETE, &path, Some(&auth), None).await;
    create_task(&db, &auth, "f", json!({ "project_id": other })).await;
    let p = get(&db, &auth, &format!("/api/projects/{id}")).await.json();
    assert_eq!(p["project"]["open_tasks"], 3);
    assert_eq!(p["project"]["done_tasks"], 1);
}

#[tokio::test]
async fn list_filters_by_status_and_area_and_orders_by_title() {
    let (_dir, db) = DataDir::new("project-list");
    let auth = token(&db).await;
    let area = create_object(&db, &auth, "area", "Work").await;
    let b = create_object(&db, &auth, "project", "beta").await;
    create_object(&db, &auth, "project", "Alpha").await;
    let s = create_object(&db, &auth, "project", "Someday thing").await;
    let d = create_object(&db, &auth, "project", "Done thing").await;
    let archived = create_object(&db, &auth, "project", "Archived").await;
    let trashed = create_object(&db, &auth, "project", "Trashed").await;
    create_object(&db, &auth, "note", "Not a project").await;
    patch(
        &db,
        &auth,
        &format!("/api/projects/{s}"),
        json!({ "status": "someday" }),
    )
    .await;
    patch(
        &db,
        &auth,
        &format!("/api/projects/{d}"),
        json!({ "status": "done", "area_id": area }),
    )
    .await;
    patch(
        &db,
        &auth,
        &format!("/api/projects/{b}"),
        json!({ "area_id": area }),
    )
    .await;
    patch(
        &db,
        &auth,
        &format!("/api/objects/{archived}"),
        json!({ "archived": true }),
    )
    .await;
    call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{trashed}"),
        Some(&auth),
        None,
    )
    .await;

    assert_eq!(titles(&db, &auth, "").await, ["Alpha", "beta"]);
    assert_eq!(
        titles(&db, &auth, "?status=active").await,
        ["Alpha", "beta"]
    );
    assert_eq!(
        titles(&db, &auth, "?status=someday").await,
        ["Someday thing"]
    );
    assert_eq!(titles(&db, &auth, "?status=done").await, ["Done thing"]);
    assert_eq!(
        titles(&db, &auth, "?status=any").await,
        ["Alpha", "beta", "Done thing", "Someday thing"]
    );
    assert_eq!(
        titles(&db, &auth, &format!("?status=any&area={area}")).await,
        ["beta", "Done thing"]
    );
    assert_eq!(titles(&db, &auth, &format!("?area={area}")).await, ["beta"]);

    let res = get(&db, &auth, "/api/projects?status=open").await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "status must be active, someday, done or any",
    );
    let res = get(&db, &auth, "/api/projects?area=x").await;
    assert_error(&res, StatusCode::BAD_REQUEST, "area must be an integer id");
}

#[tokio::test]
async fn patch_sets_and_clears_fields_and_bumps_updated() {
    let (dir, db) = DataDir::new("project-patch");
    let auth = token(&db).await;
    let area = create_object(&db, &auth, "area", "Health").await;
    let id = create_object(&db, &auth, "project", "Run").await;
    dir.sql()
        .execute("UPDATE objects SET updated = 0 WHERE id = ?1", [id])
        .unwrap();
    let path = format!("/api/projects/{id}");
    let body = json!({ "status": "someday", "area_id": area, "start": "2026-11-01",
                       "deadline": "2026-12-01T17:00" });
    let res = patch(&db, &auth, &path, body).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    let p = res.json();
    assert_eq!(p["project"]["status"], "someday");
    assert_eq!(p["project"]["area_id"], area);
    assert_eq!(p["project"]["start"], "2026-11-01");
    assert_eq!(p["project"]["deadline"], "2026-12-01T17:00");
    assert_ne!(p["updated"], "1970-01-01T00:00:00Z");
    assert_eq!(get(&db, &auth, &path).await.json(), p);

    let res = patch(&db, &auth, &path, json!({ "area_id": null, "start": null })).await;
    let p = res.json();
    assert_eq!(p["project"]["area_id"], Value::Null);
    assert_eq!(p["project"]["start"], Value::Null);
    assert_eq!(p["project"]["status"], "someday");
    assert_eq!(p["project"]["deadline"], "2026-12-01T17:00");

    dir.sql()
        .execute("UPDATE objects SET updated = 0 WHERE id = ?1", [id])
        .unwrap();
    let res = patch(&db, &auth, &path, json!({})).await;
    assert_eq!(res.json()["updated"], "1970-01-01T00:00:00Z");
}

#[tokio::test]
async fn patch_rejects_bad_fields() {
    let (_dir, db) = DataDir::new("project-patch-rules");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "P").await;
    let note = create_object(&db, &auth, "note", "N").await;
    let area = create_object(&db, &auth, "area", "A").await;
    call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{area}"),
        Some(&auth),
        None,
    )
    .await;
    let path = format!("/api/projects/{id}");
    for (body, error) in [
        (
            json!({ "status": "open" }),
            "status must be active, someday or done",
        ),
        (json!({ "area_id": note }), "area must be a live area"),
        (json!({ "area_id": area }), "area must be a live area"),
        (
            json!({ "start": "2026-02-30" }),
            "start must be a date YYYY-MM-DD or a local time YYYY-MM-DDTHH:MM",
        ),
        (
            json!({ "deadline": "tomorrow" }),
            "deadline must be a date YYYY-MM-DD or a local time YYYY-MM-DDTHH:MM",
        ),
    ] {
        assert_error(
            &patch(&db, &auth, &path, body).await,
            StatusCode::BAD_REQUEST,
            error,
        );
    }
    let res = patch(&db, &auth, &path, json!({ "status": null })).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = patch(&db, &auth, &path, json!({ "title": "x" })).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let p = get(&db, &auth, &path).await.json();
    assert_eq!(p["project"]["status"], "active");
}

#[tokio::test]
async fn non_projects_and_trashed_ids_are_404() {
    let (_dir, db) = DataDir::new("project-404");
    let auth = token(&db).await;
    let note = create_object(&db, &auth, "note", "N").await;
    let trashed = create_object(&db, &auth, "project", "T").await;
    call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{trashed}"),
        Some(&auth),
        None,
    )
    .await;
    for id in [note, trashed, 999] {
        let path = format!("/api/projects/{id}");
        assert_error(
            &get(&db, &auth, &path).await,
            StatusCode::NOT_FOUND,
            "project not found",
        );
        let res = patch(&db, &auth, &path, json!({ "status": "done" })).await;
        assert_error(&res, StatusCode::NOT_FOUND, "project not found");
        let sections = format!("{path}/sections");
        let res = get(&db, &auth, &sections).await;
        assert_error(&res, StatusCode::NOT_FOUND, "project not found");
        let res = post(&db, &auth, &sections, json!({ "title": "s" })).await;
        assert_error(&res, StatusCode::NOT_FOUND, "project not found");
    }
}

#[tokio::test]
async fn every_route_requires_auth() {
    let (_dir, db) = DataDir::new("project-401");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "P").await;
    let sid = add_section(&db, &auth, id, json!({ "title": "s" }))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    for (method, path, body) in [
        (Method::GET, "/api/projects".to_owned(), None),
        (Method::GET, format!("/api/projects/{id}"), None),
        (
            Method::PATCH,
            format!("/api/projects/{id}"),
            Some(json!({})),
        ),
        (Method::GET, format!("/api/projects/{id}/sections"), None),
        (
            Method::POST,
            format!("/api/projects/{id}/sections"),
            Some(json!({ "title": "x" })),
        ),
        (
            Method::PATCH,
            format!("/api/sections/{sid}"),
            Some(json!({})),
        ),
        (Method::DELETE, format!("/api/sections/{sid}"), None),
    ] {
        let res = call(&db, method, &path, None, body).await;
        assert_error(&res, StatusCode::UNAUTHORIZED, "authentication required");
    }
    let res = get(&db, &auth, &format!("/api/projects/{id}/sections")).await;
    assert_eq!(res.json()["sections"].as_array().unwrap().len(), 1);
}
