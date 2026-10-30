//! Task commands: `add` (smart add), the GTD lists (`today`, `inbox`, `upcoming`, `someday`,
//! `anytime`), `done` and `reopen`, and the task rendering they share.

use chrono::NaiveDate;
use serde_json::Value;

use super::object::text;

pub mod add;
pub mod done;
pub mod list;
pub mod render;

/// The user's local date, sent to the server as `today`.
fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// The date part of a task date (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`).
fn day(date: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date.get(..10)?, "%Y-%m-%d").ok()
}

/// The date a task is listed by: `scheduled`, else `deadline`.
fn listed_date(task: &Value) -> Option<&str> {
    task["task"]["scheduled"]
        .as_str()
        .or_else(|| task["task"]["deadline"].as_str())
}

/// One list row: `<id>  p<n>  <title> [open/total]  <date>  #tags`; the date is `!`-prefixed
/// when it is before `today`, and empty columns are left out.
fn row(task: &Value, today: NaiveDate) -> String {
    let mut cols = vec![
        task["id"].to_string(),
        format!("p{}", task["task"]["priority"]),
    ];
    let mut title = text(&task["title"]);
    let total = task["subtasks"]["total"].as_u64().unwrap_or_default();
    if total > 0 {
        title.push_str(&format!(" [{}/{total}]", task["subtasks"]["open"]));
    }
    cols.push(title);
    if let Some(date) = listed_date(task) {
        let overdue = if day(date).is_some_and(|d| d < today) {
            "!"
        } else {
            ""
        };
        cols.push(format!("{overdue}{}", date.replace('T', " ")));
    }
    let tags = render::tags(task);
    if !tags.is_empty() {
        cols.push(tags);
    }
    cols.join("  ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn row_marks_overdue_and_shows_subtasks_and_tags() {
        let t = json!({"id": 4, "title": "Pay rent", "tags": ["home"],
            "task": {"priority": 1, "scheduled": "2026-10-29T09:00", "deadline": null},
            "subtasks": {"open": 1, "total": 3}});
        assert_eq!(
            row(&t, date("2026-10-30")),
            "4  p1  Pay rent [1/3]  !2026-10-29 09:00  #home"
        );
    }

    #[test]
    fn row_falls_back_to_the_deadline_and_skips_empty_columns() {
        let t = json!({"id": 5, "title": "Ship", "tags": [],
            "task": {"priority": 4, "scheduled": null, "deadline": "2026-10-30"},
            "subtasks": {"open": 0, "total": 0}});
        assert_eq!(row(&t, date("2026-10-30")), "5  p4  Ship  2026-10-30");
        let t = json!({"id": 6, "title": "Idea", "tags": [],
            "task": {"priority": 3, "scheduled": null, "deadline": null},
            "subtasks": {"open": 0, "total": 0}});
        assert_eq!(row(&t, date("2026-10-30")), "6  p3  Idea");
    }
}
