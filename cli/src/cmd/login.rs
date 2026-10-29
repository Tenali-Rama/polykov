//! `polykov login`: start a session and store its token.

use serde_json::json;

use super::{Ctx, Output};
use crate::client::Client;
use crate::config::{self, CliConfig};
use crate::error::Error;

pub fn run(ctx: &Ctx) -> Result<Output, Error> {
    let password = match config::env("POLYKOV_PASSWORD") {
        Some(password) => password,
        None => rpassword::prompt_password("password: ")
            .map_err(|e| Error::Msg(format!("cannot read password: {e}")))?,
    };
    let reply = Client::new(&ctx.server, None)
        .post("/api/login", Some(&json!({ "password": password })))?;
    if reply.status == 401 {
        return Err(Error::Msg(
            "login failed: wrong password, or the owner is not set up yet".into(),
        ));
    }
    let body = reply.ok()?;
    let token = body["token"]
        .as_str()
        .ok_or_else(|| Error::Msg("login failed: server sent no token".into()))?;
    CliConfig {
        server: Some(ctx.server.clone()),
        token: Some(token.to_owned()),
    }
    .save(&ctx.config_path)?;
    Ok(Output {
        human: format!("logged in to {}", ctx.server),
        json: json!({ "server": ctx.server, "logged_in": true }),
    })
}
