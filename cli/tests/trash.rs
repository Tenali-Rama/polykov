//! `trash ls`, `trash restore`, `trash purge` and `trash empty` against the real server router.

mod common;

use common::{Home, Run, Server, run};
use serde_json::json;

/// A logged-in home on its own server with objects 1 `note Old` and 2 `task Gone`, both
/// in the trash, and 3 `note Live`.
fn setup(name: &str) -> (Server, Home) {
    let server = Server::start(name);
    let home = Home::new(name);
    home.login(&server.url);
    for (kind, title) in [("note", "Old"), ("task", "Gone"), ("note", "Live")] {
        ok(&server, &home, &["new", kind, title]);
    }
    ok(&server, &home, &["rm", "1"]);
    ok(&server, &home, &["rm", "2"]);
    (server, home)
}

/// Runs `polykov <args>` against `server` and expects exit 0.
fn ok(server: &Server, home: &Home, args: &[&str]) -> Run {
    let out = run(home.polykov().args(args).args(["--server", &server.url]));
    assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
    out
}

/// `YYYY-MM-DD`: four digits, a dash, two digits, a dash, two digits.
fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

#[test]
fn ls_restore_purge_and_empty_in_human_mode() {
    let (server, home) = setup("trash-human");
    let out = ok(&server, &home, &["trash", "ls"]);
    let lines: Vec<&str> = out.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{}", out.stdout);
    for line in &lines {
        let cols: Vec<&str> = line.split("  ").collect();
        assert_eq!(cols.len(), 5, "{line}");
        assert!(
            cols[3].strip_prefix("deleted ").is_some_and(is_date),
            "{line}"
        );
        assert!(
            cols[4].strip_prefix("purge ").is_some_and(is_date),
            "{line}"
        );
    }
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("1  note  Old  deleted "))
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("2  task  Gone  deleted "))
    );

    assert_eq!(
        ok(&server, &home, &["trash", "restore", "1"]).stdout,
        "restored 1\n"
    );
    assert_eq!(
        ok(&server, &home, &["show", "1", "--json"]).json()["title"],
        "Old"
    );
    assert_eq!(
        ok(&server, &home, &["trash", "purge", "2"]).stdout,
        "purged 2\n"
    );
    assert_eq!(
        ok(&server, &home, &["trash", "ls"]).stdout,
        "trash is empty\n"
    );

    ok(&server, &home, &["rm", "1"]);
    ok(&server, &home, &["rm", "3"]);
    let out = ok(&server, &home, &["trash", "empty", "--yes"]);
    assert_eq!(out.stdout, "purged 2\n");
    assert_eq!(
        ok(&server, &home, &["trash", "ls"]).stdout,
        "trash is empty\n"
    );
}

#[test]
fn ls_restore_purge_and_empty_in_json_mode() {
    let (server, home) = setup("trash-json");
    let out = ok(&server, &home, &["trash", "ls", "--json"]);
    let objects = out.json()["objects"].clone();
    assert_eq!(objects.as_array().unwrap().len(), 2);
    assert!(objects[0]["deleted"].is_string() && objects[0]["purge_at"].is_string());

    let out = ok(&server, &home, &["trash", "restore", "1", "--json"]);
    let o = out.json();
    assert_eq!(
        (o["id"].clone(), o["title"].clone()),
        (json!(1), json!("Old"))
    );

    let out = ok(&server, &home, &["trash", "purge", "2", "--json"]);
    assert_eq!(out.json(), json!({"id": 2, "purged": true}));

    ok(&server, &home, &["rm", "3"]);
    let out = ok(&server, &home, &["trash", "empty", "--yes", "--json"]);
    assert_eq!(out.json(), json!({"purged": 1}));
    let out = ok(&server, &home, &["trash", "ls", "--json"]);
    assert_eq!(out.json(), json!({"objects": []}));
}

#[test]
fn empty_without_yes_exits_2_and_keeps_the_trash() {
    let (server, home) = setup("trash-noyes");
    let out = run(home
        .polykov()
        .args(["trash", "empty", "--server", &server.url]));
    assert_eq!(out.code, 2);
    assert!(out.stdout.is_empty());
    assert!(out.stderr.contains("--yes"), "{}", out.stderr);
    let out = ok(&server, &home, &["trash", "ls", "--json"]);
    assert_eq!(out.json()["objects"].as_array().unwrap().len(), 2);
}

#[test]
fn live_or_unknown_id_prints_404() {
    let (server, home) = setup("trash-404");
    let out = run(home
        .polykov()
        .args(["trash", "restore", "3", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "polykov: server returned 404: not in trash"
    );

    let out = run(home
        .polykov()
        .args(["trash", "purge", "99", "--json", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.err_json(),
        json!({"error": "server returned 404: not in trash"})
    );
}
