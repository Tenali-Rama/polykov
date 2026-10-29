//! The [`Session`] extractor: what makes a route require login.

use axum::{
    Json,
    extract::{FromRef, FromRequestParts},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde_json::json;

use super::{COOKIE_NAME, blocking, session, tokens};
use crate::db::Db;

/// A valid, unexpired session or API token, taken from `Authorization: Bearer <token>` or
/// else from the `polykov_session` cookie. Adding it as a handler argument makes the route
/// require auth: a missing, unknown or expired token is rejected with
/// `401 {"error":"authentication required"}` before the handler runs.
pub struct Session {
    /// The raw token the client sent.
    pub token: String,
    /// When the session expires, RFC 3339 UTC; `None` for an API token, which never expires.
    pub expires_at: Option<String>,
}

impl<S> FromRequestParts<S> for Session
where
    Db: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(token) = token(&parts.headers) else {
            return Err(unauthorized());
        };
        let db = Db::from_ref(state);
        let lookup = token.clone();
        let found = blocking(move || match session::expires_at(&db, &lookup)? {
            Some(expires_at) => Ok(Some(Some(expires_at))),
            None => Ok(tokens::authenticate(&db, &lookup)?.then_some(None)),
        });
        match found.await {
            Ok(Some(expires_at)) => Ok(Session { token, expires_at }),
            Ok(None) => Err(unauthorized()),
            Err(code) => Err(code.into_response()),
        }
    }
}

/// The token from the bearer header if there is one, otherwise from the session cookie.
/// A present but invalid bearer header does not fall back to the cookie.
fn token(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get(header::AUTHORIZATION) {
        let value = value.to_str().ok()?;
        let (scheme, token) = value.split_once(' ')?;
        let token = token.trim();
        return (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty())
            .then(|| token.to_owned());
    }
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == COOKIE_NAME && !value.is_empty())
        .map(|(_, value)| value.to_owned())
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "authentication required" })),
    )
        .into_response()
}
