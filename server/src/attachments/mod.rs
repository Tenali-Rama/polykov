//! File attachments on objects: `POST`/`GET /api/objects/{id}/attachments` and
//! `GET`/`DELETE /api/attachments/{aid}`. Every route requires auth. The upload body is the
//! raw file; files live in `<data dir>/attachments/<id>`.

mod headers;
pub mod store;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::BytesRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
};
pub use store::remove_files;
use store::{Attachment, Error};

/// Largest accepted upload, in bytes (25 MiB).
pub const MAX_BYTES: usize = 25 * 1024 * 1024;
/// Longest accepted file name, in characters.
pub const MAX_NAME_CHARS: usize = 255;

/// The attachment routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/objects/{id}/attachments", get(list).post(upload))
        .route("/api/attachments/{aid}", get(download).delete(remove))
        .layer(DefaultBodyLimit::max(MAX_BYTES))
}

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

#[derive(Deserialize)]
struct UploadQuery {
    name: Option<String>,
}

#[derive(Deserialize)]
struct DownloadQuery {
    inline: Option<u8>,
}

fn object_not_found() -> ApiError {
    ApiError::not_found("object not found")
}

fn attachment_not_found() -> ApiError {
    ApiError::not_found("attachment not found")
}

/// Checks an attachment file name: 1 to 255 characters, no `/`, `\` or control characters
/// (NUL included).
pub fn validate_name(name: &str) -> Result<(), ApiError> {
    let len = name.chars().count();
    if (1..=MAX_NAME_CHARS).contains(&len)
        && !name
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control())
    {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "name must be 1 to 255 characters with no slashes or control characters",
        ))
    }
}

async fn upload(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
    Query(query): Query<UploadQuery>,
    request_headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<(StatusCode, Json<Attachment>), ApiError> {
    let name = query.name.unwrap_or_default();
    validate_name(&name)?;
    let mime = match request_headers.get(header::CONTENT_TYPE) {
        None => "application/octet-stream".to_owned(),
        Some(v) => v
            .to_str()
            .map_err(|_| ApiError::bad_request("Content-Type must be ASCII"))?
            .to_owned(),
    };
    let bytes = body.map_err(|e| match e.status() {
        StatusCode::PAYLOAD_TOO_LARGE => {
            ApiError::Client(e.status(), "attachment must be at most 25 MiB".to_owned())
        }
        status => ApiError::Client(status, e.body_text()),
    })?;
    blocking(move || store::create(&db, id, &name, &mime, &bytes))
        .await?
        .map(|a| (StatusCode::CREATED, Json(a)))
        .ok_or_else(object_not_found)
}

async fn list(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let attachments = blocking(move || store::list(&db, id))
        .await?
        .ok_or_else(object_not_found)?;
    Ok(Json(json!({ "attachments": attachments })))
}

async fn download(
    State(db): State<Db>,
    _: Session,
    Path(aid): Path<i64>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, ApiError> {
    let (attachment, bytes) = blocking(move || {
        let Some(a) = store::get(&db, aid)? else {
            return Ok(None);
        };
        let path = store::file_path(&db, aid);
        let bytes = std::fs::read(&path).map_err(|e| Error::Io(path, e))?;
        Ok::<_, Error>(Some((a, bytes)))
    })
    .await?
    .ok_or_else(attachment_not_found)?;
    let inline = query.inline == Some(1);
    Ok((headers::download(&attachment, inline)?, bytes).into_response())
}

async fn remove(
    State(db): State<Db>,
    _: Session,
    Path(aid): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if blocking(move || store::delete(&db, aid)).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(attachment_not_found())
    }
}
