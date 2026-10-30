//! `polykov done <id>...` and `polykov reopen <id>`.

use serde_json::json;

use super::{listed_date, today};
use crate::cmd::{Ctx, Output};
use crate::error::Error;

/// Completes each task in order (`POST /api/tasks/{id}/complete?today=`); stops at the first
/// failure. Prints `done <id>`, plus `next <id>  <date>` for a recurring task.
pub fn run(ctx: &Ctx, ids: Vec<u64>) -> Result<Output, Error> {
    let client = ctx.client();
    let today = today().format("%Y-%m-%d");
    let mut human = Vec::new();
    let mut done = Vec::new();
    for id in ids {
        let path = format!("/api/tasks/{id}/complete?today={today}");
        let body = client.post(&path, None)?.ok()?;
        human.push(format!("done {id}"));
        let next = &body["next"];
        if !next.is_null() {
            let date = listed_date(next).unwrap_or_default().replace('T', " ");
            human.push(format!("next {}  {date}", next["id"]));
        }
        done.push(body);
    }
    Ok(Output {
        human: human.join("\n"),
        json: json!({ "done": done }),
    })
}

/// `POST /api/tasks/{id}/reopen`. Prints `reopened <id>`.
pub fn reopen(ctx: &Ctx, id: u64) -> Result<Output, Error> {
    let task = ctx
        .client()
        .post(&format!("/api/tasks/{id}/reopen"), None)?
        .ok()?;
    Ok(Output {
        human: format!("reopened {id}"),
        json: task,
    })
}
