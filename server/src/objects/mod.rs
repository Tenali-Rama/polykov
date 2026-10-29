//! Generic objects (spec §3): `GET`/`POST /api/objects` and `GET`/`PATCH`/`DELETE
//! /api/objects/{id}`. Every route requires auth. Delete moves an object to the trash.

mod list;
pub mod store;
pub mod validate;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    tags,
};
use store::{Changes, New, Object};

/// The object routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/objects", post(create).get(list::list))
        .route("/api/objects/{id}", get(read).patch(update).delete(remove))
}

/// Body of `POST /api/objects`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewBody {
    r#type: String,
    title: String,
    #[serde(default, deserialize_with = "present")]
    properties: Option<Value>,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    tags: Vec<String>,
}

/// Body of `PATCH /api/objects/{id}`. A field that is present must not be `null`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchBody {
    #[serde(default, deserialize_with = "present")]
    title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    properties: Option<Value>,
    #[serde(default, deserialize_with = "present")]
    pinned: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    archived: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    tags: Option<Vec<String>>,
}

/// Deserializes a present field as `Some`, so an explicit `null` is not mistaken for absent.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

fn not_found() -> ApiError {
    ApiError::not_found("object not found")
}

async fn create(
    State(db): State<Db>,
    _: Session,
    Json(body): Json<NewBody>,
) -> Result<(StatusCode, Json<Object>), ApiError> {
    validate::object_type(&body.r#type)?;
    let new = New {
        title: validate::title(&body.title)?,
        properties: match body.properties {
            Some(value) => validate::properties(value)?,
            None => Default::default(),
        },
        r#type: body.r#type,
        pinned: body.pinned,
        archived: body.archived,
        tags: tags::normalize_all(&body.tags)?,
    };
    let object = blocking(move || store::create(&db, &new)).await?;
    Ok((StatusCode::CREATED, Json(object)))
}

async fn read(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Object>, ApiError> {
    blocking(move || store::get(&db, id))
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

async fn update(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
    Json(body): Json<PatchBody>,
) -> Result<Json<Object>, ApiError> {
    let changes = Changes {
        title: body.title.as_deref().map(validate::title).transpose()?,
        properties: body.properties.map(validate::properties).transpose()?,
        pinned: body.pinned,
        archived: body.archived,
        tags: body.tags.as_deref().map(tags::normalize_all).transpose()?,
    };
    blocking(move || store::update(&db, id, &changes))
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

async fn remove(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if blocking(move || store::trash(&db, id)).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}
