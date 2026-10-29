//! Project sections: add, move, rename and delete with dense positions, and their 404s.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use super::common::{DataDir, call, create_object, create_task, token};
use super::{add_section, assert_error, get, patch};
use polykov_server::db::Db;

/// `(title, position)` of the project's sections, as listed.
async fn sections(db: &Db, auth: &str, project: i64) -> Vec<(String, i64)> {
    let res = get(db, auth, &format!("/api/projects/{project}/sections")).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    res.json()["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            assert_eq!(s["project_id"], project);
            (
                s["title"].as_str().unwrap().to_owned(),
                s["position"].as_i64().unwrap(),
            )
        })
        .collect()
}

fn names(list: &[(String, i64)]) -> Vec<&str> {
    list.iter().map(|(t, _)| t.as_str()).collect()
}

fn assert_dense(list: &[(String, i64)]) {
    let positions: Vec<i64> = list.iter().map(|&(_, p)| p).collect();
    assert_eq!(positions, (0..list.len() as i64).collect::<Vec<_>>());
}

#[tokio::test]
async fn sections_append_insert_and_validate() {
    let (_dir, db) = DataDir::new("section-add");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "P").await;
    assert!(sections(&db, &auth, id).await.is_empty());
    let res = add_section(&db, &auth, id, json!({ "title": "  Plan  " })).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    let s = res.json();
    assert_eq!(s["title"], "Plan");
    assert_eq!(s["position"], 0);
    assert_eq!(s["project_id"], id);
    add_section(&db, &auth, id, json!({ "title": "Ship" })).await;
    let res = add_section(&db, &auth, id, json!({ "title": "Build", "position": 1 })).await;
    assert_eq!(res.json()["position"], 1);
    let res = add_section(&db, &auth, id, json!({ "title": "Idea", "position": 0 })).await;
    assert_eq!(res.json()["position"], 0);
    let res = add_section(&db, &auth, id, json!({ "title": "Retro", "position": 99 })).await;
    assert_eq!(res.json()["position"], 4);
    let list = sections(&db, &auth, id).await;
    assert_eq!(names(&list), ["Idea", "Plan", "Build", "Ship", "Retro"]);
    assert_dense(&list);

    let long = "x".repeat(201);
    for body in [json!({ "title": "   " }), json!({ "title": long })] {
        let res = add_section(&db, &auth, id, body).await;
        assert_error(
            &res,
            StatusCode::BAD_REQUEST,
            "title must be 1 to 200 characters",
        );
    }
    let ok = add_section(&db, &auth, id, json!({ "title": "x".repeat(200) })).await;
    assert_eq!(ok.status, StatusCode::CREATED);
    let res = add_section(&db, &auth, id, json!({ "title": "s", "position": -1 })).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "position must be a non-negative integer",
    );
}

#[tokio::test]
async fn sections_rename_and_move_keep_positions_dense() {
    let (_dir, db) = DataDir::new("section-move");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "P").await;
    let mut ids = Vec::new();
    for title in ["a", "b", "c", "d"] {
        let res = add_section(&db, &auth, id, json!({ "title": title })).await;
        ids.push(res.json()["id"].as_i64().unwrap());
    }
    let path = |sid: i64| format!("/api/sections/{sid}");
    let res = patch(&db, &auth, &path(ids[0]), json!({ "position": 2 })).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    assert_eq!(res.json()["position"], 2);
    assert_eq!(names(&sections(&db, &auth, id).await), ["b", "c", "a", "d"]);
    patch(&db, &auth, &path(ids[3]), json!({ "position": 0 })).await;
    assert_eq!(names(&sections(&db, &auth, id).await), ["d", "b", "c", "a"]);
    let res = patch(&db, &auth, &path(ids[3]), json!({ "position": 50 })).await;
    assert_eq!(res.json()["position"], 3);
    let res = patch(
        &db,
        &auth,
        &path(ids[1]),
        json!({ "title": " B2 ", "position": 0 }),
    )
    .await;
    assert_eq!(res.json()["title"], "B2");
    assert_eq!(res.json()["position"], 0);
    let res = patch(&db, &auth, &path(ids[2]), json!({ "title": "C2" })).await;
    assert_eq!(res.json()["position"], 1);
    let list = sections(&db, &auth, id).await;
    assert_eq!(names(&list), ["B2", "C2", "a", "d"]);
    assert_dense(&list);

    let res = patch(&db, &auth, &path(ids[0]), json!({ "title": "" })).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "title must be 1 to 200 characters",
    );
    let res = patch(&db, &auth, &path(ids[0]), json!({ "position": -2 })).await;
    assert_error(
        &res,
        StatusCode::BAD_REQUEST,
        "position must be a non-negative integer",
    );
}

#[tokio::test]
async fn deleting_a_section_keeps_its_tasks_in_the_project() {
    let (_dir, db) = DataDir::new("section-delete");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "P").await;
    let mut ids = Vec::new();
    for title in ["a", "b", "c"] {
        let res = add_section(&db, &auth, id, json!({ "title": title })).await;
        ids.push(res.json()["id"].as_i64().unwrap());
    }
    let task = create_task(
        &db,
        &auth,
        "t",
        json!({ "project_id": id, "section_id": ids[0] }),
    )
    .await;
    let path = format!("/api/sections/{}", ids[0]);
    let res = call(&db, Method::DELETE, &path, Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    let list = sections(&db, &auth, id).await;
    assert_eq!(names(&list), ["b", "c"]);
    assert_dense(&list);
    let t = get(&db, &auth, &format!("/api/tasks/{task}")).await.json();
    assert_eq!(t["task"]["project_id"], id);
    assert_eq!(t["task"]["section_id"], Value::Null);

    let res = call(&db, Method::DELETE, &path, Some(&auth), None).await;
    assert_error(&res, StatusCode::NOT_FOUND, "section not found");
    let res = patch(&db, &auth, &path, json!({ "title": "x" })).await;
    assert_error(&res, StatusCode::NOT_FOUND, "section not found");
}

#[tokio::test]
async fn sections_of_a_trashed_project_are_404() {
    let (_dir, db) = DataDir::new("section-trashed");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "project", "P").await;
    let sid = add_section(&db, &auth, id, json!({ "title": "s" }))
        .await
        .json()["id"]
        .as_i64()
        .unwrap();
    call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{id}"),
        Some(&auth),
        None,
    )
    .await;
    let path = format!("/api/sections/{sid}");
    let res = patch(&db, &auth, &path, json!({ "title": "x" })).await;
    assert_error(&res, StatusCode::NOT_FOUND, "section not found");
    let res = call(&db, Method::DELETE, &path, Some(&auth), None).await;
    assert_error(&res, StatusCode::NOT_FOUND, "section not found");
}
