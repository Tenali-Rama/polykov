//! `polykov trash ls|restore|purge|empty`: objects deleted with `polykov rm`.

use clap::Subcommand;
use serde_json::{Value, json};

use super::object::{summary, text};
use super::{Ctx, Output};
use crate::error::Error;

#[derive(Subcommand)]
pub enum Cmd {
    /// List trashed objects, newest deleted first
    Ls,
    /// Take an object out of the trash
    Restore { id: u64 },
    /// Delete one trashed object for good
    Purge { id: u64 },
    /// Delete every trashed object for good (needs --yes)
    Empty {
        /// Confirm deleting everything in the trash
        #[arg(long, required = true)]
        yes: bool,
    },
}

pub fn run(ctx: &Ctx, cmd: Cmd) -> Result<Output, Error> {
    let client = ctx.client();
    match cmd {
        Cmd::Ls => {
            let body = client.get("/api/trash")?.ok()?;
            let objects = body["objects"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default();
            let human = if objects.is_empty() {
                "trash is empty".to_owned()
            } else {
                objects.iter().map(line).collect::<Vec<_>>().join("\n")
            };
            Ok(Output { human, json: body })
        }
        Cmd::Restore { id } => {
            let object = client
                .post(&format!("/api/trash/{id}/restore"), None)?
                .ok()?;
            Ok(Output {
                human: format!("restored {id}"),
                json: object,
            })
        }
        Cmd::Purge { id } => {
            client.delete(&format!("/api/trash/{id}"))?.ok()?;
            Ok(Output {
                human: format!("purged {id}"),
                json: json!({ "id": id, "purged": true }),
            })
        }
        Cmd::Empty { yes: _ } => {
            let body = client.delete("/api/trash")?.ok()?;
            Ok(Output {
                human: format!("purged {}", body["purged"]),
                json: body,
            })
        }
    }
}

/// `<id>  <type>  <title>  deleted <date>  purge <date>`, dates as `YYYY-MM-DD`.
fn line(object: &Value) -> String {
    format!(
        "{}  deleted {}  purge {}",
        summary(object),
        date(&object["deleted"]),
        date(&object["purge_at"])
    )
}

/// The date part of an RFC 3339 timestamp.
fn date(value: &Value) -> String {
    let stamp = text(value);
    stamp.split('T').next().unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_shows_deleted_and_purge_dates() {
        let o = json!({"id": 3, "type": "note", "title": "Old",
            "deleted": "2026-10-29T09:15:00Z", "purge_at": "2026-11-28T09:15:00Z"});
        assert_eq!(
            line(&o),
            "3  note  Old  deleted 2026-10-29  purge 2026-11-28"
        );
    }
}
