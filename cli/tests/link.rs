//! `link add`, `link rm` and `link ls` against the real server router.

mod common;

use common::{Home, Run, Server, run};
use serde_json::json;

/// A logged-in home on its own server with objects 1 `note Plan`, 2 `task Write copy` and
/// 3 `note Ideas`.
fn setup(name: &str) -> (Server, Home) {
    let server = Server::start(name);
    let home = Home::new(name);
    home.login(&server.url);
    for (kind, title) in [("note", "Plan"), ("task", "Write copy"), ("note", "Ideas")] {
        ok(&server, &home, &["new", kind, title]);
    }
    (server, home)
}

/// Runs `polykov <args>` against `server` and expects exit 0.
fn ok(server: &Server, home: &Home, args: &[&str]) -> Run {
    let out = run(home.polykov().args(args).args(["--server", &server.url]));
    assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
    out
}

#[test]
fn add_ls_and_rm_in_human_mode() {
    let (server, home) = setup("link-human");
    assert_eq!(
        ok(&server, &home, &["link", "add", "1", "2"]).stdout,
        "linked 1 -> 2\n"
    );
    ok(&server, &home, &["link", "add", "3", "1"]);

    let out = ok(&server, &home, &["link", "ls", "1"]);
    assert_eq!(
        out.stdout,
        "links:\n  2  task  Write copy\nbacklinks:\n  3  note  Ideas\n"
    );

    let out = ok(&server, &home, &["link", "rm", "1", "2"]);
    assert_eq!(out.stdout, "unlinked 1 -> 2\n");
    let out = ok(&server, &home, &["link", "ls", "1"]);
    assert_eq!(out.stdout, "links:\n  none\nbacklinks:\n  3  note  Ideas\n");
}

#[test]
fn add_ls_and_rm_in_json_mode() {
    let (server, home) = setup("link-json");
    let out = ok(&server, &home, &["link", "add", "1", "2", "--json"]);
    assert_eq!(out.json(), json!({"id": 1, "to": 2, "linked": true}));

    let out = ok(&server, &home, &["link", "ls", "2", "--json"]);
    assert_eq!(
        out.json(),
        json!({"links": [], "backlinks": [{"id": 1, "type": "note", "title": "Plan"}]})
    );

    let out = ok(&server, &home, &["link", "rm", "1", "2", "--json"]);
    assert_eq!(out.json(), json!({"id": 1, "to": 2, "unlinked": true}));
    let out = ok(&server, &home, &["link", "ls", "2", "--json"]);
    assert_eq!(out.json(), json!({"links": [], "backlinks": []}));
}

#[test]
fn unknown_object_and_missing_link_print_404() {
    let (server, home) = setup("link-404");
    let out = run(home
        .polykov()
        .args(["link", "add", "1", "99", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "polykov: server returned 404: object not found"
    );

    let out = run(home
        .polykov()
        .args(["link", "rm", "1", "2", "--json", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert!(out.stdout.is_empty());
    assert_eq!(
        out.err_json(),
        json!({"error": "server returned 404: link not found"})
    );
}
