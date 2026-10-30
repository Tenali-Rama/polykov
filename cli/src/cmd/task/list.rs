//! `polykov today|inbox|upcoming|someday|anytime`: a GTD list from `GET /api/lists/<name>`.

use chrono::NaiveDate;
use serde_json::Value;

use super::{day, listed_date, row, today};
use crate::cmd::{Ctx, Output};
use crate::error::Error;

#[derive(clap::Args)]
pub struct Args {
    /// Only tasks in this project
    #[arg(long, value_name = "ID")]
    project: Option<u64>,
    /// Only tasks in this area or in its projects
    #[arg(long, value_name = "ID")]
    area: Option<u64>,
    /// At most this many tasks (server default 100)
    #[arg(long, value_name = "N")]
    limit: Option<u32>,
}

pub fn run(ctx: &Ctx, name: &str, args: Args) -> Result<Output, Error> {
    let today = today();
    let body = ctx.client().get(&query(name, today, &args))?.ok()?;
    let tasks = body["tasks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let total = body["total"].as_u64().unwrap_or_default();
    let mut human = if name == "upcoming" {
        grouped(tasks, today)
    } else {
        tasks.iter().map(|t| row(t, today)).collect()
    };
    if tasks.is_empty() {
        human.push(format!("nothing in {name}"));
    } else if (tasks.len() as u64) < total {
        human.push(format!("({} of {total} shown)", tasks.len()));
    }
    Ok(Output {
        human: human.join("\n"),
        json: body,
    })
}

/// `/api/lists/<name>` with `today` and the filters as query parameters.
fn query(name: &str, today: NaiveDate, args: &Args) -> String {
    let mut path = format!("/api/lists/{name}?today={}", today.format("%Y-%m-%d"));
    if let Some(project) = args.project {
        path.push_str(&format!("&project={project}"));
    }
    if let Some(area) = args.area {
        path.push_str(&format!("&area={area}"));
    }
    if let Some(limit) = args.limit {
        path.push_str(&format!("&limit={limit}"));
    }
    path
}

/// Rows under a `YYYY-MM-DD` header per listed date, indented two spaces. The server sorts
/// `upcoming` by that date, so each date is one run.
fn grouped(tasks: &[Value], today: NaiveDate) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = None;
    for task in tasks {
        let date = listed_date(task).and_then(day);
        if lines.is_empty() || date != current {
            lines.push(date.map_or_else(|| "no date".to_owned(), |d| d.to_string()));
            current = date;
        }
        lines.push(format!("  {}", row(task, today)));
    }
    lines
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn task(id: u64, scheduled: &str) -> Value {
        json!({"id": id, "title": format!("t{id}"), "tags": [],
            "task": {"priority": 4, "scheduled": scheduled, "deadline": null},
            "subtasks": {"open": 0, "total": 0}})
    }

    #[test]
    fn upcoming_groups_rows_under_date_headers() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let tasks = [
            task(1, "2026-10-31"),
            task(2, "2026-10-31T09:00"),
            task(3, "2026-11-02"),
        ];
        assert_eq!(
            grouped(&tasks, today),
            [
                "2026-10-31",
                "  1  p4  t1  2026-10-31",
                "  2  p4  t2  2026-10-31 09:00",
                "2026-11-02",
                "  3  p4  t3  2026-11-02",
            ]
        );
    }

    #[test]
    fn query_sends_today_and_the_filters() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let args = Args {
            project: Some(2),
            area: None,
            limit: Some(5),
        };
        assert_eq!(
            query("today", today, &args),
            "/api/lists/today?today=2026-10-30&project=2&limit=5"
        );
    }
}
