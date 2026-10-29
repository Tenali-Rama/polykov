//! Configuration errors. Each message names the file, line, key or variable at fault.

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    /// Reading or creating something in the data dir failed.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `config.toml` is not valid TOML, or has an unknown key or a bad value.
    File {
        path: PathBuf,
        line: Option<usize>,
        message: String,
    },
    /// An env var holds a value that cannot be parsed.
    Env {
        key: &'static str,
        value: String,
        message: String,
    },
    /// No data dir was given and none could be derived from `XDG_DATA_HOME` or `HOME`.
    NoDataDir,
}

impl Error {
    pub(super) fn io(path: &Path, source: std::io::Error) -> Self {
        Error::Io {
            path: path.to_path_buf(),
            source,
        }
    }

    pub(super) fn env(key: &'static str, value: &str, err: impl fmt::Display) -> Self {
        Error::Env {
            key,
            value: value.to_string(),
            message: err.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::File {
                path,
                line: Some(line),
                message,
            } => write!(f, "{}: line {line}: {message}", path.display()),
            Error::File {
                path,
                line: None,
                message,
            } => write!(f, "{}: {message}", path.display()),
            Error::Env {
                key,
                value,
                message,
            } => write!(f, "{key}={value:?}: {message}"),
            Error::NoDataDir => write!(
                f,
                "cannot find a data dir: pass --data-dir or set POLYKOV_DATA_DIR, XDG_DATA_HOME or HOME"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
