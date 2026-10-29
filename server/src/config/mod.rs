//! Server settings: resolves the data dir, creates its layout, and merges
//! `config.toml`, `POLYKOV_*` env vars and command-line flags.
//!
//! Precedence for every setting: flag > env var > `config.toml` > default.

mod data_dir;
mod error;
mod file;
#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub use error::Error;

/// Default web UI directory, relative to the working directory.
pub const DEFAULT_WEB_DIR: &str = "web/dist";

/// Values given on the command line. `None` means the flag was not passed.
#[derive(Debug, Default, Clone)]
pub struct Flags {
    pub data_dir: Option<PathBuf>,
    pub bind: Option<SocketAddr>,
    pub web_dir: Option<PathBuf>,
}

/// Fully resolved server settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub data_dir: PathBuf,
    pub bind: SocketAddr,
    pub web_dir: PathBuf,
}

/// Resolves the data dir, creates its layout (and a default `config.toml` if
/// missing), then reads the settings.
///
/// `env` looks up an environment variable; `main` passes `std::env::var`.
pub fn load(flags: Flags, env: impl Fn(&str) -> Option<String>) -> Result<Config, Error> {
    let env = |key: &str| env(key).filter(|v| !v.is_empty());
    let data_dir = data_dir::resolve(flags.data_dir.clone(), &env)?;
    data_dir::prepare(&data_dir)?;
    let path = data_dir.join(file::NAME);
    let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
    let file = file::parse(&path, &text)?;
    merge(data_dir, flags, file, &env)
}

fn merge(
    data_dir: PathBuf,
    flags: Flags,
    file: file::FileConfig,
    env: &impl Fn(&str) -> Option<String>,
) -> Result<Config, Error> {
    let bind = match flags.bind {
        Some(bind) => bind,
        None => match env("POLYKOV_BIND") {
            Some(v) => v.parse().map_err(|e| Error::env("POLYKOV_BIND", &v, e))?,
            None => match file.bind {
                Some(bind) => bind,
                None => default_bind(),
            },
        },
    };
    let web_dir = flags
        .web_dir
        .or_else(|| env("POLYKOV_WEB_DIR").map(PathBuf::from))
        .or(file.web_dir)
        .unwrap_or_else(|| Path::new(DEFAULT_WEB_DIR).to_path_buf());
    Ok(Config {
        data_dir,
        bind,
        web_dir,
    })
}

/// Default listen address, `127.0.0.1:7700`.
fn default_bind() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 7700))
}
