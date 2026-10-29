//! Owner setup, password login and session issuing (spec §9).
//!
//! Routes: `GET`/`POST /api/setup` and `POST /api/login`. Checking sessions on requests is
//! built on [`session::is_valid`].

pub mod owner;
pub mod password;
pub mod session;

use std::fmt;

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;

use crate::db::{self, Db};

/// Minimum owner password length, in characters.
pub const MIN_PASSWORD_CHARS: usize = 12;

/// Name of the session cookie set on login.
pub const COOKIE_NAME: &str = "polykov_session";

/// The auth routes, to be merged into the app router.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/setup", get(setup_needed).post(setup))
        .route("/api/login", post(login))
}

/// Request body of setup and login. No `Debug`, so the password cannot end up in a log line.
#[derive(Deserialize)]
struct PasswordBody {
    password: String,
}

async fn setup_needed(State(db): State<Db>) -> Response {
    match blocking(move || owner::exists(&db)).await {
        Ok(exists) => Json(json!({ "needed": !exists })).into_response(),
        Err(code) => code.into_response(),
    }
}

async fn setup(State(db): State<Db>, Json(body): Json<PasswordBody>) -> Response {
    if body.password.chars().count() < MIN_PASSWORD_CHARS {
        let error = format!("password must be at least {MIN_PASSWORD_CHARS} characters");
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
    }
    let created = blocking(move || {
        if owner::exists(&db)? {
            return Ok(false);
        }
        let hash = password::hash(&body.password)?;
        owner::create(&db, &hash)
    })
    .await;
    match created {
        Ok(true) => StatusCode::CREATED.into_response(),
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "owner already exists" })),
        )
            .into_response(),
        Err(code) => code.into_response(),
    }
}

async fn login(State(db): State<Db>, Json(body): Json<PasswordBody>) -> Response {
    let token = blocking(move || {
        let Some(hash) = owner::password_hash(&db)? else {
            return Ok(None);
        };
        if !password::verify(&body.password, &hash)? {
            return Ok(None);
        }
        session::create(&db).map(Some)
    })
    .await;
    match token {
        Ok(Some(token)) => {
            let cookie = format!(
                "{COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
                session::TTL_SECS
            );
            (
                [(header::SET_COOKIE, cookie)],
                Json(json!({ "token": token })),
            )
                .into_response()
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(code) => code.into_response(),
    }
}

/// Runs `f` off the async runtime. Failures are logged (they never carry the password or
/// token) and become a bare 500.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> Result<T, StatusCode> {
    let failed = |e: &dyn fmt::Display| {
        eprintln!("polykov-server: auth: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    };
    match tokio::task::spawn_blocking(f).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(failed(&e)),
        Err(e) => Err(failed(&e)),
    }
}

#[derive(Debug)]
pub enum Error {
    /// A database call failed.
    Db(db::Error),
    /// Hashing or verifying a password failed, or a stored hash is malformed.
    Hash(argon2::password_hash::Error),
    /// The OS random number generator failed.
    Random(getrandom::Error),
}

impl From<db::Error> for Error {
    fn from(e: db::Error) -> Self {
        Error::Db(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Db(e) => write!(f, "{e}"),
            Error::Hash(e) => write!(f, "password hash error: {e}"),
            Error::Random(e) => write!(f, "random number generator failed: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Db(e) => Some(e),
            Error::Hash(_) | Error::Random(_) => None,
        }
    }
}
