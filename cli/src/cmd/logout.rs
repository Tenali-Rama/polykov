//! `polykov logout`: end the session and forget its token.

use serde_json::json;

use super::{Ctx, Output};
use crate::config::CliConfig;
use crate::error::Error;

pub fn run(ctx: &Ctx) -> Result<Output, Error> {
    if ctx.token().is_none() {
        return Err(Error::Msg(format!(
            "not logged in to {}; run polykov login",
            ctx.server
        )));
    }
    let result = ctx.client().post("/api/logout", None).and_then(|r| r.ok());
    if let Ok(_) | Err(Error::Unauthorized) = result {
        // The token is dead either way, so forget it.
        CliConfig {
            server: ctx.config.server.clone(),
            token: None,
        }
        .save(&ctx.config_path)?;
    }
    result?;
    Ok(Output {
        human: format!("logged out of {}", ctx.server),
        json: json!({ "server": ctx.server, "logged_out": true }),
    })
}
