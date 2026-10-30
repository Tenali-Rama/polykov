//! The parsed-fields summary `polykov add` prints, e.g.
//! `tomorrow 17:00 · 30m · p2 · #family · every sunday`.

use chrono::NaiveDate;
use serde_json::Value;

use super::day;
use crate::cmd::object::text;

/// Weekday codes of `BYDAY` and their names, Monday first.
const DAYS: [(&str, &str); 7] = [
    ("MO", "monday"),
    ("TU", "tuesday"),
    ("WE", "wednesday"),
    ("TH", "thursday"),
    ("FR", "friday"),
    ("SA", "saturday"),
    ("SU", "sunday"),
];

/// `#a #b` for a task's tags, empty if it has none.
pub fn tags(task: &Value) -> String {
    let tags = task["tags"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    tags.iter()
        .map(|t| format!("#{}", text(t)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The fields smart add set, joined with ` · `: scheduled, `due` deadline, estimate, priority
/// (unless the default p4), project, area, tags and recurrence. `inbox` when there are none.
pub fn summary(task: &Value, today: NaiveDate) -> String {
    let fields = &task["task"];
    let mut parts = Vec::new();
    if let Some(date) = fields["scheduled"].as_str() {
        parts.push(when(date, today));
    }
    if let Some(date) = fields["deadline"].as_str() {
        parts.push(format!("due {}", when(date, today)));
    }
    if let Some(minutes) = fields["estimate"].as_u64() {
        parts.push(duration(minutes));
    }
    if fields["priority"].as_u64().is_some_and(|p| p != 4) {
        parts.push(format!("p{}", fields["priority"]));
    }
    if !fields["project_id"].is_null() {
        parts.push(format!("project {}", fields["project_id"]));
    }
    if !fields["area_id"].is_null() {
        parts.push(format!("area {}", fields["area_id"]));
    }
    let tags = tags(task);
    if !tags.is_empty() {
        parts.push(tags);
    }
    if let Some(rule) = fields["recurrence"]["rule"].as_str() {
        let mut every = recurrence(rule);
        if fields["recurrence"]["from"] == "completion" {
            every.push_str(" after completion");
        }
        parts.push(every);
    }
    if parts.is_empty() {
        "inbox".to_owned()
    } else {
        parts.join(" · ")
    }
}

/// A task date relative to `today`: `today`, `tomorrow`, `yesterday` or `YYYY-MM-DD`, then
/// the time if it has one.
fn when(date: &str, today: NaiveDate) -> String {
    let (date_part, time) = match date.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (date, None),
    };
    let mut out = match day(date).map(|d| (d - today).num_days()) {
        Some(0) => "today".to_owned(),
        Some(1) => "tomorrow".to_owned(),
        Some(-1) => "yesterday".to_owned(),
        _ => date_part.to_owned(),
    };
    if let Some(time) = time {
        out.push(' ');
        out.push_str(time);
    }
    out
}

/// Minutes as `45m`, `2h` or `1h30m`.
fn duration(minutes: u64) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h{m}m"),
    }
}

/// An RRULE of the 027 subset in words: `every day`, `every 2 weeks`, `every weekday`,
/// `every monday, friday`, `every 15th`. Unknown parts are ignored.
fn recurrence(rule: &str) -> String {
    let (mut unit, mut interval, mut by_day, mut month_day) = ("day", 1, None, None);
    for part in rule.split(';') {
        match part.split_once('=') {
            Some(("FREQ", "WEEKLY")) => unit = "week",
            Some(("FREQ", "MONTHLY")) => unit = "month",
            Some(("FREQ", "YEARLY")) => unit = "year",
            Some(("INTERVAL", n)) => interval = n.parse().unwrap_or(1),
            Some(("BYDAY", days)) => by_day = Some(days),
            Some(("BYMONTHDAY", d)) => month_day = d.parse::<u32>().ok(),
            _ => {}
        }
    }
    let every = if interval == 1 {
        format!("every {unit}")
    } else {
        format!("every {interval} {unit}s")
    };
    if let Some(codes) = by_day {
        let names = if codes == "MO,TU,WE,TH,FR" {
            "weekday".to_owned()
        } else {
            let names: Vec<&str> = codes
                .split(',')
                .filter_map(|c| DAYS.iter().find(|(code, _)| *code == c))
                .map(|(_, name)| *name)
                .collect();
            names.join(", ")
        };
        return if interval == 1 {
            format!("every {names}")
        } else {
            format!("{every} on {names}")
        };
    }
    if let Some(d) = month_day {
        return if interval == 1 {
            format!("every {}", ordinal(d))
        } else {
            format!("{every} on the {}", ordinal(d))
        };
    }
    every
}

/// `1st`, `2nd`, `3rd`, `4th`, `11th`, `21st`...
fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn summary_of_the_spec_example() {
        let t = json!({"tags": ["family"], "task": {"scheduled": "2026-10-31T17:00",
            "deadline": null, "estimate": 30, "priority": 2, "project_id": null, "area_id": null,
            "recurrence": {"rule": "FREQ=WEEKLY;BYDAY=SU", "from": "scheduled"}}});
        assert_eq!(
            summary(&t, date("2026-10-30")),
            "tomorrow 17:00 · 30m · p2 · #family · every sunday"
        );
    }

    #[test]
    fn summary_is_inbox_without_parsed_fields() {
        let t = json!({"tags": [], "task": {"scheduled": null, "deadline": null,
            "estimate": null, "priority": 4, "project_id": null, "area_id": null,
            "recurrence": null}});
        assert_eq!(summary(&t, date("2026-10-30")), "inbox");
    }

    #[test]
    fn summary_shows_deadline_project_area_and_completion_anchor() {
        let t = json!({"tags": [], "task": {"scheduled": "2026-11-12", "deadline": "2026-10-30",
            "estimate": 90, "priority": 4, "project_id": 2, "area_id": 3,
            "recurrence": {"rule": "FREQ=DAILY;INTERVAL=3", "from": "completion"}}});
        assert_eq!(
            summary(&t, date("2026-10-30")),
            "2026-11-12 · due today · 1h30m · project 2 · area 3 · every 3 days after completion"
        );
    }

    #[test]
    fn recurrence_in_words() {
        assert_eq!(recurrence("FREQ=DAILY"), "every day");
        assert_eq!(recurrence("FREQ=YEARLY"), "every year");
        assert_eq!(
            recurrence("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR"),
            "every weekday"
        );
        assert_eq!(
            recurrence("FREQ=WEEKLY;BYDAY=MO,FR"),
            "every monday, friday"
        );
        assert_eq!(
            recurrence("FREQ=WEEKLY;INTERVAL=2;BYDAY=TU"),
            "every 2 weeks on tuesday"
        );
        assert_eq!(recurrence("FREQ=MONTHLY;BYMONTHDAY=15"), "every 15th");
        assert_eq!(
            recurrence("FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=1"),
            "every 2 months on the 1st"
        );
    }

    #[test]
    fn ordinals_and_durations() {
        let ords: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 22, 31]
            .into_iter()
            .map(ordinal)
            .collect();
        assert_eq!(
            ords,
            [
                "1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "31st"
            ]
        );
        assert_eq!(duration(45), "45m");
        assert_eq!(duration(120), "2h");
        assert_eq!(duration(90), "1h30m");
    }
}
