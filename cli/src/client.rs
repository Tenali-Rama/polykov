//! Blocking HTTP client for the server's API: JSON calls plus raw-bytes upload and download.

use std::time::Duration;

use serde_json::Value;
use ureq::http::{HeaderMap, Response};
use ureq::typestate::WithBody;
use ureq::{Agent, RequestBuilder};

use crate::error::Error;

const TIMEOUT: Duration = Duration::from_secs(10);
/// The largest body read back: above the server's 25 MiB attachment limit.
const MAX_BODY_BYTES: u64 = 26 * 1024 * 1024;

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

/// A response of any status with its raw body and headers, from `get_bytes`.
pub struct BytesReply {
    pub status: u16,
    pub headers: HeaderMap,
    pub bytes: Vec<u8>,
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
        self.reply(self.auth(self.agent.get(self.url(path))).call())
    }

    pub fn delete(&self, path: &str) -> Result<Reply, Error> {
        self.reply(self.auth(self.agent.delete(self.url(path))).call())
    }

    /// POSTs `body` as JSON, or an empty body if `None`.
    pub fn post(&self, path: &str, body: Option<&Value>) -> Result<Reply, Error> {
        self.send(self.agent.post(self.url(path)), body)
    }

    /// PUTs `body` as JSON, or an empty body if `None`.
    #[allow(dead_code)] // First used by `polykov link` (task 019).
    pub fn put(&self, path: &str, body: Option<&Value>) -> Result<Reply, Error> {
        self.send(self.agent.put(self.url(path)), body)
    }

    /// PATCHes `body` as JSON.
    pub fn patch(&self, path: &str, body: &Value) -> Result<Reply, Error> {
        self.send(self.agent.patch(self.url(path)), Some(body))
    }

    /// POSTs `bytes` as the raw body with the given `Content-Type`; the reply is JSON.
    pub fn post_bytes(&self, path: &str, content_type: &str, bytes: &[u8]) -> Result<Reply, Error> {
        let req = self.auth(self.agent.post(self.url(path)));
        self.reply(req.header("Content-Type", content_type).send(bytes))
    }

    /// GETs `path` and keeps the raw body and the headers.
    pub fn get_bytes(&self, path: &str) -> Result<BytesReply, Error> {
        let mut res = self
            .auth(self.agent.get(self.url(path)))
            .call()
            .map_err(|e| self.unreachable(e))?;
        let bytes = res
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .read_to_vec()
            .map_err(|e| self.unreachable(e))?;
        Ok(BytesReply {
            status: res.status().as_u16(),
            headers: res.headers().clone(),
            bytes,
        })
    }

    fn auth<B>(&self, req: RequestBuilder<B>) -> RequestBuilder<B> {
        match &self.token {
            Some(token) => req.header("Authorization", format!("Bearer {token}")),
            None => req,
        }
    }

    fn send(&self, req: RequestBuilder<WithBody>, body: Option<&Value>) -> Result<Reply, Error> {
        let req = self.auth(req);
        let res = match body {
            Some(body) => req.send_json(body),
            None => req.send_empty(),
        };
        self.reply(res)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.server)
    }

    fn unreachable(&self, e: ureq::Error) -> Error {
        Error::Unreachable {
            server: self.server.clone(),
            cause: e.to_string(),
        }
    }

    fn reply(&self, res: Result<Response<ureq::Body>, ureq::Error>) -> Result<Reply, Error> {
        let mut res = res.map_err(|e| self.unreachable(e))?;
        let status = res.status().as_u16();
        let text = res
            .body_mut()
            .read_to_string()
            .map_err(|e| self.unreachable(e))?;
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

impl BytesReply {
    /// The reply itself if `2xx`; otherwise the same errors as `Reply::ok`, the message taken
    /// from a JSON error body.
    pub fn ok(self) -> Result<Self, Error> {
        if (200..=299).contains(&self.status) {
            return Ok(self);
        }
        let body = serde_json::from_slice(&self.bytes).unwrap_or(Value::Null);
        let reply = Reply {
            status: self.status,
            body,
        };
        reply.ok().map(|_| self)
    }
}
