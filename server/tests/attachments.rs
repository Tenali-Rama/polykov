//! Attachments: upload, list, download and delete under `/api/objects/{id}/attachments` and
//! `/api/attachments/{aid}`, the files in `<data dir>/attachments/`, and
//! `attachments::remove_files`.

mod common;

use std::fs;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{DataDir, Reply, call, create_object, send, token};
use polykov_server::{
    attachments::{self, MAX_BYTES},
    db::Db,
};
use serde_json::json;
use tower::ServiceExt;

async fn upload(
    db: &Db,
    auth: &str,
    id: i64,
    query: &str,
    mime: Option<&str>,
    body: Vec<u8>,
) -> Reply {
    let mut req = Request::post(format!("/api/objects/{id}/attachments?{query}"))
        .header(header::AUTHORIZATION, format!("Bearer {auth}"));
    if let Some(mime) = mime {
        req = req.header(header::CONTENT_TYPE, mime);
    }
    send(db, req.body(Body::from(body)).unwrap()).await
}

fn assert_error(res: &Reply, status: StatusCode, error: &str) {
    assert_eq!(res.status, status);
    assert_eq!(res.json(), json!({ "error": error }));
}

fn rows(dir: &DataDir) -> i64 {
    let sql = "SELECT COUNT(*) FROM attachments";
    dir.sql().query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Uploads `b"a"` as `name` and returns the attachment id.
async fn upload_id(db: &Db, auth: &str, id: i64, name: &str) -> i64 {
    let res = upload(db, auth, id, &format!("name={name}"), None, b"a".to_vec()).await;
    res.json()["id"].as_i64().unwrap()
}

fn file(db: &Db, aid: i64) -> std::path::PathBuf {
    db.data_dir().join("attachments").join(aid.to_string())
}

/// Sends `GET path` and returns the status, the named headers and the body.
async fn download(db: &Db, auth: &str, path: &str) -> (StatusCode, [Option<String>; 3], Vec<u8>) {
    let req = Request::get(path)
        .header(header::AUTHORIZATION, format!("Bearer {auth}"))
        .body(Body::empty())
        .unwrap();
    let res = polykov_server::app::router(db.clone(), &std::env::temp_dir().join("polykov-no-web"))
        .oneshot(req)
        .await
        .unwrap();
    let h = |name| {
        res.headers()
            .get(name)
            .map(|v: &header::HeaderValue| v.to_str().unwrap().to_owned())
    };
    let headers = [
        h(header::CONTENT_TYPE),
        h(header::CONTENT_DISPOSITION),
        h(header::X_CONTENT_TYPE_OPTIONS),
    ];
    let status = res.status();
    let body = http_body_util::BodyExt::collect(res.into_body())
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, headers, body)
}

