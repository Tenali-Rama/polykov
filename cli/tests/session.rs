//! `login`, `status` and `logout` against the real server router.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use common::{Home, PASSWORD, Server, dead_url, run};

fn stored_token(home: &Home) -> String {
    let config: toml::Table = toml::from_str(&home.config()).unwrap();
    config["token"].as_str().unwrap().to_owned()
}

fn me_status(server: &Server, token: &str) -> u16 {
    ureq::get(format!("{}/api/me", server.url))
        .header("Authorization", format!("Bearer {token}"))
        .config()
        .http_status_as_error(false)
        .build()
        .call()
        .unwrap()
        .status()
        .as_u16()
}

fn mode(path: &std::path::Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn login_stores_server_and_valid_token_with_mode_0600() {
    let server = Server::start("login");
    let home = Home::new("login");
    let out = run(home
        .polykov()
        .args(["login", "--server", &server.url])
        .env("POLYKOV_PASSWORD", PASSWORD));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout.trim(), format!("logged in to {}", server.url));

    let config: toml::Table = toml::from_str(&home.config()).unwrap();
    assert_eq!(config["server"].as_str(), Some(server.url.as_str()));
    let token = stored_token(&home);
    assert_eq!(token.len(), 64);
    assert_eq!(me_status(&server, &token), 200);
    assert_eq!(mode(&home.config_file()), 0o600);
}

#[test]
fn login_keeps_a_mode_the_owner_changed() {
    let server = Server::start("login-mode");
    let home = Home::new("login-mode");
    home.login(&server.url);
    fs::set_permissions(home.config_file(), fs::Permissions::from_mode(0o640)).unwrap();
    home.login(&server.url);
    assert_eq!(mode(&home.config_file()), 0o640);
}

#[test]
fn login_falls_back_to_home_dot_config() {
    let server = Server::start("login-home");
    let home = Home::new("login-home");
    let out = run(home
        .polykov()
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", home.0.path())
        .env("POLYKOV_PASSWORD", PASSWORD)
        .args(["login", "--server", &server.url]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(home.0.path().join(".config/polykov/cli.toml").is_file());
}

#[test]
fn login_with_wrong_password_fails_and_stores_nothing() {
    let server = Server::start("login-bad");
    let home = Home::new("login-bad");
    let out = run(home
        .polykov()
        .args(["login", "--server", &server.url])
        .env("POLYKOV_PASSWORD", "wrong password!!"));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "polykov: login failed: wrong password, or the owner is not set up yet\n"
    );
    assert!(!home.config_file().exists());
}

#[test]
fn status_with_valid_token() {
    let server = Server::start("status-ok");
    let home = Home::new("status-ok");
    home.login(&server.url);

    // The server comes from the config file written by login.
    let out = run(home.polykov().arg("status"));
    assert_eq!(out.code, 0, "{}", out.stderr);
    let version = env!("CARGO_PKG_VERSION");
    assert!(out.stdout.contains(&format!("server   {}\n", server.url)));
    assert!(out.stdout.contains(&format!("version  {version}\n")));
    assert!(out.stdout.contains("db       ok\n"));
    assert!(
        out.stdout.contains("token    valid, expires 20"),
        "{}",
        out.stdout
    );

    let json = run(home.polykov().args(["status", "--json"])).json();
    assert_eq!(json["server"], server.url.as_str());
    assert_eq!(json["version"], version);
    assert_eq!(json["db"], "ok");
    assert_eq!(json["token"], "valid");
    assert!(json["session_expires"].as_str().unwrap().ends_with('Z'));
}

#[test]
fn status_with_invalid_or_missing_token() {
    let server = Server::start("status-bad");
    let home = Home::new("status-bad");
    fs::create_dir_all(home.config_file().parent().unwrap()).unwrap();
    let bogus = format!(
        "server = \"{}\"\ntoken = \"{}\"\n",
        server.url,
        "0".repeat(64)
    );
    fs::write(home.config_file(), bogus).unwrap();

    let out = run(home.polykov().arg("status"));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("token    invalid (run polykov login)"));
    let json = run(home.polykov().args(["--json", "status"])).json();
    assert_eq!(json["token"], "invalid");
    assert!(json["session_expires"].is_null());

    // A token stored for another server is never sent elsewhere.
    let other = Server::start("status-other");
    let json = run(home
        .polykov()
        .args(["status", "--json", "--server", &other.url]))
    .json();
    assert_eq!(json["token"], "none");
}

#[test]
fn logout_ends_session_and_removes_token() {
    let server = Server::start("logout");
    let home = Home::new("logout");
    home.login(&server.url);
    let token = stored_token(&home);

    let out = run(home.polykov().args(["logout", "--json"]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.json()["logged_out"], true);
    assert_eq!(me_status(&server, &token), 401);
    assert!(!home.config().contains("token"), "{}", home.config());
    assert!(home.config().contains(&server.url));

    let again = run(home.polykov().arg("logout"));
    assert_eq!(again.code, 1);
    assert_eq!(
        again.stderr,
        format!(
            "polykov: not logged in to {}; run polykov login\n",
            server.url
        )
    );
}

#[test]
fn logout_with_rejected_token_says_run_login() {
    let server = Server::start("logout-401");
    let home = Home::new("logout-401");
    home.login(&server.url);
    // End the session behind the CLI's back.
    let token = stored_token(&home);
    ureq::post(format!("{}/api/logout", server.url))
        .header("Authorization", format!("Bearer {token}"))
        .send_empty()
        .unwrap();

    let out = run(home.polykov().args(["logout", "--json"]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.err_json()["error"],
        "session missing or expired; run polykov login"
    );
    assert!(out.stdout.is_empty());
    assert!(!home.config().contains("token"));
}

#[test]
fn unreachable_server_is_one_line_exit_1() {
    let home = Home::new("dead");
    let dead = dead_url();
    let out = run(home.polykov().args(["status", "--server", &dead]));
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .starts_with(&format!("polykov: cannot reach server at {dead}: ")),
        "{}",
        out.stderr
    );
    assert_eq!(out.stderr.lines().count(), 1);

    let out = run(home.polykov().args(["--json", "status", "--server", &dead]));
    assert_eq!(out.code, 1);
    let error = out.err_json()["error"].as_str().unwrap().to_owned();
    assert!(error.starts_with("cannot reach server at"), "{error}");
}

#[test]
fn server_flag_beats_env_beats_config_file() {
    let server = Server::start("precedence");
    let home = Home::new("precedence");
    home.login(&server.url);
    let dead = dead_url();

    // The env var beats the config file...
    let env_wins = run(home.polykov().arg("status").env("POLYKOV_SERVER", &dead));
    assert_eq!(env_wins.code, 1);
    assert!(env_wins.stderr.contains(&dead));
    // ...an empty one counts as unset...
    let empty_env = run(home.polykov().arg("status").env("POLYKOV_SERVER", ""));
    assert_eq!(empty_env.code, 0, "{}", empty_env.stderr);
    // ...and the flag beats the env var.
    let flag_wins = run(home
        .polykov()
        .args(["status", "--server", &server.url])
        .env("POLYKOV_SERVER", &dead));
    assert_eq!(flag_wins.code, 0, "{}", flag_wins.stderr);
}
