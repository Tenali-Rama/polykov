//! Global search: `GET /api/search`, the index triggers and the migration backfill.

mod common;

use axum::http::{Method, StatusCode};
use common::{DataDir, Reply, call, create_object, create_task, token};
use polykov_server::db::{self, Db};
use rusqlite::Connection;
use serde_json::{Value, json};

async fn search(db: &Db, auth: &str, query: &str) -> Reply {
    call(
        db,
        Method::GET,
        &format!("/api/search?{query}"),
        Some(auth),
        None,
    )
    .await
}

/// The ids of the results for `q` (URL-encoded by the caller), in order.
async fn ids(db: &Db, auth: &str, q: &str) -> Vec<i64> {
    let res = search(db, auth, &format!("q={q}")).await;
    assert_eq!(res.status, StatusCode::OK, "{q}: {:?}", res.json());
    res.json()["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["object"]["id"].as_i64().unwrap())
        .collect()
}

async fn patch(db: &Db, auth: &str, path: &str, body: Value) {
    let res = call(db, Method::PATCH, path, Some(auth), Some(body)).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.json());
}

#[tokio::test]
async fn finds_by_prefix_substring_and_typo() {
    let (_dir, db) = DataDir::new("search-fuzzy");
    let auth = token(&db).await;
    let meeting = create_object(&db, &auth, "note", "Meeting notes").await;
    let groceries = create_object(&db, &auth, "note", "Groceries").await;
    create_object(&db, &auth, "note", "Unrelated").await;

    assert_eq!(ids(&db, &auth, "meet").await, vec![meeting]);
    assert_eq!(ids(&db, &auth, "ocer").await, vec![groceries]);
    assert_eq!(ids(&db, &auth, "groceris").await, vec![groceries]);
    assert_eq!(ids(&db, &auth, "GROCERIES").await, vec![groceries]);
    assert_eq!(ids(&db, &auth, "meet%20notes").await, vec![meeting]);
    assert_eq!(ids(&db, &auth, "meet%20groceries").await, Vec::<i64>::new());
    assert_eq!(ids(&db, &auth, "zzzzzz").await, Vec::<i64>::new());
}

#[tokio::test]
async fn typos_are_tolerated_only_for_words_with_no_exact_match() {
    let (_dir, db) = DataDir::new("search-typo-fallback");
    let auth = token(&db).await;
    let typo = create_object(&db, &auth, "note", "Groceris list").await;
    assert_eq!(ids(&db, &auth, "groceries").await, vec![typo]);
    let exact = create_task(&db, &auth, "Groceries", json!({})).await;
    assert_eq!(ids(&db, &auth, "groceries").await, vec![exact]);
    // Within one type, the other type's exact match doesn't count.
    let res = search(&db, &auth, "q=groceries&type=note").await.json();
    assert_eq!(res["results"][0]["object"]["id"], typo);
    assert_eq!(res["results"][0]["snippet"], "[Groceris] list");
}

#[tokio::test]
async fn reply_has_snippet_score_total_and_best_match_first() {
    let (_dir, db) = DataDir::new("search-shape");
    let auth = token(&db).await;
    let substring = create_object(&db, &auth, "note", "Remeeting plan").await;
    let word = create_object(&db, &auth, "note", "Plan the meet").await;
    let prefix = create_object(&db, &auth, "note", "Meeting notes").await;

    let res = search(&db, &auth, "q=meet").await;
    assert_eq!(res.status, StatusCode::OK);
    let body = res.json();
    assert_eq!(body["total"], 3);
    let results = body["results"].as_array().unwrap();
    let order: Vec<i64> = results
        .iter()
        .map(|r| r["object"]["id"].as_i64().unwrap())
        .collect();
    assert_eq!(order, vec![word, prefix, substring]);
    assert_eq!(results[1]["snippet"], "[Meet]ing notes");
    assert_eq!(results[1]["object"]["title"], "Meeting notes");
    let scores: Vec<f64> = results
        .iter()
        .map(|r| r["score"].as_f64().unwrap())
        .collect();
    assert!(scores[0] > scores[1] && scores[1] > scores[2], "{scores:?}");

    let page = search(&db, &auth, "q=meet&limit=1&offset=1").await.json();
    assert_eq!(page["total"], 3);
    assert_eq!(page["results"].as_array().unwrap().len(), 1);
    assert_eq!(page["results"][0]["object"]["id"], prefix);
    let past = search(&db, &auth, "q=meet&offset=9").await.json();
    assert_eq!(past, json!({ "results": [], "total": 3 }));
}

