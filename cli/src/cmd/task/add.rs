//! `polykov add <text>...`: smart add through `POST /api/tasks/add`.

use serde_json::json;

use super::{render, today};
use crate::cmd::object::text;
use crate::cmd::{Ctx, Output};
use crate::error::Error;

pub fn run(ctx: &Ctx, words: Vec<String>) -> Result<Output, Error> {
    let today = today();
    let body = json!({ "text": words.join(" "), "today": today.format("%Y-%m-%d").to_string() });
    let reply = ctx.client().post("/api/tasks/add", Some(&body))?;
    if reply.status == 400 {
        let message = reply.body["error"]
            .as_str()
            .unwrap_or("server returned 400");
        return Err(Error::Msg(message.to_owned()));
    }
    let task = reply.ok()?;
    Ok(Output {
        human: format!(
            "added {}  {}\n{}",
            task["id"],
            text(&task["title"]),
            render::summary(&task, today)
        ),
        json: task,
    })
}
