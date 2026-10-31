//! `GET /api/views/{id}/items`: each group kind, each sort, the Eisenhower quadrants and
//! truncation.

use axum::http::Method;
use serde_json::json;

use super::Ctx;
use super::common::create_object;

#[tokio::test]
async fn groups_by_every_kind() {
    let c = Ctx::new("views-groups").await;
    let make = |kind: &'static str, title: &'static str| create_object(&c.db, &c.auth, kind, title);
    let (beta, alpha, area) = (
        make("project", "Beta").await,
        make("project", "alpha").await,
        make("area", "Health").await,
    );
    c.call(
        Method::PATCH,
        &format!("/api/projects/{alpha}"),
        Some(json!({"area_id": area})),
    )
    .await;
    let res = c
        .call(
            Method::POST,
            &format!("/api/projects/{beta}/sections"),
            Some(json!({"title": "Plan"})),
        )
        .await;
    let sec = res.json()["id"].as_i64().unwrap();
    let t1 = c.task(json!({"title": "a", "tags": ["z", "home"], "task": {"priority": 1, "project_id": beta, "section_id": sec}})).await;
    let t2 = c
        .task(json!({"title": "b", "task": {"priority": 3, "project_id": alpha}}))
        .await;
    let t3 = c.task(json!({"title": "c", "tags": ["work"]})).await;
    c.call(Method::POST, &format!("/api/tasks/{t3}/complete"), None)
        .await;
    let note = make("note", "n").await;
    let q = "-type:project -type:area -type:view";
    let g = |group: &'static str| c.groups(q, "list", group, "created");

    assert_eq!(g("none").await, [json!([null, "all", [note, t3, t2, t1]])]);
    assert_eq!(
        g("status").await,
        [
            json!(["open", "open", [t2, t1]]),
            json!(["done", "done", [t3]]),
            json!([null, "none", [note]])
        ]
    );
    assert_eq!(
        g("priority").await,
        [
            json!([1, "P1", [t1]]),
            json!([3, "P3", [t2]]),
            json!([4, "P4", [t3]]),
            json!([null, "none", [note]])
        ]
    );
    assert_eq!(
        g("project").await,
        [
            json!([alpha, "alpha", [t2]]),
            json!([beta, "Beta", [t1]]),
            json!([null, "none", [note, t3]])
        ]
    );
    assert_eq!(
        g("section").await,
        [
            json!([sec, "Plan", [t1]]),
            json!([null, "none", [note, t3, t2]])
        ]
    );
    assert_eq!(
        g("area").await,
        [
            json!([area, "Health", [t2]]),
            json!([null, "none", [note, t3, t1]])
        ]
    );
    assert_eq!(
        g("tag").await,
        [
            json!(["home", "home", [t1]]),
            json!(["work", "work", [t3]]),
            json!([null, "none", [note, t2]])
        ]
    );
    assert_eq!(
        g("type").await,
        [
            json!(["note", "note", [note]]),
            json!(["task", "task", [t3, t2, t1]])
        ]
    );
    // A project's own area groups it; a trashed project counts as none.
    let all = c.groups("-type:view", "list", "area", "created").await;
    assert_eq!(all[0], json!([area, "Health", [t2, alpha]]));
    c.call(Method::DELETE, &format!("/api/objects/{alpha}"), None)
        .await;
    assert_eq!(g("project").await[0], json!([beta, "Beta", [t1]]));
    // Kanban without a group groups by status.
    let kanban = c.groups(q, "kanban", "none", "created").await;
    assert_eq!(kanban[0], json!(["open", "open", [t2, t1]]));
    // Items carry the task JSON.
    let body = c
        .items(c.view("type:task", "list", "none", "title").await)
        .await;
    assert_eq!(body["groups"][0]["items"][0]["task"]["priority"], 1);
    assert_eq!(
        (body["total"].clone(), body["truncated"].clone()),
        (json!(3), json!(false))
    );
    assert_eq!(body["view"]["properties"]["sort"], "title");
}

