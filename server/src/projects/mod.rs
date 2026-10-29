//! Projects (spec §3 "Project", "Tasks (GTD)"): `GET /api/projects`, `GET`/`PATCH
//! /api/projects/{id}`, and project sections under `/api/projects/{id}/sections` and
//! `/api/sections/{sid}`. Every route requires auth.

pub mod sections;
pub mod store;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch},
};
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
};
use sections::Section;
use store::{Changes, Filter, Project};

/// The project and section routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/projects", get(list))
        .route("/api/projects/{id}", get(read).patch(update))
        .route(
            "/api/projects/{id}/sections",
            get(list_sections).post(create_section),
        )
        .route(
            "/api/sections/{sid}",
            patch(update_section).delete(delete_section),
        )
}

/// Query string of `GET /api/projects`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListParams {
    status: Option<String>,
    area: Option<String>,
}

/// Body of `POST /api/projects/{id}/sections`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewSection {
    title: String,
    #[serde(default, deserialize_with = "present")]
    position: Option<i64>,
}

/// Body of `PATCH /api/sections/{sid}`. A present field must not be `null`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionPatch {
    #[serde(default, deserialize_with = "present")]
    title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    position: Option<i64>,
}

/// Deserializes a present field as `Some`, so an explicit `null` is not mistaken for absent.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

fn not_found() -> ApiError {
    ApiError::not_found("project not found")
}

fn section_not_found() -> ApiError {
    ApiError::not_found("section not found")
}

impl ListParams {
    fn parse(self) -> Result<Filter, ApiError> {
        let status = match self.status.as_deref() {
            None => Some("active".to_owned()),
            Some("any") => None,
            Some(s) if store::STATUSES.contains(&s) => Some(s.to_owned()),
            Some(_) => {
                return Err(ApiError::bad_request(
                    "status must be active, someday, done or any",
                ));
            }
        };
        let area_id = self
            .area
            .map(|raw| {
                raw.parse()
                    .map_err(|_| ApiError::bad_request("area must be an integer id"))
            })
            .transpose()?;
        Ok(Filter { status, area_id })
    }
}

async fn list(
    State(db): State<Db>,
    _: Session,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    let filter = params.parse()?;
    let projects = blocking(move || store::list(&db, &filter)).await?;
    Ok(Json(json!({ "projects": projects })))
}

async fn read(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Project>, ApiError> {
    blocking(move || store::get(&db, id))
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

async fn update(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
    Json(changes): Json<Changes>,
) -> Result<Json<Project>, ApiError> {
    blocking(move || store::update(&db, id, &changes))
        .await?
        .map(Json)
        .ok_or_else(not_found)
}

async fn list_sections(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let sections = blocking(move || sections::list(&db, id))
        .await?
        .ok_or_else(not_found)?;
    Ok(Json(json!({ "sections": sections })))
}

async fn create_section(
    State(db): State<Db>,
    _: Session,
    Path(id): Path<i64>,
    Json(body): Json<NewSection>,
) -> Result<(StatusCode, Json<Section>), ApiError> {
    let title = sections::title(&body.title)?;
    let position = body.position.map(sections::position).transpose()?;
    let section = blocking(move || sections::create(&db, id, &title, position))
        .await?
        .ok_or_else(not_found)?;
    Ok((StatusCode::CREATED, Json(section)))
}

async fn update_section(
    State(db): State<Db>,
    _: Session,
    Path(sid): Path<i64>,
    Json(body): Json<SectionPatch>,
) -> Result<Json<Section>, ApiError> {
    let title = body.title.as_deref().map(sections::title).transpose()?;
    let position = body.position.map(sections::position).transpose()?;
    blocking(move || sections::update(&db, sid, title.as_deref(), position))
        .await?
        .map(Json)
        .ok_or_else(section_not_found)
}

async fn delete_section(
    State(db): State<Db>,
    _: Session,
    Path(sid): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if blocking(move || sections::delete(&db, sid)).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(section_not_found())
    }
}
