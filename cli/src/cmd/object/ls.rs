//! `polykov ls`: list objects, filtered by the server.

use serde_json::json;

use super::line;
use crate::cmd::{Ctx, Output};
use crate::error::Error;

#[derive(clap::Args)]
pub struct Args {
    /// Only objects of this type
    #[arg(long = "type", value_name = "T")]
    kind: Option<String>,
    /// Only objects with this tag or a tag under it
    #[arg(long, value_name = "T")]
    tag: Option<String>,
    /// Only pinned objects
    #[arg(long)]
    pinned: bool,
    /// Only archived objects
    #[arg(long, conflicts_with = "all")]
    archived: bool,
    /// Archived and unarchived objects
    #[arg(long)]
    all: bool,
    /// At most this many objects (server default 100)
    #[arg(long, value_name = "N")]
    limit: Option<u32>,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<Output, Error> {
    let body = ctx.client().get(&query(&args))?.ok()?;
    let objects = body["objects"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let total = body["total"].as_u64().unwrap_or_default();
    let mut human: Vec<String> = objects.iter().map(line).collect();
    if objects.is_empty() {
        human.push("no objects".to_owned());
    } else if (objects.len() as u64) < total {
        human.push(format!("({} of {total} shown)", objects.len()));
    }
    Ok(Output {
        human: human.join("\n"),
        json: json!({ "objects": objects, "total": total }),
    })
}

/// `/api/objects` with the filters as query parameters.
fn query(args: &Args) -> String {
    let mut params = Vec::new();
    if let Some(kind) = &args.kind {
        params.push(format!("type={}", encode(kind)));
    }
    if let Some(tag) = &args.tag {
        params.push(format!("tag={}", encode(tag)));
    }
    if args.pinned {
        params.push("pinned=true".to_owned());
    }
    if args.archived {
        params.push("archived=true".to_owned());
    }
    if args.all {
        params.push("archived=any".to_owned());
    }
    if let Some(limit) = args.limit {
        params.push(format!("limit={limit}"));
    }
    if params.is_empty() {
        "/api/objects".to_owned()
    } else {
        format!("/api/objects?{}", params.join("&"))
    }
}

/// Percent-encodes everything but unreserved URL characters.
pub fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_escapes_reserved_and_non_ascii_bytes() {
        assert_eq!(encode("work/a-b_c.d~"), "work%2Fa-b_c.d~");
        assert_eq!(encode("a&b=c?"), "a%26b%3Dc%3F");
        assert_eq!(encode("é"), "%C3%A9");
    }
}
