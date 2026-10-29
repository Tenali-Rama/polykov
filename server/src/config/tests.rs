use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// A fresh, empty temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "polykov-config-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn env_of(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |key| map.get(key).cloned()
}

fn flags_for(dir: &Path) -> Flags {
    Flags {
        data_dir: Some(dir.to_path_buf()),
        ..Flags::default()
    }
}

fn write_config(dir: &Path, text: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("config.toml"), text).unwrap();
}

#[test]
fn defaults_when_nothing_is_set() {
    let tmp = TempDir::new("defaults");
    let cfg = load(flags_for(&tmp.0), env_of(&[])).unwrap();
    assert_eq!(cfg.data_dir, tmp.0);
    assert_eq!(cfg.bind.to_string(), "127.0.0.1:7700");
    assert_eq!(cfg.web_dir, PathBuf::from("web/dist"));
}

#[test]
fn first_start_creates_layout_and_commented_config() {
    let tmp = TempDir::new("layout");
    load(flags_for(&tmp.0), env_of(&[])).unwrap();
    for sub in ["attachments", "models", "backups"] {
        assert!(tmp.0.join(sub).is_dir(), "{sub} missing");
    }
    let text = fs::read_to_string(tmp.0.join("config.toml")).unwrap();
    assert_eq!(text, file::DEFAULT);
    assert!(text.contains("# bind = \"127.0.0.1:7700\""));
    assert!(text.contains("# web_dir = \"web/dist\""));
    // Every non-blank line is a comment, so the file sets nothing.
    assert!(text.lines().all(|l| l.is_empty() || l.starts_with('#')));
}

#[test]
fn existing_config_is_left_untouched() {
    let tmp = TempDir::new("untouched");
    let edited = "bind = \"127.0.0.1:9000\"\n";
    write_config(&tmp.0, edited);
    load(flags_for(&tmp.0), env_of(&[])).unwrap();
    load(flags_for(&tmp.0), env_of(&[])).unwrap();
    assert_eq!(
        fs::read_to_string(tmp.0.join("config.toml")).unwrap(),
        edited
    );
}

#[test]
fn file_values_override_defaults() {
    let tmp = TempDir::new("file");
    write_config(&tmp.0, "bind = \"0.0.0.0:8000\"\nweb_dir = \"/srv/web\"\n");
    let cfg = load(flags_for(&tmp.0), env_of(&[])).unwrap();
    assert_eq!(cfg.bind.to_string(), "0.0.0.0:8000");
    assert_eq!(cfg.web_dir, PathBuf::from("/srv/web"));
}

#[test]
fn env_overrides_file() {
    let tmp = TempDir::new("env");
    write_config(&tmp.0, "bind = \"0.0.0.0:8000\"\nweb_dir = \"/srv/web\"\n");
    let env = env_of(&[
        ("POLYKOV_BIND", "127.0.0.1:8001"),
        ("POLYKOV_WEB_DIR", "/env/web"),
    ]);
    let cfg = load(flags_for(&tmp.0), env).unwrap();
    assert_eq!(cfg.bind.to_string(), "127.0.0.1:8001");
    assert_eq!(cfg.web_dir, PathBuf::from("/env/web"));
}

#[test]
fn empty_env_var_counts_as_unset() {
    let tmp = TempDir::new("empty-env");
    write_config(&tmp.0, "bind = \"0.0.0.0:8000\"\n");
    let cfg = load(flags_for(&tmp.0), env_of(&[("POLYKOV_BIND", "")])).unwrap();
    assert_eq!(cfg.bind.to_string(), "0.0.0.0:8000");
}

#[test]
fn flag_overrides_env() {
    let tmp = TempDir::new("flag");
    write_config(&tmp.0, "bind = \"0.0.0.0:8000\"\n");
    let env = env_of(&[
        ("POLYKOV_BIND", "127.0.0.1:8001"),
        ("POLYKOV_WEB_DIR", "/env/web"),
    ]);
    let flags = Flags {
        bind: Some("127.0.0.1:8002".parse().unwrap()),
        web_dir: Some(PathBuf::from("/flag/web")),
        ..flags_for(&tmp.0)
    };
    let cfg = load(flags, env).unwrap();
    assert_eq!(cfg.bind.to_string(), "127.0.0.1:8002");
    assert_eq!(cfg.web_dir, PathBuf::from("/flag/web"));
}

#[test]
fn invalid_toml_names_file_and_line() {
    let tmp = TempDir::new("invalid");
    write_config(&tmp.0, "# comment\nbind = \"127.0.0.1:7700\"\nweb_dir = \n");
    let err = load(flags_for(&tmp.0), env_of(&[]))
        .unwrap_err()
        .to_string();
    let path = tmp.0.join("config.toml");
    assert!(
        err.starts_with(&format!("{}: line 3:", path.display())),
        "{err}"
    );
}

#[test]
fn bad_bind_in_file_names_file_and_line() {
    let tmp = TempDir::new("bad-bind");
    write_config(&tmp.0, "\nbind = \"not-an-addr\"\n");
    let err = load(flags_for(&tmp.0), env_of(&[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("config.toml: line 2:"), "{err}");
}

#[test]
fn unknown_key_fails_naming_the_key() {
    let tmp = TempDir::new("unknown");
    write_config(&tmp.0, "bnid = \"127.0.0.1:7700\"\n");
    let err = load(flags_for(&tmp.0), env_of(&[]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("config.toml: line 1:"), "{err}");
    assert!(err.contains("`bnid`"), "{err}");
}

#[test]
fn bad_env_bind_names_the_variable() {
    let tmp = TempDir::new("bad-env");
    let err = load(flags_for(&tmp.0), env_of(&[("POLYKOV_BIND", "nope")]))
        .unwrap_err()
        .to_string();
    assert!(err.starts_with("POLYKOV_BIND=\"nope\":"), "{err}");
}

#[test]
fn data_dir_flag_beats_env() {
    let env = env_of(&[("POLYKOV_DATA_DIR", "/env"), ("XDG_DATA_HOME", "/xdg")]);
    let dir = data_dir::resolve(Some(PathBuf::from("/flag")), &env).unwrap();
    assert_eq!(dir, PathBuf::from("/flag"));
}

#[test]
fn data_dir_env_beats_xdg() {
    let env = env_of(&[("POLYKOV_DATA_DIR", "/env"), ("XDG_DATA_HOME", "/xdg")]);
    assert_eq!(
        data_dir::resolve(None, &env).unwrap(),
        PathBuf::from("/env")
    );
}

#[test]
fn data_dir_falls_back_to_xdg_then_home() {
    let xdg = env_of(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/u")]);
    assert_eq!(
        data_dir::resolve(None, &xdg).unwrap(),
        PathBuf::from("/xdg/polykov")
    );
    let relative_xdg = env_of(&[("XDG_DATA_HOME", "rel"), ("HOME", "/home/u")]);
    assert_eq!(
        data_dir::resolve(None, &relative_xdg).unwrap(),
        PathBuf::from("/home/u/.local/share/polykov")
    );
}

#[test]
fn no_data_dir_source_is_an_error() {
    let err = data_dir::resolve(None, &env_of(&[])).unwrap_err();
    assert!(matches!(err, Error::NoDataDir));
}
