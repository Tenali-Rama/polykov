//! `polykov edit <id>`: change the title, tags and properties of an object.

use serde_json::{Map, Value};

use super::{output, parse_prop, path};
use crate::cmd::{Ctx, Output};
use crate::error::Error;

#[derive(clap::Args)]
pub struct Args {
    /// Object id
    id: u64,
    /// New title
    #[arg(long, value_name = "T")]
    title: Option<String>,
    /// Add a tag (repeatable)
    #[arg(long, value_name = "T")]
    add_tag: Vec<String>,
    /// Remove a tag (repeatable)
    #[arg(long, value_name = "T")]
    rm_tag: Vec<String>,
    /// Set a property; the value is a number, true/false or a string (repeatable)
    #[arg(long, value_name = "K=V", value_parser = parse_prop)]
    prop: Vec<(String, Value)>,
    /// Remove a property (repeatable)
    #[arg(long, value_name = "K")]
    unset: Vec<String>,
}

/// GETs the object, merges the changes into its tags and properties, then sends one PATCH
/// with only the fields that were asked to change.
pub fn run(ctx: &Ctx, args: Args) -> Result<Output, Error> {
    let client = ctx.client();
    let current = client.get(&path(args.id))?.ok()?;
    let mut changes = Map::new();
    if let Some(title) = args.title {
        changes.insert("title".to_owned(), Value::String(title));
    }
    if !args.add_tag.is_empty() || !args.rm_tag.is_empty() {
        let removed: Vec<String> = args.rm_tag.iter().map(|t| normalize(t)).collect();
        let mut tags: Vec<Value> = current["tags"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|t| !removed.iter().any(|r| t == r))
            .collect();
        tags.extend(args.add_tag.into_iter().map(Value::String));
        changes.insert("tags".to_owned(), Value::Array(tags));
    }
    if !args.prop.is_empty() || !args.unset.is_empty() {
        let mut props = current["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        for key in &args.unset {
            props.remove(key);
        }
        props.extend(args.prop);
        changes.insert("properties".to_owned(), Value::Object(props));
    }
    let object = client
        .patch(&path(args.id), &Value::Object(changes))?
        .ok()?;
    Ok(output(format!("updated {}", args.id), object))
}

/// A tag as the server stores it (trimmed, one leading `#` dropped, lowercased), so
/// `--rm-tag '#Work'` removes `work`.
fn normalize(tag: &str) -> String {
    let tag = tag.trim();
    tag.strip_prefix('#').unwrap_or(tag).to_lowercase()
}
