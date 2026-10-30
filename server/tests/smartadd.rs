//! Smart add: `POST /api/parse` and `POST /api/tasks/add`.

mod common;

use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

const SPEC_LINE: &str = "call mom tmrw 5pm 30m p2 #family every sunday";
const TODAY: &str = "2026-10-30";

async fn post(db: &Db, auth: &str, path: &str, body: Value) -> Reply {
    call(db, Method::POST, path, Some(auth), Some(body)).await
}

async fn parse(db: &Db, auth: &str, text: &str) -> Value {
    let res = post(
        db,
        auth,
        "/api/parse",
        json!({ "text": text, "today": TODAY }),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    res.json()
}

async fn add(db: &Db, auth: &str, text: &str) -> Reply {
    post(
        db,
        auth,
        "/api/tasks/add",
        json!({ "text": text, "today": TODAY }),
    )
    .await
}

fn assert_error(res: &Reply, error: &str) {
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{:?}", res.json());
    assert_eq!(res.json(), json!({ "error": error }));
}

/// Ids of the tasks in the GTD list `name` as of [`TODAY`].
async fn list_ids(db: &Db, auth: &str, name: &str) -> Vec<Value> {
    let path = format!("/api/lists/{name}?today={TODAY}");
    let res = call(db, Method::GET, &path, Some(auth), None).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
    res.json()["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].clone())
        .collect()
}

fn task_count(dir: &DataDir) -> i64 {
    dir.sql()
        .query_row(
            "SELECT count(*) FROM objects WHERE type = 'task'",
            [],
            |r| r.get(0),
        )
        .unwrap()
}

#[tokio::test]
async fn spec_example_becomes_a_filled_in_task() {
    let (_dir, db) = DataDir::new("smartadd-spec");
    let auth = token(&db).await;
    let res = add(&db, &auth, SPEC_LINE).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    let task = res.json();
    assert_eq!(task["type"], "task");
    assert_eq!(task["title"], "call mom");
    assert_eq!(task["tags"], json!(["family"]));
    let fields = &task["task"];
    assert_eq!(fields["scheduled"], "2026-10-31T17:00");
    assert_eq!(fields["estimate"], 30);
    assert_eq!(fields["priority"], 2);
    assert_eq!(
        fields["recurrence"],
        json!({ "rule": "FREQ=WEEKLY;BYDAY=SU", "from": "scheduled" })
    );
    for empty in ["deadline", "project_id", "area_id"] {
        assert_eq!(fields[empty], Value::Null, "{empty}");
    }
    assert_eq!(list_ids(&db, &auth, "upcoming").await, [task["id"].clone()]);
    assert!(list_ids(&db, &auth, "inbox").await.is_empty());
}

#[tokio::test]
async fn parse_previews_the_spec_example() {
    let (_dir, db) = DataDir::new("smartadd-preview");
    let auth = token(&db).await;
    let preview = parse(&db, &auth, SPEC_LINE).await;
    assert_eq!(preview["title"], "call mom");
    assert_eq!(preview["tags"], json!(["family"]));
    assert_eq!(
        preview["task"],
        json!({
            "scheduled": "2026-10-31T17:00", "deadline": null, "estimate": 30, "priority": 2,
            "recurrence": { "rule": "FREQ=WEEKLY;BYDAY=SU", "from": "scheduled" },
            "project_id": null, "area_id": null
        })
    );
    let kinds: Vec<&str> = preview["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["date", "time", "duration", "priority", "tag", "recurrence"]
    );
    let first = &preview["tokens"][0];
    assert_eq!(
        first,
        &json!({ "start": 9, "end": 13, "kind": "date", "text": "tmrw", "resolved": true })
    );
    assert_eq!(task_count(&_dir), 0, "parse creates nothing");
}

#[tokio::test]
async fn a_plain_line_lands_in_the_inbox() {
    let (_dir, db) = DataDir::new("smartadd-inbox");
    let auth = token(&db).await;
    let res = add(&db, &auth, "  buy milk  ").await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    let task = res.json();
    assert_eq!(task["title"], "buy milk");
    assert_eq!(task["tags"], json!([]));
    let fields = &task["task"];
    assert_eq!(fields["priority"], 4);
    for empty in [
        "scheduled",
        "deadline",
        "project_id",
        "area_id",
        "parent_id",
        "recurrence",
    ] {
        assert_eq!(fields[empty], Value::Null, "{empty}");
    }
    assert_eq!(fields["someday"], false);
    assert_eq!(list_ids(&db, &auth, "inbox").await, [task["id"].clone()]);
}

#[tokio::test]
async fn projects_and_areas_resolve_to_live_ones() {
    let (_dir, db) = DataDir::new("smartadd-resolve");
    let auth = token(&db).await;
    let project = create_object(&db, &auth, "project", "Home Office").await;
    let area = create_object(&db, &auth, "area", "Health").await;
    let archived = create_object(&db, &auth, "project", "Old").await;
    let path = format!("/api/objects/{archived}");
    let res = call(
        &db,
        Method::PATCH,
        &path,
        Some(&auth),
        Some(json!({ "archived": true })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    let preview = parse(&db, &auth, "fix desk +home-office @HEALTH +old").await;
    assert_eq!(preview["title"], "fix desk");
    assert_eq!(preview["task"]["project_id"], project);
    assert_eq!(preview["task"]["area_id"], area);
    let resolved: Vec<(&str, bool)> = preview["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["text"].as_str().unwrap(),
                t["resolved"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        resolved,
        [("+home-office", true), ("@HEALTH", true), ("+old", false)]
    );

    let res = add(&db, &auth, "fix desk +home-office @health").await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
    assert_eq!(res.json()["task"]["project_id"], project);
    assert_eq!(res.json()["task"]["area_id"], area);
}

#[tokio::test]
async fn unknown_names_and_empty_titles_create_nothing() {
    let (dir, db) = DataDir::new("smartadd-errors");
    let auth = token(&db).await;
    create_object(&db, &auth, "project", "Home").await;
    let preview = parse(&db, &auth, "call +nowhere").await;
    assert_eq!(preview["tokens"][0]["resolved"], false);
    assert_eq!(preview["task"]["project_id"], Value::Null);

    assert_error(
        &add(&db, &auth, "call +nowhere").await,
        "unknown project: nowhere",
    );
    assert_error(
        &add(&db, &auth, "call +home @gym").await,
        "unknown area: gym",
    );
    assert_error(
        &add(&db, &auth, "tmrw 5pm #family").await,
        "title must be 1 to 1000 characters",
    );
    assert_error(
        &add(&db, &auth, "").await,
        "title must be 1 to 1000 characters",
    );
    assert_eq!(task_count(&dir), 0);
}

#[tokio::test]
async fn text_is_at_most_1000_characters() {
    let (dir, db) = DataDir::new("smartadd-long");
    let auth = token(&db).await;
    let long = "é".repeat(1001);
    for path in ["/api/parse", "/api/tasks/add"] {
        let res = post(&db, &auth, path, json!({ "text": long })).await;
        assert_error(&res, "text must be at most 1000 characters");
    }
    assert_eq!(task_count(&dir), 0);
    let res = add(&db, &auth, &"é".repeat(1000)).await;
    assert_eq!(res.status, StatusCode::CREATED);
}

#[tokio::test]
async fn bad_today_and_bodies_are_rejected() {
    let (_dir, db) = DataDir::new("smartadd-today");
    let auth = token(&db).await;
    for path in ["/api/parse", "/api/tasks/add"] {
        let res = post(
            &db,
            &auth,
            path,
            json!({ "text": "x", "today": "2026-02-30" }),
        )
        .await;
        assert_error(&res, "today must be a date YYYY-MM-DD");
        let res = post(&db, &auth, path, json!({ "text": "x", "extra": 1 })).await;
        assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn offsets_count_utf16_code_units() {
    let (_dir, db) = DataDir::new("smartadd-utf16");
    let auth = token(&db).await;
    let text = "😀 call #mom p1";
    let preview = parse(&db, &auth, text).await;
    let tokens = preview["tokens"].as_array().unwrap();
    // "😀" is 4 bytes but 2 UTF-16 units, so `#mom` starts at unit 8, not byte 10.
    assert_eq!(
        (&tokens[0]["start"], &tokens[0]["end"]),
        (&json!(8), &json!(12))
    );
    assert_eq!(
        (&tokens[1]["start"], &tokens[1]["end"]),
        (&json!(13), &json!(15))
    );
    let units: Vec<u16> = text.encode_utf16().collect();
    let slice = String::from_utf16(&units[8..12]).unwrap();
    assert_eq!(slice, tokens[0]["text"]);
}

#[tokio::test]
async fn every_route_needs_auth() {
    let (dir, db) = DataDir::new("smartadd-auth");
    token(&db).await;
    for path in ["/api/parse", "/api/tasks/add"] {
        let res = call(&db, Method::POST, path, None, Some(json!({ "text": "x" }))).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{path}");
    }
    assert_eq!(task_count(&dir), 0);
}

#[tokio::test]
async fn parse_answers_a_long_line_quickly() {
    let (_dir, db) = DataDir::new("smartadd-speed");
    let auth = token(&db).await;
    create_object(&db, &auth, "project", "Home").await;
    let mut line = String::from("call mom tmrw 5pm 30m p2 #family every sunday +home due nov 12");
    while line.len() < 200 {
        line.push_str(" and more");
    }
    let line = &line[..200];
    parse(&db, &auth, line).await;
    let start = Instant::now();
    parse(&db, &auth, line).await;
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "{:?}",
        start.elapsed()
    );
}
