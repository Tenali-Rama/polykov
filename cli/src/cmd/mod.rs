//! Subcommands. Each one takes a `Ctx` and returns an `Output`; `main` prints it.

use std::path::PathBuf;

use serde_json::Value;

use crate::client::Client;
use crate::config::{self, CliConfig};
use crate::error::Error;

pub mod login;
pub mod logout;
pub mod status;

/// What a command prints: `human` as text, or `json` (one object) under `--json`.
pub struct Output {
    pub human: String,
    pub json: Value,
}

/// The resolved server, the loaded config file and where it lives.
pub struct Ctx {
    pub server: String,
    pub config: CliConfig,
    pub config_path: PathBuf,
}

impl Ctx {
    pub fn load(server_flag: Option<String>) -> Result<Self, Error> {
        let config_path = config::path(config::env)?;
        let config = CliConfig::load(&config_path)?;
        let server = config::resolve_server(
            server_flag,
            config::env("POLYKOV_SERVER"),
            config.server.clone(),
        )?;
        Ok(Ctx {
            server,
            config,
            config_path,
        })
    }

    /// The stored session token, if it belongs to the resolved server.
    pub fn token(&self) -> Option<&str> {
        self.config.token_for(&self.server)
    }

    /// A client for the resolved server, authenticated if a token is stored for it.
    pub fn client(&self) -> Client {
        Client::new(&self.server, self.token())
    }
}
