//! `comment add`, `comment ls`, `comment edit` and `comment rm` against the real server router.

mod common;

use common::{Home, Run, Server, run};
use serde_json::json;

/// A logged-in home on its own server with object 1 `note Plan`.
fn setup(name: &str) -> (Server, Home) {
    let server = Server::start(name);
    let home = Home::new(name);
    home.login(&server.url);
    ok(&server, &home, &["new", "note", "Plan"]);
    (server, home)
}

/// Runs `polykov <args>` against `server` and expects exit 0.
fn ok(server: &Server, home: &Home, args: &[&str]) -> Run {
    let out = run(home.polykov().args(args).args(["--server", &server.url]));
    assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
    out
}

#[test]
fn add_ls_edit_and_rm_in_human_mode() {
    let (server, home) = setup("comment-human");
    assert_eq!(
        ok(&server, &home, &["comment", "ls", "1"]).stdout,
        "no comments\n"
    );

    let out = ok(&server, &home, &["comment", "add", "1", "looks", "good"]);
    assert_eq!(out.stdout, "added comment 1\n");
    ok(
        &server,
        &home,
        &["comment", "add", "1", "line one\nline two"],
    );

    let out = ok(&server, &home, &["comment", "ls", "1"]);
    let lines: Vec<&str> = out.stdout.lines().collect();
    assert_eq!(lines.len(), 5, "{}", out.stdout);
    assert!(
        lines[0].starts_with("#1 20") && lines[0].ends_with('Z'),
        "{}",
        lines[0]
    );
    assert_eq!(lines[1], "  looks good");
    assert!(lines[2].starts_with("#2 "));
    assert_eq!(&lines[3..], ["  line one", "  line two"]);

    let out = ok(&server, &home, &["comment", "edit", "1", "even", "better"]);
    assert_eq!(out.stdout, "updated comment 1\n");
    let out = ok(&server, &home, &["comment", "rm", "2"]);
    assert_eq!(out.stdout, "deleted comment 2\n");
    let out = ok(&server, &home, &["comment", "ls", "1"]);
    assert_eq!(out.stdout.lines().nth(1), Some("  even better"));
    assert_eq!(out.stdout.lines().count(), 2);
}

#[test]
fn add_ls_edit_and_rm_in_json_mode() {
    let (server, home) = setup("comment-json");
    let out = ok(&server, &home, &["comment", "add", "1", "hi", "--json"]);
    let c = out.json();
    assert_eq!(
        (c["id"].clone(), c["object_id"].clone()),
        (json!(1), json!(1))
    );
    assert_eq!(c["body"], "hi");

    let out = ok(&server, &home, &["comment", "edit", "1", "bye", "--json"]);
    assert_eq!(out.json()["body"], "bye");

    let out = ok(&server, &home, &["comment", "ls", "1", "--json"]);
    let comments = out.json()["comments"].clone();
    assert_eq!(comments.as_array().unwrap().len(), 1);
    assert_eq!(comments[0]["body"], "bye");

    let out = ok(&server, &home, &["comment", "rm", "1", "--json"]);
    assert_eq!(out.json(), json!({"id": 1, "deleted": true}));
    let out = ok(&server, &home, &["comment", "ls", "1", "--json"]);
    assert_eq!(out.json(), json!({"comments": []}));
}

#[test]
fn unknown_comment_prints_404_and_text_is_required() {
    let (server, home) = setup("comment-404");
    let out = run(home
        .polykov()
        .args(["comment", "edit", "9", "x", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "polykov: server returned 404: comment not found"
    );

    let out = run(home
        .polykov()
        .args(["comment", "ls", "9", "--json", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.err_json(),
        json!({"error": "server returned 404: object not found"})
    );

    let out = run(home
        .polykov()
        .args(["comment", "add", "1", "--server", &server.url]));
    assert_eq!(out.code, 2);
}
