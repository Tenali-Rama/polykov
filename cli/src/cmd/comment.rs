//! `polykov comment add|ls|edit|rm`: comments on objects.

use clap::Subcommand;
use serde_json::{Value, json};

use super::object::text;
use super::{Ctx, Output};
use crate::error::Error;

#[derive(Subcommand)]
pub enum Cmd {
    /// Comment on object <ID>; several words are joined with spaces
    Add {
        id: u64,
        #[arg(required = true)]
        text: Vec<String>,
    },
    /// List an object's comments, oldest first
    Ls { id: u64 },
    /// Replace the text of comment <CID>
    Edit {
        cid: u64,
        #[arg(required = true)]
        text: Vec<String>,
    },
    /// Delete comment <CID>
    Rm { cid: u64 },
}

pub fn run(ctx: &Ctx, cmd: Cmd) -> Result<Output, Error> {
    let client = ctx.client();
    match cmd {
        Cmd::Add { id, text } => {
            let body = json!({ "body": text.join(" ") });
            let path = format!("/api/objects/{id}/comments");
            let comment = client.post(&path, Some(&body))?.ok()?;
            Ok(Output {
                human: format!("added comment {}", comment["id"]),
                json: comment,
            })
        }
        Cmd::Ls { id } => {
            let body = client.get(&format!("/api/objects/{id}/comments"))?.ok()?;
            let comments = body["comments"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default();
            let human = if comments.is_empty() {
                "no comments".to_owned()
            } else {
                comments.iter().map(render).collect::<Vec<_>>().join("\n")
            };
            Ok(Output { human, json: body })
        }
        Cmd::Edit { cid, text } => {
            let body = json!({ "body": text.join(" ") });
            let comment = client.patch(&format!("/api/comments/{cid}"), &body)?.ok()?;
            Ok(Output {
                human: format!("updated comment {cid}"),
                json: comment,
            })
        }
        Cmd::Rm { cid } => {
            client.delete(&format!("/api/comments/{cid}"))?.ok()?;
            Ok(Output {
                human: format!("deleted comment {cid}"),
                json: json!({ "id": cid, "deleted": true }),
            })
        }
    }
}

/// `#<cid> <created>`, then the body with each line indented by two spaces.
fn render(comment: &Value) -> String {
    let mut out = format!("#{} {}", comment["id"], text(&comment["created"]));
    for line in text(&comment["body"]).lines() {
        out.push_str("\n  ");
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_indents_every_body_line() {
        let c = json!({"id": 4, "created": "2026-10-29T09:15:00Z", "body": "one\ntwo"});
        assert_eq!(render(&c), "#4 2026-10-29T09:15:00Z\n  one\n  two");
    }
}
