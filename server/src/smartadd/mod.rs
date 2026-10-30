//! Smart add (spec §4 "Capture"): `POST /api/parse` previews how a line is read, and
//! `POST /api/tasks/add` turns it into a task. Every route requires auth.

pub mod line;

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use chrono::NaiveDate;
use polykov_parse::{Anchor, Kind, Recurrence};
use serde::{Deserialize, Serialize};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
    objects::validate as object_rules,
    tags,
    tasks::{
        self, recurrence as task_recurrence,
        store::{FieldChanges, New, Task},
    },
};

/// The smart-add routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new()
        .route("/api/parse", post(parse))
        .route("/api/tasks/add", post(add_task))
}

/// Body of both routes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    text: String,
    today: Option<String>,
}

impl Body {
    /// `today`, else the server's UTC date.
    fn today(&self) -> Result<NaiveDate, ApiError> {
        match &self.today {
            Some(day) => NaiveDate::parse_from_str(day, "%Y-%m-%d")
                .ok()
                .filter(|_| day.len() == 10)
                .ok_or_else(|| ApiError::bad_request("today must be a date YYYY-MM-DD")),
            None => Ok(tasks::validate::today()),
        }
    }
}

/// `POST /api/parse` reply.
#[derive(Serialize)]
struct Preview {
    title: String,
    tokens: Vec<TokenJson>,
    tags: Vec<String>,
    task: PreviewFields,
}

/// A token with UTF-16 offsets, so JavaScript can slice its strings with them.
#[derive(Serialize)]
struct TokenJson {
    start: usize,
    end: usize,
    kind: Kind,
    text: String,
    resolved: bool,
}

#[derive(Serialize)]
struct PreviewFields {
    scheduled: Option<String>,
    deadline: Option<String>,
    estimate: Option<u32>,
    priority: Option<u8>,
    recurrence: Option<Recurrence>,
    project_id: Option<i64>,
    area_id: Option<i64>,
}

async fn parse(
    State(db): State<Db>,
    _: Session,
    Json(body): Json<Body>,
) -> Result<Json<Preview>, ApiError> {
    let today = body.today()?;
    let text = body.text;
    let preview = blocking(move || {
        let line = line::read(&db, &text, today)?;
        let utf16 = |byte: usize| text.get(..byte).map_or(0, |s| s.encode_utf16().count());
        let parsed = line.parsed;
        let tokens = parsed
            .tokens
            .into_iter()
            .zip(line.resolved)
            .map(|(token, resolved)| TokenJson {
                start: utf16(token.start),
                end: utf16(token.end),
                kind: token.kind,
                text: token.text,
                resolved,
            })
            .collect();
        Ok::<_, ApiError>(Preview {
            title: parsed.title,
            tokens,
            tags: parsed.tags,
            task: PreviewFields {
                scheduled: parsed.scheduled,
                deadline: parsed.deadline,
                estimate: parsed.estimate_min,
                priority: parsed.priority,
                recurrence: parsed.recurrence,
                project_id: line.project_id,
                area_id: line.area_id,
            },
        })
    })
    .await?;
    Ok(Json(preview))
}

async fn add_task(
    State(db): State<Db>,
    _: Session,
    Json(body): Json<Body>,
) -> Result<(StatusCode, Json<Task>), ApiError> {
    let today = body.today()?;
    let text = body.text;
    let task = blocking(move || add(&db, &text, today)).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

/// Parses the smart-add line `text` as of `today` (the user's local date) and creates the
/// task through `tasks::store::create`. Blocking; shared with every capture channel.
///
/// With no project, area or date the task lands in the Inbox. A `400` (nothing created)
/// for: a line over [`line::MAX_TEXT_CHARS`], `unknown project: <name>` /
/// `unknown area: <name>`, an empty title after parsing, or a broken tag or task rule.
pub fn add(db: &Db, text: &str, today: NaiveDate) -> Result<Task, ApiError> {
    let line = line::read(db, text, today)?;
    if let Some((kind, name)) = line.unknown() {
        return Err(ApiError::bad_request(format!("unknown {kind}: {name}")));
    }
    let parsed = line.parsed;
    let new = New {
        title: object_rules::title(&parsed.title)?,
        tags: tags::normalize_all(&parsed.tags)?,
        task: FieldChanges {
            priority: parsed.priority.map(i64::from),
            scheduled: parsed.scheduled.map(Some),
            deadline: parsed.deadline.map(Some),
            estimate: parsed.estimate_min.map(|m| Some(i64::from(m))),
            project_id: line.project_id.map(Some),
            area_id: line.area_id.map(Some),
            recurrence: parsed.recurrence.map(|r| {
                Some(task_recurrence::Recurrence {
                    rule: r.rule,
                    from: match r.from {
                        Anchor::Scheduled => task_recurrence::Anchor::Scheduled,
                        Anchor::Completion => task_recurrence::Anchor::Completion,
                    },
                })
            }),
            ..FieldChanges::default()
        },
        ..New::default()
    };
    tasks::store::create(db, &new)
}
