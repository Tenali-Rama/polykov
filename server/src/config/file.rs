//! `config.toml`: parsing and the commented default written on first start.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::Error;

/// File name inside the data dir.
pub const NAME: &str = "config.toml";

/// Written when the data dir has no `config.toml`. Every option is commented
/// out and shows its default.
pub const DEFAULT: &str = r#"# Polykov server configuration.
# Every option is commented out and shows its default.
# Env vars (POLYKOV_BIND, ...) override this file; command-line flags override both.

# Address the HTTP server listens on. Env: POLYKOV_BIND. Flag: --bind.
# bind = "127.0.0.1:7700"

# Directory with the built web UI; a relative path is resolved against the
# server's working directory. Env: POLYKOV_WEB_DIR. Flag: --web-dir.
# web_dir = "web/dist"
"#;

/// Settings as written in `config.toml`. Missing keys are `None`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub bind: Option<SocketAddr>,
    pub web_dir: Option<PathBuf>,
}

/// Parses `text`, the contents of the file at `path`.
pub fn parse(path: &Path, text: &str) -> Result<FileConfig, Error> {
    toml::from_str(text).map_err(|e| Error::File {
        path: path.to_path_buf(),
        line: e.span().map(|span| line_of(text, span.start)),
        message: e.message().to_string(),
    })
}

/// 1-based line number of byte offset `pos` in `text`.
fn line_of(text: &str, pos: usize) -> usize {
    text.as_bytes()[..pos.min(text.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}
