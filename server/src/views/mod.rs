//! Saved views (spec "Organization", ADR 0016): objects of type `view` whose `properties` hold
//! a query, layout, group and sort. `POST`/`GET /api/views`, `PATCH /api/views/{id}` and
//! `GET /api/views/{id}/items`. Every route requires auth; rename, delete and pin go through
//! the object routes.

mod items;
mod keys;
pub mod spec;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch},
};
use rusqlite::params;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value, json};

use crate::{
    auth::Session,
    db::{self, Db},
    http::{ApiError, blocking},
    objects::{
        store::{self as objects, Changes, New, Object},
        validate,
    },
};
use spec::{GROUPS, Invalid, LAYOUTS, SORTS};

/// The object type of a view.
pub const TYPE: &str = "view";

/// The view routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/views", get(list).post(create))
        .route("/api/views/{id}", patch(update))
        .route("/api/views/{id}/items", get(items::items))
}

/// Body of `POST /api/views`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewBody {
    title: String,
    query: String,
    #[serde(default = "default_layout")]
    layout: String,
    #[serde(default = "default_group")]
    group: String,
    #[serde(default = "default_sort")]
    sort: String,
    #[serde(default)]
    pinned: bool,
}

fn default_layout() -> String {
    "list".into()
}

fn default_group() -> String {
    "none".into()
}

fn default_sort() -> String {
    "smart".into()
}

/// Body of `PATCH /api/views/{id}`. A field that is present must not be `null`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchBody {
    #[serde(default, deserialize_with = "present")]
    title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    query: Option<String>,
    #[serde(default, deserialize_with = "present")]
    layout: Option<String>,
    #[serde(default, deserialize_with = "present")]
    group: Option<String>,
    #[serde(default, deserialize_with = "present")]
    sort: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pinned: Option<bool>,
}

/// Deserializes a present field as `Some`, so an explicit `null` is not mistaken for absent.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

pub(crate) fn not_found() -> ApiError {
    ApiError::not_found("view not found")
}

/// Checks each given setting and writes it into `properties`.
fn apply(
    properties: &mut Map<String, Value>,
    query: Option<String>,
    layout: Option<String>,
    group: Option<String>,
    sort: Option<String>,
) -> Result<(), Invalid> {
    if let Some(text) = query {
        spec::query(&text)?;
        properties.insert("query".into(), Value::String(text));
    }
    set(properties, "layout", layout, &LAYOUTS)?;
    set(properties, "group", group, &GROUPS)?;
    set(properties, "sort", sort, &SORTS)
}

/// Checks a given `value` is one of `options` and writes it into `properties` as `field`.
fn set<T: Copy>(
    properties: &mut Map<String, Value>,
    field: &str,
    value: Option<String>,
    options: &[(&str, T)],
) -> Result<(), Invalid> {
    if let Some(value) = value {
        spec::pick(field, &value, options)?;
        properties.insert(field.into(), Value::String(value));
    }
    Ok(())
}

async fn create(
    State(db): State<Db>,
    _: Session,
    Json(body): Json<NewBody>,
) -> Result<Response, ApiError> {
    let title = validate::title(&body.title)?;
    let mut properties = Map::new();
    if let Err(e) = apply(
        &mut properties,
        Some(body.query),
        Some(body.layout),
        Some(body.group),
        Some(body.sort),
    ) {
        return Ok(e.respond(StatusCode::BAD_REQUEST));
    }
    let new = New {
        r#type: TYPE.into(),
        title,
        properties,
        pinned: body.pinned,
        archived: false,
        tags: Vec::new(),
    };
    let view = blocking(move || objects::create(&db, &new)).await?;
    Ok((StatusCode::CREATED, Json(view)).into_response())
}

async fn update(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
    Json(body): Json<PatchBody>,
) -> Result<Response, ApiError> {
    let title = body.title.as_deref().map(validate::title).transpose()?;
    let read = db.clone();
    let view = blocking(move || find(&read, id))
        .await?
        .ok_or_else(not_found)?;
    let given = body.query.is_some()
        || body.layout.is_some()
        || body.group.is_some()
        || body.sort.is_some();
    let mut properties = view.properties;
    if let Err(e) = apply(
        &mut properties,
        body.query,
        body.layout,
        body.group,
        body.sort,
    ) {
        return Ok(e.respond(StatusCode::BAD_REQUEST));
    }
    let changes = Changes {
        title,
        properties: given.then_some(properties),
        pinned: body.pinned,
        ..Changes::default()
    };
    let view = blocking(move || objects::update(&db, id, &changes))
        .await?
        .ok_or_else(not_found)?;
    Ok(Json(view).into_response())
}

/// The live object `id` if it is a view.
pub(crate) fn find(db: &Db, id: i64) -> Result<Option<Object>, db::Error> {
    Ok(objects::get(db, id)?.filter(|o| o.r#type == TYPE))
}

async fn list(State(db): State<Db>, _: Session) -> Result<Json<Value>, ApiError> {
    let views = blocking(move || select(&db)).await?;
    Ok(Json(json!({ "views": views })))
}

/// Live, non-archived views: pinned first, then by title (ignoring case), then id.
fn select(db: &Db) -> Result<Vec<Object>, db::Error> {
    let conn = db.conn()?;
    let sql = format!(
        "SELECT {} FROM objects WHERE type = ?1 AND deleted_at IS NULL AND archived = 0 \
         ORDER BY pinned DESC, title COLLATE NOCASE, id",
        objects::columns()
    );
    let mut stmt = conn.prepare(&sql).map_err(db::Error::Sqlite)?;
    let rows = stmt
        .query_map(params![TYPE], objects::object)
        .map_err(db::Error::Sqlite)?;
    rows.collect::<Result<_, _>>().map_err(db::Error::Sqlite)
}
