//! A real `polykov-server` router on a random port, and a `polykov` command isolated in its
//! own config dir.
#![allow(dead_code)]

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use polykov_server::{app, db::Db};
use serde_json::{Value, json};

pub const PASSWORD: &str = "correct horse battery";

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
        let res = ureq::post(format!("{url}/api/setup"))
            .send_json(json!({ "password": PASSWORD }))
            .unwrap();
        assert_eq!(res.status(), 201);
        Server { url, _data: data }
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
