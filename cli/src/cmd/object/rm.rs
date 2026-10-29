//! `polykov rm <id>`: move an object to the trash.

use serde_json::json;

use super::path;
use crate::cmd::{Ctx, Output};
use crate::error::Error;

pub fn run(ctx: &Ctx, id: u64) -> Result<Output, Error> {
    ctx.client().delete(&path(id))?.ok()?;
    Ok(Output {
        human: format!("trashed {id}"),
        json: json!({ "id": id, "trashed": true }),
    })
}
