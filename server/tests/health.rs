use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn get(path: &str) -> (StatusCode, Vec<u8>) {
    let req = Request::get(path).body(Body::empty()).unwrap();
    let res = polykov_server::app::router().oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, body)
}

#[tokio::test]
async fn health_returns_ok_and_version() {
    let (status, body) = get("/health").await;
    assert_eq!(status, StatusCode::OK);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })
    );
}

#[tokio::test]
async fn unknown_path_returns_404() {
    let (status, _) = get("/no-such-path").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
