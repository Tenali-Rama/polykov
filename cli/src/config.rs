//! The CLI config file (`cli.toml`) and server URL resolution.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Error;

pub const DEFAULT_SERVER: &str = "http://127.0.0.1:7700";
pub const FILE_NAME: &str = "cli.toml";

/// Contents of `cli.toml`. The token belongs to `server` and is never sent anywhere else.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CliConfig {
    pub server: Option<String>,
    pub token: Option<String>,
}

/// An env var's value; empty counts as unset.
pub fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// `$XDG_CONFIG_HOME/polykov/cli.toml` (absolute `XDG_CONFIG_HOME` only), else
/// `$HOME/.config/polykov/cli.toml`.
pub fn path(env: impl Fn(&str) -> Option<String>) -> Result<PathBuf, Error> {
    let base = match env("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(xdg) if xdg.is_absolute() => xdg,
        _ => env("HOME")
            .map(|home| PathBuf::from(home).join(".config"))
            .ok_or_else(|| {
                Error::Msg("cannot find the config dir: set XDG_CONFIG_HOME or HOME".into())
            })?,
    };
    Ok(base.join("polykov").join(FILE_NAME))
}

/// First match wins: `--server`, `POLYKOV_SERVER`, the config file, `DEFAULT_SERVER`.
/// The result has no trailing `/`.
pub fn resolve_server(
    flag: Option<String>,
    env: Option<String>,
    file: Option<String>,
) -> Result<String, Error> {
    let raw = flag
        .or(env)
        .or(file)
        .unwrap_or_else(|| DEFAULT_SERVER.into());
    normalize(&raw)
}

fn normalize(raw: &str) -> Result<String, Error> {
    let url = raw.trim().trim_end_matches('/');
    let host = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"));
    match host {
        Some(host) if !host.is_empty() => Ok(url.to_owned()),
        _ => Err(Error::Msg(format!(
            "invalid server URL \"{raw}\": must start with http:// or https://"
        ))),
    }
}

impl CliConfig {
    /// Reads `path`; a missing file is an empty config.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(Error::Msg(format!("cannot read {}: {e}", path.display()))),
        };
        toml::from_str(&text)
            .map_err(|e| Error::Msg(format!("invalid {}: {}", path.display(), e.message())))
    }

    /// Writes `path`, creating its dir. A new file gets mode `0600`; an existing file keeps
    /// whatever mode the owner gave it.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        let fail = |e: std::io::Error| Error::Msg(format!("cannot write {}: {e}", path.display()));
        let text = toml::to_string(self).map_err(|e| Error::Msg(e.to_string()))?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(fail)?;
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        options
            .open(path)
            .and_then(|mut file| file.write_all(text.as_bytes()))
            .map_err(fail)
    }

    /// The stored token, only if it was issued by `server`.
    pub fn token_for(&self, server: &str) -> Option<&str> {
        let stored = self.server.as_deref().and_then(|s| normalize(s).ok())?;
        (stored == server).then_some(self.token.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_owned())
    }

    #[test]
    fn server_precedence_is_flag_env_file_default() {
        let all = resolve_server(s("http://flag"), s("http://env"), s("http://file"));
        assert_eq!(all.unwrap(), "http://flag");
        let no_flag = resolve_server(None, s("http://env"), s("http://file"));
        assert_eq!(no_flag.unwrap(), "http://env");
        assert_eq!(
            resolve_server(None, None, s("http://file")).unwrap(),
            "http://file"
        );
        assert_eq!(resolve_server(None, None, None).unwrap(), DEFAULT_SERVER);
    }

    #[test]
    fn server_url_is_normalized_and_validated() {
        let url = resolve_server(s(" https://example.org/polykov/ "), None, None);
        assert_eq!(url.unwrap(), "https://example.org/polykov");
        for bad in ["example.org", "http://", "ftp://example.org", ""] {
            let err = resolve_server(s(bad), None, None).unwrap_err().to_string();
            assert!(err.contains("invalid server URL"), "{bad}: {err}");
        }
    }

    #[test]
    fn path_prefers_absolute_xdg_then_home() {
        let env = |xdg: &'static str| {
            move |k: &str| match k {
                "XDG_CONFIG_HOME" => s(xdg),
                "HOME" => s("/home/me"),
                _ => None,
            }
        };
        assert_eq!(
            path(env("/xdg")).unwrap(),
            Path::new("/xdg/polykov/cli.toml")
        );
        assert_eq!(
            path(env("relative")).unwrap(),
            Path::new("/home/me/.config/polykov/cli.toml")
        );
        assert!(path(|_| None).is_err());
    }

    #[test]
    fn token_is_only_offered_to_its_server() {
        let config = CliConfig {
            server: s("http://a:1/"),
            token: s("t"),
        };
        assert_eq!(config.token_for("http://a:1"), Some("t"));
        assert_eq!(config.token_for("http://b:1"), None);
        assert_eq!(CliConfig::default().token_for(DEFAULT_SERVER), None);
    }
}
