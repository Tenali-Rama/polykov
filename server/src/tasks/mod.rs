//! Tasks (spec §3 "Task", "Tasks (GTD)"): `POST /api/tasks`, `GET`/`PATCH
//! /api/tasks/{id}` and `POST /api/tasks/{id}/complete|reopen`. Every route requires auth.

pub mod store;
pub mod validate;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    objects::{self, validate as object_rules},
    tags,
};
use store::{Changes, FieldChanges, New, Task};

/// The task routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/tasks", post(create))
        .route("/api/tasks/{id}", get(read).patch(update))
        .route("/api/tasks/{id}/complete", post(complete))
        .route("/api/tasks/{id}/reopen", post(reopen))
}

/// Body of `POST /api/tasks`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewBody {
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default, deserialize_with = "present")]
    properties: Option<Value>,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    task: FieldChanges,
}

/// Body of `PATCH /api/tasks/{id}`. A present object field must not be `null`.
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
    #[serde(default)]
    task: FieldChanges,
}

/// Deserializes a present field as `Some`, so an explicit `null` is not mistaken for absent.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

fn not_found() -> ApiError {
    ApiError::not_found("task not found")
}

async fn create(
    State(db): State<Db>,
    _: Session,
    Json(body): Json<NewBody>,
) -> Result<(StatusCode, Json<Task>), ApiError> {
    let new = New {
        title: object_rules::title(&body.title)?,
        properties: match body.properties {
            Some(value) => object_rules::properties(value)?,
            None => Default::default(),
        },
        pinned: body.pinned,
        tags: tags::normalize_all(&body.tags)?,
        task: body.task,
    };
    let task = blocking(move || store::create(&db, &new)).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

async fn read(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Task>, ApiError> {
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
) -> Result<Json<Task>, ApiError> {
    let changes = Changes {
        object: objects::store::Changes {
            title: body.title.as_deref().map(object_rules::title).transpose()?,
            properties: body.properties.map(object_rules::properties).transpose()?,
            pinned: body.pinned,
            archived: body.archived,
            tags: body.tags.as_deref().map(tags::normalize_all).transpose()?,
        },
        task: body.task,
    };
    blocking(move || store::update(&db, id, &changes))
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

async fn complete(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let task = blocking(move || store::complete(&db, id))
        .await?
        .ok_or_else(not_found)?;
    Ok(Json(json!({ "task": task, "next": null })))
}

async fn reopen(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Task>, ApiError> {
    blocking(move || store::reopen(&db, id))
        .await?
        .map(Json)
        .ok_or_else(not_found)
}
