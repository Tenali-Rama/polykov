//! Blocking HTTP client for the server's JSON API.

use std::time::Duration;

use serde_json::Value;
use ureq::{Agent, http::Response};

use crate::error::Error;

const TIMEOUT: Duration = Duration::from_secs(10);

/// A connection to one server, optionally carrying a session token as a bearer token.
pub struct Client {
    server: String,
    token: Option<String>,
    agent: Agent,
}

/// A response of any status. `body` is the parsed JSON, or `Null` if empty or not JSON.
pub struct Reply {
    pub status: u16,
    pub body: Value,
}

impl Client {
    /// `server` is a normalized base URL (see `config::resolve_server`).
    pub fn new(server: &str, token: Option<&str>) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .build()
            .new_agent();
        Client {
            server: server.to_owned(),
            token: token.map(str::to_owned),
            agent,
        }
    }

    pub fn get(&self, path: &str) -> Result<Reply, Error> {
        let mut req = self.agent.get(self.url(path));
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        self.reply(req.call())
    }

    /// POSTs `body` as JSON, or an empty body if `None`.
    pub fn post(&self, path: &str, body: Option<&Value>) -> Result<Reply, Error> {
        let mut req = self.agent.post(self.url(path));
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let res = match body {
            Some(body) => req.send_json(body),
            None => req.send_empty(),
        };
        self.reply(res)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.server)
    }

    fn reply(&self, res: Result<Response<ureq::Body>, ureq::Error>) -> Result<Reply, Error> {
        let mut res = res.map_err(|e| Error::Unreachable {
            server: self.server.clone(),
            cause: e.to_string(),
        })?;
        let status = res.status().as_u16();
        let text = res
            .body_mut()
            .read_to_string()
            .map_err(|e| Error::Unreachable {
                server: self.server.clone(),
                cause: e.to_string(),
            })?;
        let body = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok(Reply { status, body })
    }
}

impl Reply {
    /// The body of a `2xx` reply. `401` becomes `Error::Unauthorized`; any other status
    /// becomes `Error::Server` with the body's `error` message.
    pub fn ok(self) -> Result<Value, Error> {
        match self.status {
            200..=299 => Ok(self.body),
            401 => Err(Error::Unauthorized),
            status => Err(Error::Server {
                status,
                message: self.body["error"].as_str().unwrap_or_default().to_owned(),
            }),
        }
    }
}
