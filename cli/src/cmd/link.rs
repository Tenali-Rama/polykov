//! `polykov link add|rm|ls`: links between objects, and backlinks.

use clap::Subcommand;
use serde_json::{Value, json};

use super::object::summary;
use super::{Ctx, Output};
use crate::error::Error;

#[derive(Subcommand)]
pub enum Cmd {
    /// Link object <ID> to object <TO>
    Add { id: u64, to: u64 },
    /// Remove the link from <ID> to <TO>
    Rm { id: u64, to: u64 },
    /// List an object's links and backlinks
    Ls { id: u64 },
}

pub fn run(ctx: &Ctx, cmd: Cmd) -> Result<Output, Error> {
    let client = ctx.client();
    match cmd {
        Cmd::Add { id, to } => {
            client
                .put(&format!("/api/objects/{id}/links/{to}"), None)?
                .ok()?;
            Ok(Output {
                human: format!("linked {id} -> {to}"),
                json: json!({ "id": id, "to": to, "linked": true }),
            })
        }
        Cmd::Rm { id, to } => {
            client
                .delete(&format!("/api/objects/{id}/links/{to}"))?
                .ok()?;
            Ok(Output {
                human: format!("unlinked {id} -> {to}"),
                json: json!({ "id": id, "to": to, "unlinked": true }),
            })
        }
        Cmd::Ls { id } => {
            let body = client.get(&format!("/api/objects/{id}/links"))?.ok()?;
            let human = format!(
                "{}\n{}",
                section("links", &body),
                section("backlinks", &body)
            );
            Ok(Output { human, json: body })
        }
    }
}

/// `<name>:` then one indented `<id>  <type>  <title>` line per object, or `  none`.
fn section(name: &str, body: &Value) -> String {
    let objects = body[name].as_array().map(Vec::as_slice).unwrap_or_default();
    let mut out = format!("{name}:");
    if objects.is_empty() {
        out.push_str("\n  none");
    }
    for object in objects {
        out.push_str("\n  ");
        out.push_str(&summary(object));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_lists_summaries_or_none() {
        let body = json!({"links": [{"id": 2, "type": "task", "title": "Copy"}], "backlinks": []});
        assert_eq!(section("links", &body), "links:\n  2  task  Copy");
        assert_eq!(section("backlinks", &body), "backlinks:\n  none");
    }
}