#[tokio::test]
async fn upload_list_download_round_trip() {
    let (_dir, db) = DataDir::new("att-roundtrip");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "task", "Receipt").await;
    let bytes = vec![0u8, 1, 2, 255, 10];

    let res = upload(
        &db,
        &auth,
        id,
        "name=r%C3%A9sum%C3%A9%20v2.pdf",
        Some("application/pdf"),
        bytes.clone(),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let a = res.json();
    let aid = a["id"].as_i64().unwrap();
    assert_eq!(a["object_id"], id);
    assert_eq!(a["name"], "résumé v2.pdf");
    assert_eq!(a["mime"], "application/pdf");
    assert_eq!(a["size"], 5);
    assert_eq!(a["created"].as_str().unwrap().len(), 20);
    assert_eq!(fs::read(file(&db, aid)).unwrap(), bytes);

    let second = upload(&db, &auth, id, "name=b.bin", None, b"x".to_vec()).await;
    assert_eq!(second.json()["mime"], "application/octet-stream");

    let res = call(
        &db,
        Method::GET,
        &format!("/api/objects/{id}/attachments"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let list = res.json();
    assert_eq!(list["attachments"].as_array().unwrap().len(), 2);
    assert_eq!(list["attachments"][0], a);
    assert_eq!(list["attachments"][1], second.json());

    let (status, [mime, disposition, nosniff], body) =
        download(&db, &auth, &format!("/api/attachments/{aid}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, bytes);
    assert_eq!(mime.as_deref(), Some("application/pdf"));
    assert_eq!(
        disposition.as_deref(),
        Some("attachment; filename*=UTF-8''r%C3%A9sum%C3%A9%20v2.pdf")
    );
    assert_eq!(nosniff.as_deref(), Some("nosniff"));

    // `?inline=1` is ignored for anything but a raster image.
    let (_, [_, disposition, _], _) =
        download(&db, &auth, &format!("/api/attachments/{aid}?inline=1")).await;
    assert!(disposition.unwrap().starts_with("attachment;"));
}

#[tokio::test]
async fn inline_is_for_images_only() {
    let (_dir, db) = DataDir::new("att-inline");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "Pics").await;
    let png = upload(
        &db,
        &auth,
        id,
        "name=a.png",
        Some("image/png"),
        b"png".to_vec(),
    )
    .await;
    let svg = upload(
        &db,
        &auth,
        id,
        "name=a.svg",
        Some("image/svg+xml"),
        b"<svg/>".to_vec(),
    )
    .await;
    let png = png.json()["id"].as_i64().unwrap();
    let svg = svg.json()["id"].as_i64().unwrap();

    let (_, [mime, disposition, _], _) =
        download(&db, &auth, &format!("/api/attachments/{png}?inline=1")).await;
    assert_eq!(mime.as_deref(), Some("image/png"));
    assert_eq!(
        disposition.as_deref(),
        Some("inline; filename*=UTF-8''a.png")
    );
    let (_, [_, disposition, _], _) =
        download(&db, &auth, &format!("/api/attachments/{png}")).await;
    assert_eq!(
        disposition.as_deref(),
        Some("attachment; filename*=UTF-8''a.png")
    );
    let (_, [_, disposition, _], _) =
        download(&db, &auth, &format!("/api/attachments/{svg}?inline=1")).await;
    assert_eq!(
        disposition.as_deref(),
        Some("attachment; filename*=UTF-8''a.svg")
    );
}

#[tokio::test]
async fn bad_names_get_400() {
    let (dir, db) = DataDir::new("att-names");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "N").await;
    let long = "a".repeat(256);
    let ok_long = "é".repeat(255);
    let msg = "name must be 1 to 255 characters with no slashes or control characters";
    for query in [
        "",
        "name=",
        "name=a%2Fb",
        "name=a%5Cb",
        "name=a%00b",
        "name=a%0Ab",
        &format!("name={long}"),
    ] {
        let res = upload(&db, &auth, id, query, None, b"x".to_vec()).await;
        assert_error(&res, StatusCode::BAD_REQUEST, msg);
    }
    let res = upload(
        &db,
        &auth,
        id,
        &format!("name={ok_long}"),
        None,
        b"x".to_vec(),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(rows(&dir), 1);
}

#[tokio::test]
async fn body_over_25_mib_gets_413() {
    let (dir, db) = DataDir::new("att-413");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "Big").await;
    let res = upload(&db, &auth, id, "name=big", None, vec![7; MAX_BYTES + 1]).await;
    assert_error(
        &res,
        StatusCode::PAYLOAD_TOO_LARGE,
        "attachment must be at most 25 MiB",
    );
    let res = upload(&db, &auth, id, "name=max", None, vec![7; MAX_BYTES]).await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.json()["size"], MAX_BYTES);
    assert_eq!(rows(&dir), 1);
}

#[tokio::test]
async fn unknown_or_trashed_targets_get_404() {
    let (_dir, db) = DataDir::new("att-404");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "Gone").await;
    let aid = upload_id(&db, &auth, id, "a").await;

    let res = upload(&db, &auth, 9999, "name=a", None, b"a".to_vec()).await;
    assert_error(&res, StatusCode::NOT_FOUND, "object not found");
    let res = call(
        &db,
        Method::GET,
        "/api/objects/9999/attachments",
        Some(&auth),
        None,
    )
    .await;
    assert_error(&res, StatusCode::NOT_FOUND, "object not found");
    for method in [Method::GET, Method::DELETE] {
        let res = call(&db, method, "/api/attachments/9999", Some(&auth), None).await;
        assert_error(&res, StatusCode::NOT_FOUND, "attachment not found");
    }

    let res = call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{id}"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let res = upload(&db, &auth, id, "name=b", None, b"b".to_vec()).await;
    assert_error(&res, StatusCode::NOT_FOUND, "object not found");
    let res = call(
        &db,
        Method::GET,
        &format!("/api/objects/{id}/attachments"),
        Some(&auth),
        None,
    )
    .await;
    assert_error(&res, StatusCode::NOT_FOUND, "object not found");
    for method in [Method::GET, Method::DELETE] {
        let res = call(
            &db,
            method,
            &format!("/api/attachments/{aid}"),
            Some(&auth),
            None,
        )
        .await;
        assert_error(&res, StatusCode::NOT_FOUND, "attachment not found");
    }
    // Deleting through a trashed object changes nothing.
    assert!(file(&db, aid).exists());
}

