//! Named, long-lived API tokens for scripts and clients: `POST`/`GET /api/tokens` and
//! `DELETE /api/tokens/{id}`. A token authenticates like a session bearer token but never
//! expires; only its SHA-256 hash is kept in `api_tokens`.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Error, Session, blocking, session};
use crate::db::{self, Db};

/// Maximum token name length, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// A stored token as listed: never the token or its hash. Times are RFC 3339 UTC.
#[derive(Debug, Serialize)]
pub struct Info {
    pub id: i64,
    pub name: String,
    pub created: String,
    pub last_used: Option<String>,
}

/// Stores a new token named `name` and returns its id and the plain token (64 hex chars),
/// which cannot be recovered later.
pub fn create(db: &Db, name: &str) -> Result<(i64, String), Error> {
    let token = session::new_token()?;
    let conn = db.conn()?;
    conn.execute(
        "INSERT INTO api_tokens (name, token_hash) VALUES (?1, ?2)",
        [name, &session::hash_token(&token)],
    )
    .map_err(db::Error::Sqlite)?;
    Ok((conn.last_insert_rowid(), token))
}

/// All tokens, oldest first.
pub fn list(db: &Db) -> Result<Vec<Info>, Error> {
    let conn = db.conn()?;
    let rows = conn
        .prepare(
            "SELECT id, name, strftime('%Y-%m-%dT%H:%M:%SZ', created_at, 'unixepoch'), \
             strftime('%Y-%m-%dT%H:%M:%SZ', last_used, 'unixepoch') FROM api_tokens ORDER BY id",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |r| {
                Ok(Info {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created: r.get(2)?,
                    last_used: r.get(3)?,
                })
            })?
            .collect()
        })
        .map_err(db::Error::Sqlite)?;
    Ok(rows)
}

/// Revokes token `id`. Returns whether it existed.
pub fn delete_by_id(db: &Db, id: i64) -> Result<bool, Error> {
    let conn = db.conn()?;
    let n = conn
        .execute("DELETE FROM api_tokens WHERE id = ?1", [id])
        .map_err(db::Error::Sqlite)?;
    Ok(n > 0)
}

/// Whether `token` is a known API token. A hit records now as its `last_used`.
pub fn authenticate(db: &Db, token: &str) -> Result<bool, Error> {
    let conn = db.conn()?;
    let n = conn
        .execute(
            "UPDATE api_tokens SET last_used = unixepoch() WHERE token_hash = ?1",
            [session::hash_token(token)],
        )
        .map_err(db::Error::Sqlite)?;
    Ok(n > 0)
}

/// The token routes, merged into [`super::routes`]. All require auth.
pub(super) fn routes() -> Router<Db> {
    Router::new()
        .route("/api/tokens", get(list_tokens).post(create_token))
        .route("/api/tokens/{id}", delete(revoke_token))
}

/// Request body of `POST /api/tokens`.
#[derive(Deserialize)]
struct NewToken {
    name: String,
}

async fn create_token(State(db): State<Db>, _: Session, Json(body): Json<NewToken>) -> Response {
    let name = body.name.trim().to_owned();
    let len = name.chars().count();
    if len == 0 || len > MAX_NAME_CHARS {
        let error = format!("name must be 1 to {MAX_NAME_CHARS} characters");
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
    }
    let stored = name.clone();
    match blocking(move || create(&db, &stored)).await {
        Ok((id, token)) => (
            StatusCode::CREATED,
            Json(json!({ "id": id, "name": name, "token": token })),
        )
            .into_response(),
        Err(code) => code.into_response(),
    }
}

async fn list_tokens(State(db): State<Db>, _: Session) -> Response {
    match blocking(move || list(&db)).await {
        Ok(tokens) => Json(tokens).into_response(),
        Err(code) => code.into_response(),
    }
}

async fn revoke_token(State(db): State<Db>, _: Session, Path(id): Path<i64>) -> Response {
    match blocking(move || delete_by_id(&db, id)).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "token not found" })),
        )
            .into_response(),
        Err(code) => code.into_response(),
    }
}
