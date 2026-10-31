//! A view's settings, stored in its object `properties` (ADR 0016): `query`, `layout`, `group`
//! and `sort`, and the rules each must meet.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value, json};

use crate::query::{self, MAX_QUERY_CHARS, Query};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    List,
    Kanban,
    Eisenhower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    None,
    Status,
    Priority,
    Project,
    Section,
    Area,
    Tag,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Smart,
    Priority,
    Deadline,
    Scheduled,
    Created,
    Updated,
    Completed,
    Title,
}

pub const LAYOUTS: [(&str, Layout); 3] = [
    ("list", Layout::List),
    ("kanban", Layout::Kanban),
    ("eisenhower", Layout::Eisenhower),
];

pub const GROUPS: [(&str, Group); 8] = [
    ("none", Group::None),
    ("status", Group::Status),
    ("priority", Group::Priority),
    ("project", Group::Project),
    ("section", Group::Section),
    ("area", Group::Area),
    ("tag", Group::Tag),
    ("type", Group::Type),
];

pub const SORTS: [(&str, Sort); 8] = [
    ("smart", Sort::Smart),
    ("priority", Sort::Priority),
    ("deadline", Sort::Deadline),
    ("scheduled", Sort::Scheduled),
    ("created", Sort::Created),
    ("updated", Sort::Updated),
    ("completed", Sort::Completed),
    ("title", Sort::Title),
];

/// The settings of a valid view.
#[derive(Debug)]
pub struct Spec {
    pub query: Query,
    pub layout: Layout,
    pub group: Group,
    pub sort: Sort,
}

/// Why a view setting is invalid; `at` is the char offset of a query parse error.
#[derive(Debug, PartialEq, Eq)]
pub struct Invalid {
    pub message: String,
    pub at: Option<usize>,
}

impl Invalid {
    fn new(message: String) -> Self {
        Invalid { message, at: None }
    }

    /// `status {"error": message}`, plus `"at"` for a query parse error.
    pub fn respond(self, status: StatusCode) -> Response {
        let body = match self.at {
            Some(at) => json!({ "error": self.message, "at": at }),
            None => json!({ "error": self.message }),
        };
        (status, Json(body)).into_response()
    }
}

/// Checks a view query: at most `MAX_QUERY_CHARS` characters and parsing.
pub fn query(text: &str) -> Result<Query, Invalid> {
    if text.chars().count() > MAX_QUERY_CHARS {
        return Err(Invalid::new(format!(
            "query must be at most {MAX_QUERY_CHARS} characters"
        )));
    }
    query::parse(text).map_err(|e| Invalid {
        message: format!("query: {}", e.message),
        at: Some(e.offset),
    })
}

/// The option named `value` among `options`, or an error naming `field` and the allowed values.
pub fn pick<T: Copy>(field: &str, value: &str, options: &[(&str, T)]) -> Result<T, Invalid> {
    options
        .iter()
        .find(|(name, _)| *name == value)
        .map(|&(_, option)| option)
        .ok_or_else(|| {
            let names: Vec<&str> = options.iter().map(|(name, _)| *name).collect();
            Invalid::new(format!("{field} must be one of: {}", names.join(", ")))
        })
}

/// Reads and checks the settings stored in a view's `properties`.
pub fn read(properties: &Map<String, Value>) -> Result<Spec, Invalid> {
    let text = |field: &str| {
        properties
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| Invalid::new(format!("{field} must be a string")))
    };
    Ok(Spec {
        query: query(text("query")?)?,
        layout: pick("layout", text("layout")?, &LAYOUTS)?,
        group: pick("group", text("group")?, &GROUPS)?,
        sort: pick("sort", text("sort")?, &SORTS)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn reads_valid_settings() {
        let spec = read(&props(json!({
            "query": "type:task", "layout": "kanban", "group": "tag", "sort": "title"
        })))
        .unwrap();
        assert_eq!(spec.layout, Layout::Kanban);
        assert_eq!(spec.group, Group::Tag);
        assert_eq!(spec.sort, Sort::Title);
        assert_eq!(spec.query.terms.len(), 1);
    }

    #[test]
    fn names_the_bad_field() {
        let base = json!({ "query": "", "layout": "list", "group": "none", "sort": "smart" });
        let mut missing = props(base.clone());
        missing.remove("group");
        assert_eq!(
            read(&missing).unwrap_err().message,
            "group must be a string"
        );
        let mut bad = props(base);
        bad.insert("sort".into(), json!("random"));
        assert_eq!(
            read(&bad).unwrap_err().message,
            "sort must be one of: smart, priority, deadline, scheduled, created, updated, \
             completed, title"
        );
    }

    #[test]
    fn query_errors_carry_the_offset() {
        let err = query("type:task bogus:1").unwrap_err();
        assert_eq!(err.at, Some(10));
        assert!(err.message.starts_with("query: "), "{}", err.message);
        let long = "a".repeat(MAX_QUERY_CHARS + 1);
        assert_eq!(query(&long).unwrap_err().at, None);
    }
}
