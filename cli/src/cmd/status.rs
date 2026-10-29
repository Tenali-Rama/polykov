//! `polykov status`: server version, database state and stored-token validity.

use serde_json::{Value, json};

use super::{Ctx, Output};
use crate::error::Error;

pub fn run(ctx: &Ctx) -> Result<Output, Error> {
    let client = ctx.client();
    // `/health` answers 503 with the same body when the database is down.
    let health = client.get("/health")?;
    let health = match health.status {
        200 | 503 => health.body,
        _ => health.ok()?,
    };
    let field = |name: &str| health[name].as_str().unwrap_or("unknown").to_owned();
    let (version, db) = (field("version"), field("db"));

    let (token, expires) = match ctx.token() {
        None => ("none", None),
        Some(_) => match client.get("/api/me")?.ok() {
            Ok(me) => ("valid", me["session_expires"].as_str().map(str::to_owned)),
            Err(Error::Unauthorized) => ("invalid", None),
            Err(e) => return Err(e),
        },
    };
    let token_line = match &expires {
        Some(at) => format!("valid, expires {at}"),
        None if token == "valid" => "valid".to_owned(),
        None => format!("{token} (run polykov login)"),
    };
    Ok(Output {
        human: format!(
            "server   {}\nversion  {version}\ndb       {db}\ntoken    {token_line}",
            ctx.server
        ),
        json: json!({
            "server": ctx.server,
            "version": version,
            "db": db,
            "token": token,
            "session_expires": expires.map_or(Value::Null, Value::from),
        }),
    })
}
