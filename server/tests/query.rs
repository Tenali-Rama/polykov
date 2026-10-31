//! View queries: `GET /api/query` (each field, negation, text terms, errors, injection).

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

const TODAY: &str = "2026-11-01";
const NONE: [i64; 0] = [];

/// Percent-encodes every byte but ASCII letters and digits.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A fresh database with a logged-in owner.
struct Ctx {
    dir: DataDir,
    db: Db,
    auth: String,
}

impl Ctx {
    async fn new(name: &str) -> Self {
        let (dir, db) = DataDir::new(name);
        let auth = token(&db).await;
        Ctx { dir, db, auth }
    }

    async fn get(&self, query: &str) -> Reply {
        let path = format!("/api/query?{query}");
        call(&self.db, Method::GET, &path, Some(&self.auth), None).await
    }

    /// The ids matching `q` on `TODAY`, in the order served.
    async fn ordered(&self, q: &str) -> Vec<i64> {
        let res = self.get(&format!("today={TODAY}&q={}", encode(q))).await;
        assert_eq!(res.status, StatusCode::OK, "{q}: {:?}", res.json());
        let body = res.json();
        let ids: Vec<i64> = body["objects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["id"].as_i64().unwrap())
            .collect();
        assert_eq!(body["total"], ids.len(), "{q}");
        ids
    }

    /// The ids matching `q` on `TODAY`, sorted.
    async fn ids(&self, q: &str) -> Vec<i64> {
        let mut ids = self.ordered(q).await;
        ids.sort();
        ids
    }

    /// Creates a task from a full `POST /api/tasks` body and returns its id.
    async fn task(&self, body: Value) -> i64 {
        let res = call(
            &self.db,
            Method::POST,
            "/api/tasks",
            Some(&self.auth),
            Some(body),
        )
        .await;
        assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
        res.json()["id"].as_i64().unwrap()
    }

    async fn object(&self, kind: &str, title: &str) -> i64 {
        create_object(&self.db, &self.auth, kind, title).await
    }

    async fn send(&self, method: Method, path: &str, body: Option<Value>) {
        let res = call(&self.db, method, path, Some(&self.auth), body).await;
        assert!(res.status.is_success(), "{path}: {:?}", res.json());
    }

    async fn patch(&self, path: &str, body: Value) {
        self.send(Method::PATCH, path, Some(body)).await;
    }

    async fn complete(&self, id: i64) {
        let path = format!("/api/tasks/{id}/complete?today={TODAY}");
        self.send(Method::POST, &path, None).await;
    }
}

fn assert_error(res: &Reply, error: Value) {
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{:?}", res.json());
    assert_eq!(res.json(), error);
}

#[tokio::test]
async fn spec_example_returns_exactly_the_expected_tasks() {
    let c = Ctx::new("query-spec").await;
    let t = |tags: Value, priority: u8, deadline: Value| {
        json!({ "title": "t", "tags": tags,
                "task": { "priority": priority, "deadline": deadline } })
    };
    let work = || json!(["work"]);
    let a = c.task(t(json!(["work/a"]), 1, json!("2026-11-03"))).await;
    let b = c.task(t(work(), 2, json!("2026-11-07T10:00"))).await;
    c.task(t(work(), 3, json!("2026-11-03"))).await;
    c.task(t(work(), 1, json!("2026-11-08"))).await;
    c.task(t(json!(["workshop"]), 1, json!("2026-11-03"))).await;
    c.task(t(work(), 1, Value::Null)).await;
    let done = c.task(t(work(), 1, json!("2026-11-03"))).await;
    c.complete(done).await;
    let note = c.object("note", "note").await;
    c.patch(&format!("/api/objects/{note}"), json!({ "tags": ["work"] }))
        .await;

    let q = "type:task priority:<=2 due:<7d tag:work/* -status:done";
    assert_eq!(c.ids(q).await, [a, b]);
}

#[tokio::test]
async fn type_tag_status_priority_and_pinned() {
    let c = Ctx::new("query-fields").await;
    let p1 = c
        .task(json!({ "title": "p1", "tags": ["work"], "pinned": true,
                      "task": { "priority": 1 } }))
        .await;
    let p3 = c
        .task(json!({ "title": "p3", "tags": ["work/x"], "task": { "priority": 3 } }))
        .await;
    let done = c.task(json!({ "title": "d", "task": {} })).await;
    c.complete(done).await;
    let note = c.object("note", "n").await;

    assert_eq!(c.ids("type:note").await, [note]);
    assert_eq!(c.ids("type:note,task").await, [p1, p3, done, note]);
    assert_eq!(c.ids("-type:task").await, [note]);
    assert_eq!(c.ids("tag:work").await, [p1]);
    assert_eq!(c.ids("tag:work/*").await, [p1, p3]);
    assert_eq!(c.ids("tag:work/x").await, [p3]);
    assert_eq!(c.ids("-tag:work/*").await, [done, note]);
    assert_eq!(c.ids("status:done").await, [done]);
    assert_eq!(c.ids("status:open").await, [p1, p3]);
    assert_eq!(c.ids("-status:open").await, [done, note]);
    assert_eq!(c.ids("priority:1").await, [p1]);
    assert_eq!(c.ids("priority:>=3").await, [p3, done]);
    assert_eq!(c.ids("priority:<3,4").await, [p1, done]);
    assert_eq!(c.ids("priority:>4").await, NONE);
    assert_eq!(c.ids("-priority:1").await, [p3, done, note]);
    assert_eq!(c.ids("pinned:true").await, [p1]);
    assert_eq!(c.ids("pinned:false").await, [p3, done, note]);
    // Pinned first, then the rest newest first (same second: highest id first).
    assert_eq!(c.ordered("").await, [p1, note, done, p3]);
}

#[tokio::test]
async fn date_fields_and_none() {
    let c = Ctx::new("query-dates").await;
    let due = c
        .task(json!({ "title": "due", "task": {
            "deadline": "2026-11-01T09:00", "scheduled": "2026-10-30" } }))
        .await;
    let later = c
        .task(json!({ "title": "later", "task": {
            "deadline": "2026-11-15", "scheduled": "2026-11-02" } }))
        .await;
    let undated = c.task(json!({ "title": "undated", "task": {} })).await;
    c.complete(undated).await;
    let note = c.object("note", "n").await;
    let sql = c.dir.sql();
    let run = |stmt: &str, id: i64| sql.execute(stmt, [id]).unwrap();
    run(
        "UPDATE objects SET created = unixepoch('2026-10-01'), \
         updated = unixepoch('2026-10-20') WHERE id <> ?1",
        note,
    );
    run(
        "UPDATE objects SET created = unixepoch('2026-10-31 23:59'), \
         updated = unixepoch('2026-11-01 08:00') WHERE id = ?1",
        note,
    );
    run(
        "UPDATE tasks SET completed = unixepoch('2026-10-29 12:00') WHERE object_id = ?1",
        undated,
    );

    assert_eq!(c.ids("due:today").await, [due]);
    assert_eq!(c.ids("due:>today").await, [later]);
    assert_eq!(c.ids("due:2026-11-15").await, [later]);
    assert_eq!(c.ids("due:<=2w").await, [due, later]);
    assert_eq!(c.ids("due:none").await, [undated]);
    assert_eq!(c.ids("due:none,today").await, [due, undated]);
    assert_eq!(c.ids("-due:none").await, [due, later, note]);
    assert_eq!(c.ids("scheduled:<today").await, [due]);
    assert_eq!(c.ids("scheduled:tomorrow").await, [later]);
    assert_eq!(c.ids("completed:>=-3d").await, [undated]);
    assert_eq!(c.ids("completed:yesterday").await, NONE);
    assert_eq!(c.ids("created:yesterday").await, [note]);
    assert_eq!(c.ids("created:<-1m").await, NONE);
    assert_eq!(c.ids("created:<=-1m").await, [due, later, undated]);
    assert_eq!(c.ids("updated:today").await, [note]);
    assert_eq!(c.ids("-updated:today").await, [due, later, undated]);
    assert_eq!(c.ids("updated:none").await, NONE);
}

#[tokio::test]
async fn project_area_and_archived() {
    let c = Ctx::new("query-refs").await;
    let area = c.object("area", "Health").await;
    let project = c.object("project", "Run a 10k").await;
    let other = c.object("project", "Other").await;
    c.patch(
        &format!("/api/projects/{project}"),
        json!({ "area_id": area }),
    )
    .await;
    let t = |task: Value| json!({ "title": "t", "task": task });
    let in_project = c.task(t(json!({ "project_id": project }))).await;
    let in_area = c.task(t(json!({ "area_id": area }))).await;
    let in_other = c.task(t(json!({ "project_id": other }))).await;
    let archived = c.task(t(json!({ "project_id": project }))).await;
    c.patch(
        &format!("/api/objects/{archived}"),
        json!({ "archived": true }),
    )
    .await;

    assert_eq!(c.ids("project:\"Run a 10k\"").await, [in_project]);
    assert_eq!(c.ids("project:\"run A 10K\"").await, [in_project]);
    assert_eq!(c.ids(&format!("project:{project}")).await, [in_project]);
    assert_eq!(c.ids(&format!("project:{other}")).await, [in_other]);
    assert_eq!(c.ids(&format!("project:{area}")).await, NONE);
    assert_eq!(c.ids("project:\"Nope\"").await, NONE);
    let q = format!("type:task -project:{project}");
    assert_eq!(c.ids(&q).await, [in_area, in_other]);
    assert_eq!(
        c.ids("area:\"Health\"").await,
        [project, in_project, in_area]
    );
    assert_eq!(c.ids(&format!("area:{other}")).await, NONE);
    let all = [area, project, other, in_project, in_area, in_other];
    assert_eq!(c.ids("").await, all);
    assert_eq!(c.ids("archived:true").await, [archived]);
    let mut any = all.to_vec();
    any.push(archived);
    assert_eq!(c.ids("archived:any").await, any);
    let q = format!("archived:any project:{project}");
    assert_eq!(c.ids(&q).await, [in_project, archived]);
}

#[tokio::test]
async fn text_terms_filter_through_search() {
    let c = Ctx::new("query-text").await;
    let groceries = c
        .task(json!({ "title": "Buy groceries", "tags": ["home"], "task": {} }))
        .await;
    let list = c.object("note", "Groceries list").await;
    let other = c.object("note", "Meeting notes").await;

    assert_eq!(c.ids("groceries").await, [groceries, list]);
    assert_eq!(c.ids("groceris type:note").await, [list]);
    assert_eq!(c.ids("groceries tag:home").await, [groceries]);
    assert_eq!(c.ids("-groceries").await, [other]);
    assert_eq!(c.ids("\"groceries list\"").await, [list]);
    assert_eq!(c.ids("zzzzzz").await, NONE);
}

#[tokio::test]
async fn injection_shaped_values_are_plain_values() {
    let c = Ctx::new("query-injection").await;
    let note = c.object("note", "title'-- here").await;
    c.object("note", "Other").await;
    assert_eq!(c.ids("title'--").await, [note]);
    assert_eq!(c.ids("project:\"x' OR 1=1 --\"").await, NONE);
    assert_eq!(c.ids("tag:a'--").await, NONE);
}

#[tokio::test]
async fn tasks_carry_task_fields_and_pages_report_the_total() {
    let c = Ctx::new("query-shape").await;
    let first = c
        .task(json!({ "title": "a", "task": { "priority": 2 } }))
        .await;
    let note = c.object("note", "n").await;
    let res = c.get("q=&limit=1&offset=1").await;
    assert_eq!(res.status, StatusCode::OK);
    let body = res.json();
    assert_eq!(body["total"], 2);
    assert_eq!(body["objects"][0]["id"], first);
    assert_eq!(body["objects"][0]["task"]["priority"], 2);
    assert_eq!(body["objects"][0]["task"]["status"], "open");
    let res = c.get("limit=1").await.json();
    assert_eq!(res["objects"][0]["id"], note);
    assert_eq!(res["objects"][0]["type"], "note");
    assert!(res["objects"][0].get("task").is_none());
}

#[tokio::test]
async fn bad_requests_are_rejected() {
    let c = Ctx::new("query-errors").await;
    let res = call(&c.db, Method::GET, "/api/query?q=", None, None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    let bad = |q: &str| format!("q={}", encode(q));
    let res = c.get(&bad("type:task colour:red")).await;
    assert_error(
        &res,
        json!({ "error": "unknown field \"colour\"", "at": 10 }),
    );
    let res = c.get(&bad("priority:5")).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.json()["at"], 9);
    let res = c.get(&bad("due:<99999999m")).await;
    assert_error(&res, json!({ "error": "date is out of range" }));

    let cases = [
        ("today=2026-13-01", "today must be a date YYYY-MM-DD"),
        ("today=2026-1-01", "today must be a date YYYY-MM-DD"),
        ("limit=0", "limit must be 1 to 1000"),
        ("limit=1001", "limit must be 1 to 1000"),
        ("offset=-1", "offset must be a non-negative integer"),
        ("offset=x", "offset must be a non-negative integer"),
        (
            &format!("q={}", "a".repeat(1001)),
            "q must be at most 1000 characters",
        ),
    ];
    for (query, error) in cases {
        assert_error(&c.get(query).await, json!({ "error": error }));
    }
    let res = c.get("sort=smart").await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}
