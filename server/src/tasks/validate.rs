//! Input rules for task fields. The value rules are pure; the reference rules read the
//! database inside the caller's write transaction.

use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{Connection, OptionalExtension};

use super::store::ChecklistItem;
use crate::{db, http::ApiError};

/// Highest priority number; P4 means "no priority".
pub const MAX_PRIORITY: i64 = 4;
/// Largest estimate, in minutes (one week).
pub const MAX_ESTIMATE_MINUTES: i64 = 10080;
/// Most checklist items on one task.
pub const MAX_CHECKLIST_ITEMS: usize = 100;
/// Longest checklist item text after trimming, in characters.
pub const MAX_CHECKLIST_TEXT_CHARS: usize = 500;

/// The server's "today" (UTC), for comparing against floating task dates.
pub fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

/// Checks `priority` is 1 to [`MAX_PRIORITY`].
pub fn priority(priority: i64) -> Result<i64, ApiError> {
    if (1..=MAX_PRIORITY).contains(&priority) {
        Ok(priority)
    } else {
        Err(ApiError::bad_request(format!(
            "priority must be 1 to {MAX_PRIORITY}"
        )))
    }
}

/// Checks `value` is a real calendar date `YYYY-MM-DD` or a floating local time
/// `YYYY-MM-DDTHH:MM` (no time zone). `field` names it in the error.
pub fn date(field: &str, value: &str) -> Result<(), ApiError> {
    let digits_at = |positions: &[usize]| {
        let b = value.as_bytes();
        positions.iter().all(|&i| b[i].is_ascii_digit())
    };
    let shaped = match value.len() {
        10 => digits_at(&[0, 1, 2, 3, 5, 6, 8, 9]),
        16 => digits_at(&[0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15]),
        _ => false,
    };
    let parses = shaped
        && match value.len() {
            10 => NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok(),
            _ => NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").is_ok(),
        };
    if parses {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "{field} must be a date YYYY-MM-DD or a local time YYYY-MM-DDTHH:MM"
        )))
    }
}

/// Checks `minutes` is 1 to [`MAX_ESTIMATE_MINUTES`].
pub fn estimate(minutes: i64) -> Result<i64, ApiError> {
    if (1..=MAX_ESTIMATE_MINUTES).contains(&minutes) {
        Ok(minutes)
    } else {
        Err(ApiError::bad_request(format!(
            "estimate must be 1 to {MAX_ESTIMATE_MINUTES} minutes"
        )))
    }
}

/// Returns `items` with each text trimmed, if there are at most [`MAX_CHECKLIST_ITEMS`] and
/// each text is 1 to [`MAX_CHECKLIST_TEXT_CHARS`] characters.
pub fn checklist(items: &[ChecklistItem]) -> Result<Vec<ChecklistItem>, ApiError> {
    if items.len() > MAX_CHECKLIST_ITEMS {
        return Err(ApiError::bad_request(format!(
            "checklist must have at most {MAX_CHECKLIST_ITEMS} items"
        )));
    }
    items
        .iter()
        .map(|item| {
            let text = item.text.trim();
            let len = text.chars().count();
            if len == 0 || len > MAX_CHECKLIST_TEXT_CHARS {
                return Err(ApiError::bad_request(format!(
                    "checklist text must be 1 to {MAX_CHECKLIST_TEXT_CHARS} characters"
                )));
            }
            Ok(ChecklistItem {
                text: text.to_owned(),
                done: item.done,
            })
        })
        .collect()
}

/// Checks `parent` is a live task other than `id` that does not descend from `id`.
pub(super) fn parent(conn: &Connection, id: i64, parent: i64) -> Result<(), ApiError> {
    if live_type(conn, parent)?.as_deref() != Some("task") {
        return Err(ApiError::bad_request("parent must be a live task"));
    }
    // `parent` and its ancestors; `UNION` stops at a repeat, so this ends on any data.
    let cycle = conn
        .query_row(
            "WITH RECURSIVE up(id) AS (SELECT ?1 UNION SELECT t.parent_id FROM tasks t \
             JOIN up ON t.object_id = up.id WHERE t.parent_id IS NOT NULL) \
             SELECT 1 FROM up WHERE id = ?2",
            [parent, id],
            |_| Ok(()),
        )
        .optional()
        .map_err(db::Error::Sqlite)?
        .is_some();
    if cycle {
        return Err(ApiError::bad_request("a task cannot be its own ancestor"));
    }
    Ok(())
}

/// Checks `id` is a live object of type `kind` (`project` or `area`), naming `kind` in the
/// error.
pub(super) fn live(conn: &Connection, kind: &str, id: i64) -> Result<(), ApiError> {
    if live_type(conn, id)?.as_deref() == Some(kind) {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "{kind} must be a live {kind}"
        )))
    }
}

/// Checks `section` exists and belongs to `project`.
pub(super) fn section(
    conn: &Connection,
    section: i64,
    project: Option<i64>,
) -> Result<(), ApiError> {
    let owner: Option<i64> = conn
        .query_row(
            "SELECT project_id FROM sections WHERE id = ?1",
            [section],
            |r| r.get(0),
        )
        .optional()
        .map_err(db::Error::Sqlite)?;
    if owner.is_some() && owner == project {
        Ok(())
    } else {
        Err(ApiError::bad_request("section is not in this project"))
    }
}

fn live_type(conn: &Connection, id: i64) -> Result<Option<String>, ApiError> {
    Ok(conn
        .query_row(
            "SELECT type FROM objects WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(db::Error::Sqlite)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_must_be_real_and_zone_free() {
        for ok in [
            "2026-02-28",
            "2028-02-29",
            "2026-10-30T09:05",
            "2026-12-31T23:59",
        ] {
            assert!(date("deadline", ok).is_ok(), "{ok}");
        }
        for bad in [
            "2026-02-29",
            "2026-13-01",
            "2026-1-01",
            "+2026-01-01",
            "2026-10-30T24:00",
            "2026-10-30T09:05Z",
            "2026-10-30T09:05:00",
            "2026-10-30 09:05",
            "",
        ] {
            assert!(date("deadline", bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn today_is_a_plausible_date() {
        assert!(today() > NaiveDate::from_ymd_opt(2025, 1, 1).unwrap_or_default());
    }
}
