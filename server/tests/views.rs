//! Saved views: `/api/views` validation and listing, and the errors of
//! `/api/views/{id}/items` (grouping, sorting and truncation in `views/items.rs`).

mod common;
#[path = "views/items.rs"]
mod items;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, token};
use polykov_server::db::Db;
use serde_json::{Value, json};

const TODAY: &str = "2026-11-01";

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

    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> Reply {
        call(&self.db, method, path, Some(&self.auth), body).await
    }

    /// Creates a task from a full `POST /api/tasks` body and returns its id.
    async fn task(&self, body: Value) -> i64 {
        let res = self.call(Method::POST, "/api/tasks", Some(body)).await;
        assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
        res.json()["id"].as_i64().unwrap()
    }

    async fn view(&self, query: &str, layout: &str, group: &str, sort: &str) -> i64 {
        let body =
            json!({"title": "v", "query": query, "layout": layout, "group": group, "sort": sort});
        let res = self.call(Method::POST, "/api/views", Some(body)).await;
        assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.json());
        res.json()["id"].as_i64().unwrap()
    }

    async fn items(&self, id: i64) -> Value {
        let path = format!("/api/views/{id}/items?today={TODAY}");
        let res = self.call(Method::GET, &path, None).await;
        assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
        res.json()
    }

    /// Each group's key, label and item ids, for a fresh view over `query`.
    async fn groups(&self, query: &str, layout: &str, group: &str, sort: &str) -> Vec<Value> {
        let id = self.view(query, layout, group, sort).await;
        let body = self.items(id).await;
        let groups = body["groups"].as_array().unwrap().iter();
        groups
            .map(|g| {
                let ids: Vec<Value> = g["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|o| o["id"].clone())
                    .collect();
                json!([g["key"], g["label"], ids])
            })
            .collect()
    }

    /// The item ids of a single-group view over `query` sorted by `sort`.
    async fn sorted(&self, query: &str, sort: &str) -> Value {
        self.groups(query, "list", "none", sort).await[0][2].clone()
    }

    fn sql(&self, sql: &str) {
        self.dir.sql().execute_batch(sql).unwrap();
    }
}

