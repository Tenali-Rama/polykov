//! Shared plumbing for feature-module handlers: the [`ApiError`] response and [`blocking`]
//! for running store calls off the async runtime.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::db;

/// What a handler returns when it cannot answer normally.
#[derive(Debug)]
pub enum ApiError {
    /// A client error, sent as `{"error": message}` with this 4xx status.
    Client(StatusCode, String),
    /// A store or runtime failure: its message is logged as one line, and the client gets a
    /// bare 500.
    Internal(String),
}

impl ApiError {
    /// `400 {"error": message}`.
    pub fn bad_request(message: impl Into<String>) -> Self {
        ApiError::Client(StatusCode::BAD_REQUEST, message.into())
    }

    /// `404 {"error": message}`.
    pub fn not_found(message: impl Into<String>) -> Self {
        ApiError::Client(StatusCode::NOT_FOUND, message.into())
    }
}

impl From<db::Error> for ApiError {
    fn from(e: db::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Client(code, error) => {
                (code, Json(json!({ "error": error }))).into_response()
            }
            ApiError::Internal(cause) => {
                eprintln!("polykov-server: {cause}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// Runs the blocking store call `f` on Tokio's blocking pool. Its error, or a panic, becomes
/// an [`ApiError`].
pub async fn blocking<T, E>(
    f: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> Result<T, ApiError>
where
    T: Send + 'static,
    E: Into<ApiError> + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(result) => result.map_err(Into::into),
        Err(e) => Err(ApiError::Internal(format!("blocking task failed: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn parts(e: ApiError) -> (StatusCode, Vec<u8>) {
        let res = e.into_response();
        let code = res.status();
        (
            code,
            res.into_body().collect().await.unwrap().to_bytes().to_vec(),
        )
    }

    #[tokio::test]
    async fn client_errors_are_json() {
        let (code, body) = parts(ApiError::not_found("object not found")).await;
        assert_eq!(code, StatusCode::NOT_FOUND);
        assert_eq!(body, br#"{"error":"object not found"}"#);
    }

    #[tokio::test]
    async fn store_failures_are_a_bare_500() {
        let (code, body) = parts(db::Error::Poisoned.into()).await;
        assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn blocking_maps_errors_and_panics() {
        let ok = blocking(|| Ok::<_, ApiError>(7)).await;
        assert_eq!(ok.unwrap(), 7);
        let err = blocking(|| Err::<(), _>(ApiError::bad_request("no"))).await;
        assert!(matches!(
            err,
            Err(ApiError::Client(StatusCode::BAD_REQUEST, _))
        ));
        let panicked = blocking(|| -> Result<(), ApiError> { panic!("boom") }).await;
        assert!(matches!(panicked, Err(ApiError::Internal(_))));
    }
}
