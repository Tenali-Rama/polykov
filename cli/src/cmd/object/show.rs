//! `polykov show <id>`: print one object.

use super::{detail, output, path};
use crate::cmd::{Ctx, Output};
use crate::error::Error;

pub fn run(ctx: &Ctx, id: u64) -> Result<Output, Error> {
    let object = ctx.client().get(&path(id))?.ok()?;
    Ok(output(detail(&object), object))
}
