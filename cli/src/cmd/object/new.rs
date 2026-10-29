//! `polykov new <type> <title>...`: create an object.

use serde_json::{Map, Value, json};

use super::{output, parse_prop};
use crate::cmd::{Ctx, Output};
use crate::error::Error;

#[derive(clap::Args)]
pub struct Args {
    /// Object type (task, note, project, ...)
    #[arg(value_name = "TYPE")]
    kind: String,
    /// Title; several words are joined with spaces
    #[arg(required = true)]
    title: Vec<String>,
    /// Add a tag (repeatable)
    #[arg(long, value_name = "T")]
    tag: Vec<String>,
    /// Set a property; the value is a number, true/false or a string (repeatable)
    #[arg(long, value_name = "K=V", value_parser = parse_prop)]
    prop: Vec<(String, Value)>,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<Output, Error> {
    let properties: Map<String, Value> = args.prop.into_iter().collect();
    let body = json!({
        "type": args.kind,
        "title": args.title.join(" "),
        "tags": args.tag,
        "properties": properties,
    });
    let object = ctx.client().post("/api/objects", Some(&body))?.ok()?;
    Ok(output(format!("created {}", object["id"]), object))
}
