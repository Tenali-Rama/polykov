//! Input rules for object fields, checked at the API boundary.

use serde_json::{Map, Value};

use crate::http::ApiError;

/// Every object type (spec §3).
pub const TYPES: &[&str] = &[
    "task",
    "project",
    "area",
    "goal",
    "milestone",
    "habit",
    "note",
    "notebook",
    "journal_entry",
    "daily_note",
    "review",
    "event",
    "time_entry",
    "view",
    "forecast_record",
];

/// Maximum title length after trimming, in characters.
pub const MAX_TITLE_CHARS: usize = 1000;
/// Maximum number of keys in `properties`.
pub const MAX_PROPERTIES: usize = 100;
/// Maximum property name length, in characters.
pub const MAX_KEY_CHARS: usize = 64;

/// Checks `kind` is one of [`TYPES`].
pub fn object_type(kind: &str) -> Result<(), ApiError> {
    if TYPES.contains(&kind) {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "type must be one of: {}",
            TYPES.join(", ")
        )))
    }
}

/// Returns the trimmed title if it is 1 to [`MAX_TITLE_CHARS`] characters.
pub fn title(title: &str) -> Result<String, ApiError> {
    let title = title.trim();
    let len = title.chars().count();
    if len == 0 || len > MAX_TITLE_CHARS {
        return Err(ApiError::bad_request(format!(
            "title must be 1 to {MAX_TITLE_CHARS} characters"
        )));
    }
    Ok(title.to_owned())
}

/// Returns `value` as a property map if it is an object of at most [`MAX_PROPERTIES`] keys,
/// each 1 to [`MAX_KEY_CHARS`] characters, whose values are scalars (string, number, bool,
/// null) or arrays of scalars.
pub fn properties(value: Value) -> Result<Map<String, Value>, ApiError> {
    let Value::Object(map) = value else {
        return Err(ApiError::bad_request("properties must be an object"));
    };
    if map.len() > MAX_PROPERTIES {
        return Err(ApiError::bad_request(format!(
            "properties must have at most {MAX_PROPERTIES} keys"
        )));
    }
    for (key, value) in &map {
        let len = key.chars().count();
        if len == 0 || len > MAX_KEY_CHARS {
            return Err(ApiError::bad_request(format!(
                "property names must be 1 to {MAX_KEY_CHARS} characters"
            )));
        }
        let ok = match value {
            Value::Array(items) => items.iter().all(is_scalar),
            other => is_scalar(other),
        };
        if !ok {
            return Err(ApiError::bad_request(format!(
                "property {key:?} must be a string, number, bool, null or an array of those"
            )));
        }
    }
    Ok(map)
}

fn is_scalar(value: &Value) -> bool {
    !matches!(value, Value::Array(_) | Value::Object(_))
}