#[tokio::test]
async fn every_route_requires_auth() {
    let (_dir, db) = DataDir::new("att-401");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "N").await;
    let aid = upload_id(&db, &auth, id, "a").await;
    let routes = [
        (
            Method::POST,
            format!("/api/objects/{id}/attachments?name=x"),
        ),
        (Method::GET, format!("/api/objects/{id}/attachments")),
        (Method::GET, format!("/api/attachments/{aid}")),
        (Method::DELETE, format!("/api/attachments/{aid}")),
    ];
    for (method, path) in routes {
        let res = call(&db, method, &path, None, None).await;
        assert_error(&res, StatusCode::UNAUTHORIZED, "authentication required");
    }
    assert!(file(&db, aid).exists());
}

#[tokio::test]
async fn delete_removes_row_and_file() {
    let (_dir, db) = DataDir::new("att-delete");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "N").await;
    let aid = upload_id(&db, &auth, id, "a").await;
    let path = format!("/api/attachments/{aid}");
    let res = call(&db, Method::DELETE, &path, Some(&auth), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert!(res.body.is_empty());
    assert!(!file(&db, aid).exists());
    let res = call(&db, Method::GET, &path, Some(&auth), None).await;
    assert_error(&res, StatusCode::NOT_FOUND, "attachment not found");
    let res = call(
        &db,
        Method::GET,
        &format!("/api/objects/{id}/attachments"),
        Some(&auth),
        None,
    )
    .await;
    assert_eq!(res.json(), json!({ "attachments": [] }));
}

#[tokio::test]
async fn failed_write_keeps_no_row() {
    let (dir, db) = DataDir::new("att-write-fail");
    let auth = token(&db).await;
    let id = create_object(&db, &auth, "note", "N").await;
    // A plain file where the attachments dir should be makes every write fail.
    let _ = fs::remove_dir_all(db.data_dir().join("attachments"));
    fs::write(db.data_dir().join("attachments"), b"").unwrap();
    let res = upload(&db, &auth, id, "name=a", None, b"a".to_vec()).await;
    assert_eq!(res.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(rows(&dir), 0);
}

#[tokio::test]
async fn remove_files_deletes_one_objects_files() {
    let (dir, db) = DataDir::new("att-remove-files");
    let auth = token(&db).await;
    let a = create_object(&db, &auth, "note", "A").await;
    let b = create_object(&db, &auth, "note", "B").await;
    let mut ids = Vec::new();
    for (obj, name) in [(a, "1"), (a, "2"), (b, "3")] {
        let res = upload(
            &db,
            &auth,
            obj,
            &format!("name={name}"),
            None,
            b"x".to_vec(),
        )
        .await;
        ids.push(res.json()["id"].as_i64().unwrap());
    }
    // Works on a trashed object (the purge case), and a missing file is not an error.
    call(
        &db,
        Method::DELETE,
        &format!("/api/objects/{a}"),
        Some(&auth),
        None,
    )
    .await;
    fs::remove_file(file(&db, ids[1])).unwrap();
    attachments::remove_files(&db, a).unwrap();
    assert!(!file(&db, ids[0]).exists());
    assert!(file(&db, ids[2]).exists());
    assert_eq!(rows(&dir), 3);
}