#[tokio::test]
async fn finds_tags_comments_and_property_values() {
    let (_dir, db) = DataDir::new("search-fields");
    let auth = token(&db).await;
    let tagged = create_object(&db, &auth, "note", "Design review").await;
    patch(
        &db,
        &auth,
        &format!("/api/objects/{tagged}"),
        json!({ "tags": ["work/clientA"] }),
    )
    .await;
    let commented = create_object(&db, &auth, "note", "Plain").await;
    let path = format!("/api/objects/{commented}/comments");
    let res = call(
        &db,
        Method::POST,
        &path,
        Some(&auth),
        Some(json!({ "body": "Call the plumber" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let cid = res.json()["id"].as_i64().unwrap();
    let props = create_object(&db, &auth, "note", "Other").await;
    let body = json!({ "properties": { "place": "Lisbon", "n": 7, "list": ["Kyoto"] } });
    patch(&db, &auth, &format!("/api/objects/{props}"), body).await;

    assert_eq!(ids(&db, &auth, "work%2Fclienta").await, vec![tagged]);
    assert_eq!(ids(&db, &auth, "clienta").await, vec![tagged]);
    assert_eq!(ids(&db, &auth, "plumber").await, vec![commented]);
    let res = search(&db, &auth, "q=plumber").await.json();
    assert_eq!(res["results"][0]["snippet"], "Call the [plumber]");
    assert_eq!(ids(&db, &auth, "lisbon").await, vec![props]);
    assert_eq!(ids(&db, &auth, "kyoto").await, vec![props]);

    // Edits keep the index in step.
    let comment = format!("/api/comments/{cid}");
    patch(
        &db,
        &auth,
        &comment,
        json!({ "body": "Call the electrician" }),
    )
    .await;
    assert_eq!(ids(&db, &auth, "plumber").await, Vec::<i64>::new());
    assert_eq!(ids(&db, &auth, "electrician").await, vec![commented]);
    let res = call(&db, Method::DELETE, &comment, Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(ids(&db, &auth, "electrician").await, Vec::<i64>::new());
    patch(
        &db,
        &auth,
        &format!("/api/objects/{tagged}"),
        json!({ "tags": [], "title": "Renamed" }),
    )
    .await;
    assert_eq!(ids(&db, &auth, "clienta").await, Vec::<i64>::new());
    assert_eq!(ids(&db, &auth, "renamed").await, vec![tagged]);
}

#[tokio::test]
async fn returns_archived_and_done_objects_with_their_flags() {
    let (_dir, db) = DataDir::new("search-history");
    let auth = token(&db).await;
    let archived = create_object(&db, &auth, "note", "Old budget").await;
    patch(
        &db,
        &auth,
        &format!("/api/objects/{archived}"),
        json!({ "archived": true }),
    )
    .await;
    let done = create_task(&db, &auth, "Pay budget invoice", json!({})).await;
    let path = format!("/api/tasks/{done}/complete");
    let res = call(&db, Method::POST, &path, Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::OK);

    let body = search(&db, &auth, "q=budget").await.json();
    assert_eq!(body["total"], 2);
    let results = body["results"].as_array().unwrap();
    let find = |id: i64| results.iter().find(|r| r["object"]["id"] == id).unwrap();
    assert_eq!(find(archived)["object"]["archived"], true);
    assert_eq!(find(done)["object"]["task"]["status"], "done");

    let body = search(&db, &auth, "q=budget&type=task").await.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["results"][0]["object"]["id"], done);
}

#[tokio::test]
async fn trashed_objects_do_not_match_until_restored() {
    let (_dir, db) = DataDir::new("search-trash");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "Secret recipe").await;
    let res = call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{id}"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(ids(&db, &auth, "recipe").await, Vec::<i64>::new());

    let path = format!("/api/trash/{id}/restore");
    let res = call(&db, Method::POST, &path, Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(ids(&db, &auth, "recipe").await, vec![id]);

    call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{id}"),
        Some(&auth),
        None,
    )
    .await;
    let res = call(
        &db,
        Method::DELETE,
        &format!("/api/trash/{id}"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let rows: i64 = _dir
        .sql()
        .query_row("SELECT COUNT(*) FROM search", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn fts_syntax_and_short_queries_never_fail() {
    let (_dir, db) = DataDir::new("search-syntax");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "AND or NEAR(x) \"quoted\" a*").await;
    for q in [
        "%22",
        "*",
        "NEAR(",
        "AND",
        "%22a%22%20OR%20*",
        "-x",
        "a%3Ab",
        "%5E%7B%7D",
    ] {
        let res = search(&db, &auth, &format!("q={q}")).await;
        assert_eq!(res.status, StatusCode::OK, "{q}: {:?}", res.json());
    }
    assert_eq!(ids(&db, &auth, "AND").await, vec![id]);
    assert_eq!(ids(&db, &auth, "near(").await, vec![id]);
    assert_eq!(ids(&db, &auth, "or").await, vec![id]);
    assert_eq!(ids(&db, &auth, "%22").await, Vec::<i64>::new());
}

#[tokio::test]
async fn bad_parameters_get_400_and_no_auth_gets_401() {
    let (_dir, db) = DataDir::new("search-input");
    let auth = token(&db).await;
    let long = "a".repeat(201);
    let cases = [
        ("q=", "q must be 1 to 200 characters"),
        ("q=%20%20", "q must be 1 to 200 characters"),
        ("limit=5", "q must be 1 to 200 characters"),
        (&*format!("q={long}"), "q must be 1 to 200 characters"),
        ("q=a&limit=0", "limit must be 1 to 100"),
        ("q=a&limit=101", "limit must be 1 to 100"),
        ("q=a&offset=-1", "offset must be a non-negative integer"),
    ];
    for (query, error) in cases {
        let res = search(&db, &auth, query).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(res.json(), json!({ "error": error }), "{query}");
    }
    let res = search(&db, &auth, &format!("q={}", "a".repeat(200))).await;
    assert_eq!(res.status, StatusCode::OK);
    let res = search(&db, &auth, "q=a&type=bogus").await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    let res = search(&db, &auth, "q=a&bogus=1").await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    let res = call(&db, Method::GET, "/api/search?q=meet", None, None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[test]
fn migration_backfills_existing_objects() {
    let dir = std::env::temp_dir().join(format!("polykov-search-backfill-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let conn = Connection::open(dir.join(db::FILE_NAME)).unwrap();
    for (version, sql) in [
        (1, include_str!("../migrations/0001_schema_version.sql")),
        (2, include_str!("../migrations/0002_owner_sessions.sql")),
        (3, include_str!("../migrations/0003_api_tokens.sql")),
        (4, include_str!("../migrations/0004_objects.sql")),
        (5, include_str!("../migrations/0005_tasks.sql")),
    ] {
        conn.execute_batch(sql).unwrap();
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [version],
        )
        .unwrap();
    }
    conn.execute_batch(
        "INSERT INTO objects (id, type, title, properties) VALUES (1, 'note', 'Old', '{\"k\":\"Kiwi\"}');
         INSERT INTO object_tags VALUES (1, 'fruit/green');
         INSERT INTO comments (object_id, body) VALUES (1, 'ripe soon');
         INSERT INTO objects (id, type, title, deleted_at) VALUES (2, 'note', 'Gone', 1);",
    )
    .unwrap();
    drop(conn);

    let db = Db::open(&dir).unwrap();
    let conn = Connection::open(dir.join(db::FILE_NAME)).unwrap();
    let rows: Vec<(i64, String, String, String, String)> = conn
        .prepare("SELECT rowid, title, tags, props, comments FROM search")
        .unwrap()
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let want = (
        1,
        "Old".into(),
        "fruit/green".into(),
        "Kiwi".into(),
        "ripe soon".into(),
    );
    assert_eq!(rows, vec![want]);
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