#[tokio::test]
async fn sorts_each_way() {
    let c = Ctx::new("views-sorts").await;
    let a = c.task(json!({"title": "beta", "task": {"priority": 2, "deadline": "2026-11-05", "scheduled": "2026-11-03T09:00"}})).await;
    let b = c
        .task(json!({"title": "Alpha", "task": {"priority": 1, "deadline": "2026-10-20"}}))
        .await;
    let d = c.task(json!({"title": "gamma", "task": {"priority": 2, "scheduled": "2026-11-02", "estimate": 10}})).await;
    let e = create_object(&c.db, &c.auth, "note", "delta").await;
    let f = c
        .task(json!({"title": "eps", "task": {"deadline": "2026-10-25"}}))
        .await;
    c.sql(&format!(
        "UPDATE objects SET created = 100, updated = 400 WHERE id = {a};
         UPDATE objects SET created = 300, updated = 100 WHERE id = {b};
         UPDATE objects SET created = 200, updated = 300 WHERE id = {d};
         UPDATE objects SET created = 200, updated = 200 WHERE id = {e};
         UPDATE tasks SET status = 'done', completed = 50 WHERE object_id = {a};
         UPDATE tasks SET status = 'done', completed = 70 WHERE object_id = {d};"
    ));
    let s = |sort: &'static str| c.sorted("-type:view", sort);
    // Smart: b overdue P1 (70), a P2 due in 4 days (46), f overdue P4 (40), d P2 with a
    // 10-minute estimate (24.6), the note 0. Ties and unset values go by id; f is newest.
    assert_eq!(s("smart").await, json!([b, a, f, d, e]));
    assert_eq!(s("priority").await, json!([b, a, d, f, e]));
    assert_eq!(s("deadline").await, json!([b, f, a, d, e]));
    assert_eq!(s("scheduled").await, json!([d, a, b, e, f]));
    assert_eq!(s("created").await, json!([f, b, e, d, a]));
    assert_eq!(s("updated").await, json!([f, a, d, e, b]));
    assert_eq!(s("completed").await, json!([d, a, f, e, b]));
    assert_eq!(s("title").await, json!([b, a, e, f, d]));
}

#[tokio::test]
async fn eisenhower_quadrants_at_the_boundaries() {
    let c = Ctx::new("views-eisenhower").await;
    let t = |p: i64, field: &str, date: &str| {
        let mut task = json!({"priority": p});
        if !field.is_empty() {
            task[field] = json!(date);
        }
        c.task(json!({"title": format!("{p} {field} {date}"), "task": task}))
    };
    let ui_overdue = t(1, "deadline", "2026-10-01").await;
    let ui_edge = t(2, "scheduled", "2026-11-03T23:00").await;
    let i_late = t(2, "deadline", "2026-11-04").await;
    let i_undated = t(1, "", "").await;
    let u_edge = t(3, "deadline", "2026-11-03").await;
    let u_past = t(4, "scheduled", "2026-10-30").await;
    let n_late = t(3, "scheduled", "2026-11-04").await;
    let note = create_object(&c.db, &c.auth, "note", "n").await;
    let groups = c.groups("-type:view", "eisenhower", "tag", "created").await;
    assert_eq!(
        groups,
        [
            json!([
                "urgent_important",
                "urgent & important",
                [ui_edge, ui_overdue]
            ]),
            json!(["important", "important", [i_undated, i_late]]),
            json!(["urgent", "urgent", [u_past, u_edge]]),
            json!(["neither", "neither", [note, n_late]]),
        ]
    );
    let empty = c.groups("type:habit", "eisenhower", "none", "smart").await;
    assert_eq!(empty.len(), 4);
    assert!(empty.iter().all(|g| g[2] == json!([])));
}

#[tokio::test]
async fn truncates_at_500() {
    let c = Ctx::new("views-truncate").await;
    c.sql(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 501)
           INSERT INTO objects (type, title) SELECT 'note', 'n' || i FROM n;",
    );
    let body = c
        .items(c.view("type:note", "list", "none", "created").await)
        .await;
    let items = body["groups"][0]["items"].as_array().unwrap();
    assert_eq!(
        (
            items.len(),
            body["total"].clone(),
            body["truncated"].clone()
        ),
        (500, json!(501), json!(true))
    );
    assert_eq!(items[0]["title"], "n501");
    c.sql("UPDATE objects SET deleted_at = 1 WHERE title = 'n1';");
    let body = c
        .items(c.view("type:note", "list", "none", "created").await)
        .await;
    assert_eq!(
        (body["total"].clone(), body["truncated"].clone()),
        (json!(500), json!(false))
    );
}