#[tokio::test]
async fn create_patch_and_list_validate_settings() {
    let c = Ctx::new("views-crud").await;
    let body = json!({"title": " Work ", "query": "type:task tag:work/*", "pinned": true});
    let res = c.call(Method::POST, "/api/views", Some(body)).await;
    assert_eq!(res.status, StatusCode::CREATED);
    let view = res.json();
    assert_eq!(view["type"], "view");
    assert_eq!(view["title"], "Work");
    assert_eq!(view["pinned"], true);
    let settings = json!({"query": "type:task tag:work/*", "layout": "list", "group": "none", "sort": "smart"});
    assert_eq!(view["properties"], settings);
    let id = view["id"].as_i64().unwrap();

    let bad = [
        (
            json!({"title": "x", "query": "type:task bogus:1"}),
            json!({"error": "query: unknown field \"bogus\"", "at": 10}),
        ),
        (
            json!({"title": "x", "query": "", "layout": "grid"}),
            json!({"error": "layout must be one of: list, kanban, eisenhower"}),
        ),
        (
            json!({"title": "x", "query": "", "group": "goal"}),
            json!({"error": "group must be one of: none, status, priority, project, section, area, tag, type"}),
        ),
        (
            json!({"title": "x", "query": "", "sort": "random"}),
            json!({"error": "sort must be one of: smart, priority, deadline, scheduled, created, updated, completed, title"}),
        ),
        (
            json!({"title": " ", "query": ""}),
            json!({"error": "title must be 1 to 1000 characters"}),
        ),
    ];
    for (body, error) in bad {
        let res = c.call(Method::POST, "/api/views", Some(body.clone())).await;
        assert_eq!(
            (res.status, res.json()),
            (StatusCode::BAD_REQUEST, error),
            "{body}"
        );
    }
    let res = c
        .call(Method::POST, "/api/views", Some(json!({"title": "x"})))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    let path = format!("/api/views/{id}");
    let res = c
        .call(
            Method::PATCH,
            &path,
            Some(json!({"group": "tag", "sort": "title"})),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json()["properties"]["group"], "tag");
    assert_eq!(res.json()["properties"]["query"], "type:task tag:work/*");
    for (body, status) in [
        (json!({"layout": "grid"}), StatusCode::BAD_REQUEST),
        (json!({"query": "due:<<1d"}), StatusCode::BAD_REQUEST),
        (json!({"sort": null}), StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let res = c.call(Method::PATCH, &path, Some(body.clone())).await;
        assert_eq!(res.status, status, "{body}");
    }
    let res = c
        .call(Method::PATCH, &path, Some(json!({"query": "due:<<1d"})))
        .await;
    assert!(res.json()["at"].is_u64());
    let note = create_object(&c.db, &c.auth, "note", "n").await;
    let res = c
        .call(
            Method::PATCH,
            &format!("/api/views/{note}"),
            Some(json!({})),
        )
        .await;
    assert_eq!(
        (res.status, res.json()),
        (StatusCode::NOT_FOUND, json!({"error": "view not found"}))
    );

    // Listing: pinned first, then by title ignoring case; trashed views are gone.
    let b = c.view("", "list", "none", "smart").await;
    c.call(
        Method::PATCH,
        &format!("/api/objects/{b}"),
        Some(json!({"title": "beta"})),
    )
    .await;
    let a = c.view("", "list", "none", "smart").await;
    c.call(
        Method::PATCH,
        &format!("/api/objects/{a}"),
        Some(json!({"title": "Alpha"})),
    )
    .await;
    let gone = c.view("", "list", "none", "smart").await;
    c.call(Method::DELETE, &format!("/api/objects/{gone}"), None)
        .await;
    let res = c.call(Method::GET, "/api/views", None).await;
    let ids: Vec<i64> = res.json()["views"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, [id, a, b]);
}

#[tokio::test]
async fn errors_404_422_and_401() {
    let c = Ctx::new("views-errors").await;
    let note = create_object(&c.db, &c.auth, "note", "n").await;
    let not_found = json!({"error": "view not found"});
    for path in [
        format!("/api/views/{note}/items"),
        "/api/views/999/items".into(),
    ] {
        let res = c.call(Method::GET, &path, None).await;
        assert_eq!(
            (res.status, res.json()),
            (StatusCode::NOT_FOUND, not_found.clone())
        );
    }
    let id = c.view("", "list", "none", "smart").await;
    let path = format!("/api/views/{id}/items");
    let cases = [
        (
            json!({"query": "", "layout": "grid", "group": "none", "sort": "smart"}),
            json!({"error": "layout must be one of: list, kanban, eisenhower"}),
        ),
        (
            json!({"query": "pinned:maybe", "layout": "list", "group": "none", "sort": "smart"}),
            json!({"error": "query: pinned must be true or false", "at": 7}),
        ),
        (
            json!({"layout": "list", "group": "none", "sort": "smart"}),
            json!({"error": "query must be a string"}),
        ),
    ];
    for (properties, error) in cases {
        let body = json!({"properties": properties});
        c.call(Method::PATCH, &format!("/api/objects/{id}"), Some(body))
            .await;
        let res = c.call(Method::GET, &path, None).await;
        assert_eq!(
            (res.status, res.json()),
            (StatusCode::UNPROCESSABLE_ENTITY, error)
        );
    }
    let res = c
        .call(Method::GET, &format!("{path}?today=11/01"), None)
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    for (method, path) in [
        (Method::GET, "/api/views"),
        (Method::POST, "/api/views"),
        (Method::PATCH, "/api/views/1"),
        (Method::GET, "/api/views/1/items"),
    ] {
        let res = call(&c.db, method, path, None, Some(json!({}))).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{path}");
    }
}
