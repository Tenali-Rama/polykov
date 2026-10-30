//! One smart-add line, parsed by `polykov_parse::default_parser()` against the live,
//! non-archived projects and areas, with their names resolved to ids.

use chrono::NaiveDate;
use polykov_parse::{Context, Kind, Parsed, Parser, RulesParser};

use crate::{
    db::{self, Db},
    http::ApiError,
};

/// Longest smart-add line, in characters.
pub const MAX_TEXT_CHARS: usize = 1000;

thread_local! {
    /// One parser per (blocking-pool) thread, so the model is read once, not per request.
    static PARSER: Box<dyn Parser> = polykov_parse::default_parser();
}

/// A parsed line. `resolved[i]` is false only when `parsed.tokens[i]` is a `+project` or
/// `@area` that names no live project or area.
#[derive(Debug)]
pub struct Line {
    pub parsed: Parsed,
    pub resolved: Vec<bool>,
    pub project_id: Option<i64>,
    pub area_id: Option<i64>,
}

impl Line {
    /// The first `+project`/`@area` token that names nothing, as `(kind, name)`.
    pub fn unknown(&self) -> Option<(&'static str, &str)> {
        let (token, _) = self
            .parsed
            .tokens
            .iter()
            .zip(&self.resolved)
            .find(|(_, ok)| !**ok)?;
        let kind = match token.kind {
            Kind::Area => "area",
            _ => "project",
        };
        Some((kind, token.text.get(1..).unwrap_or_default()))
    }
}

/// Parses `text` as of `today`. A line over [`MAX_TEXT_CHARS`] is a `400`.
pub fn read(db: &Db, text: &str, today: NaiveDate) -> Result<Line, ApiError> {
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(ApiError::bad_request(format!(
            "text must be at most {MAX_TEXT_CHARS} characters"
        )));
    }
    let (projects, areas) = known(db)?;
    let ctx = Context {
        today,
        projects: projects.iter().map(|(title, _)| title.clone()).collect(),
        areas: areas.iter().map(|(title, _)| title.clone()).collect(),
    };
    let parsed = PARSER.with(|parser| parser.parse(text, &ctx));
    // The rules matcher on the token alone says whether its name is known.
    let resolved = parsed
        .tokens
        .iter()
        .map(|token| match token.kind {
            Kind::Project => RulesParser.parse(&token.text, &ctx).project.is_some(),
            Kind::Area => RulesParser.parse(&token.text, &ctx).area.is_some(),
            _ => true,
        })
        .collect();
    let id_of = |known: &[(String, i64)], name: &Option<String>| {
        let name = name.as_deref()?;
        known
            .iter()
            .find(|(title, _)| title == name)
            .map(|&(_, id)| id)
    };
    Ok(Line {
        project_id: id_of(&projects, &parsed.project),
        area_id: id_of(&areas, &parsed.area),
        resolved,
        parsed,
    })
}

type Named = Vec<(String, i64)>;

/// `(title, id)` of the live, non-archived projects and of the areas, each by id.
fn known(db: &Db) -> Result<(Named, Named), db::Error> {
    let conn = db.conn()?;
    let mut stmt = conn
        .prepare(
            "SELECT type, title, id FROM objects WHERE type IN ('project', 'area') \
             AND deleted_at IS NULL AND archived = 0 ORDER BY id",
        )
        .map_err(db::Error::Sqlite)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get(2)?))
        })
        .map_err(db::Error::Sqlite)?;
    let (mut projects, mut areas) = (Vec::new(), Vec::new());
    for row in rows {
        let (kind, title, id) = row.map_err(db::Error::Sqlite)?;
        let list = if kind == "area" {
            &mut areas
        } else {
            &mut projects
        };
        list.push((title, id));
    }
    Ok((projects, areas))
}
