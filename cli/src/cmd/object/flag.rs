//! `polykov pin|unpin|archive|unarchive <id>`: set or clear one flag of an object.

use serde_json::json;

use super::{output, path};
use crate::cmd::{Ctx, Output};
use crate::error::Error;

#[derive(Clone, Copy)]
pub enum Flag {
    Pinned,
    Archived,
}

/// PATCHes the flag to `on`; prints e.g. `pinned 3` or `unarchived 3`.
pub fn run(ctx: &Ctx, id: u64, flag: Flag, on: bool) -> Result<Output, Error> {
    let field = match flag {
        Flag::Pinned => "pinned",
        Flag::Archived => "archived",
    };
    let object = ctx.client().patch(&path(id), &json!({ field: on }))?.ok()?;
    let prefix = if on { "" } else { "un" };
    Ok(output(format!("{prefix}{field} {id}"), object))
}
