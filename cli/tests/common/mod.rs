//! A real `polykov-server` router on a random port, and a `polykov` command isolated in its
//! own config dir.
#![allow(dead_code)]

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use polykov_server::{app, db::Db};
use serde_json::{Value, json};

pub const PASSWORD: &str = "correct horse battery";
/// How long `Server::start` waits for the server to answer `/health`.
const HEALTH_WAIT: Duration = Duration::from_secs(30);
/// The timeout of the `/api/setup` call, which hashes the password.
const SETUP_TIMEOUT: Duration = Duration::from_secs(60);

/// A temp dir removed on drop, unique per test.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("polykov-cli-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A server with an owner already set up (password `PASSWORD`). It runs until the test
/// process exits.
pub struct Server {
    pub url: String,
    _data: TempDir,
}

impl Server {
    pub fn start(name: &str) -> Self {
        let data = TempDir::new(&format!("{name}-data"));
        let db = Db::open(data.path()).unwrap();
        let router = app::router(db, &data.path().join("no-web"));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                axum::serve(listener, router).await.unwrap();
            });
        });
        wait_for_health(&url);
        let res = ureq::post(format!("{url}/api/setup"))
            .config()
            .timeout_global(Some(SETUP_TIMEOUT))
            .build()
            .send_json(json!({ "password": PASSWORD }))
            .unwrap_or_else(|e| panic!("test server setup: POST {url}/api/setup failed: {e}"));
        assert_eq!(
            res.status(),
            201,
            "test server setup: POST {url}/api/setup returned {}",
            res.status()
        );
        Server { url, _data: data }
    }
}

/// Retries `GET /health` until it answers `200`, for up to `HEALTH_WAIT`.
fn wait_for_health(url: &str) {
    let deadline = Instant::now() + HEALTH_WAIT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let last = match ureq::get(format!("{url}/health"))
            .config()
            .timeout_global(Some(left))
            .build()
            .call()
        {
            Ok(res) if res.status() == 200 => return,
            Ok(res) => format!("status {}", res.status()),
            Err(e) => e.to_string(),
        };
        if Instant::now() >= deadline {
            panic!(
                "test server start: GET {url}/health did not answer within {HEALTH_WAIT:?}: {last}"
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A URL on which nothing listens.
pub fn dead_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}", listener.local_addr().unwrap())
}

/// The CLI config dir (`XDG_CONFIG_HOME`) for one test.
pub struct Home(pub TempDir);

impl Home {
    pub fn new(name: &str) -> Self {
        Home(TempDir::new(&format!("{name}-home")))
    }

    pub fn config_file(&self) -> PathBuf {
        self.0.path().join("polykov").join("cli.toml")
    }

    pub fn config(&self) -> String {
        fs::read_to_string(self.config_file()).unwrap_or_default()
    }

    /// `polykov` with only this home's config, no server or password from the environment.
    pub fn polykov(&self) -> Command {
        let mut cmd = Command::cargo_bin("polykov").unwrap();
        cmd.env("XDG_CONFIG_HOME", self.0.path())
            .env_remove("POLYKOV_SERVER")
            .env_remove("POLYKOV_PASSWORD");
        cmd
    }

    pub fn login(&self, server: &str) {
        self.polykov()
            .args(["login", "--server", server])
            .env("POLYKOV_PASSWORD", PASSWORD)
            .assert()
            .success();
    }
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout).unwrap()
    }

    pub fn err_json(&self) -> Value {
        serde_json::from_str(&self.stderr).unwrap()
    }
}

pub fn run(cmd: &mut Command) -> Run {
    let out = cmd.output().unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}
