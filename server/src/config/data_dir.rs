//! Finding the data dir and creating its layout.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use super::{Error, file};

/// Subdirectories created in every data dir (spec §2).
pub const SUBDIRS: [&str; 3] = ["attachments", "models", "backups"];

/// Data dir from, in order: `--data-dir`, `POLYKOV_DATA_DIR`,
/// `$XDG_DATA_HOME/polykov`, `$HOME/.local/share/polykov`.
///
/// A relative `XDG_DATA_HOME` is ignored, as the XDG Base Directory spec requires.
pub fn resolve(
    flag: Option<PathBuf>,
    env: &impl Fn(&str) -> Option<String>,
) -> Result<PathBuf, Error> {
    if let Some(dir) = flag.or_else(|| env("POLYKOV_DATA_DIR").map(PathBuf::from)) {
        return Ok(dir);
    }
    if let Some(xdg) = env("XDG_DATA_HOME").map(PathBuf::from)
        && xdg.is_absolute()
    {
        return Ok(xdg.join("polykov"));
    }
    env("HOME")
        .map(|home| PathBuf::from(home).join(".local/share/polykov"))
        .ok_or(Error::NoDataDir)
}

/// Creates the data dir, its subdirectories and a default `config.toml`.
/// Anything that already exists is left untouched.
pub fn prepare(dir: &Path) -> Result<(), Error> {
    for sub in SUBDIRS {
        let path = dir.join(sub);
        fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;
    }
    let path = dir.join(file::NAME);
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut f) => f
            .write_all(file::DEFAULT.as_bytes())
            .map_err(|e| Error::io(&path, e)),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(Error::io(&path, e)),
    }
}
