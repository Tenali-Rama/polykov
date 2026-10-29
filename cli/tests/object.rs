//! `new`, `show`, `ls`, `edit`, `rm`, `pin`, `unpin`, `archive` and `unarchive` against the
//! real server router.

mod common;

use common::{Home, Run, Server, run};
use serde_json::{Value, json};

/// A logged-in home on its own server.
fn setup(name: &str) -> (Server, Home) {
    let server = Server::start(name);
    let home = Home::new(name);
    home.login(&server.url);
    (server, home)
}

/// Runs `polykov <args>` against `server` and expects exit 0.
fn ok(server: &Server, home: &Home, args: &[&str]) -> Run {
    let out = run(home.polykov().args(args).args(["--server", &server.url]));
    assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
    out
}

/// `polykov new ... --json` and the created object's id.
fn create(server: &Server, home: &Home, args: &[&str]) -> u64 {
    let mut full = vec!["new"];
    full.extend_from_slice(args);
    full.push("--json");
    ok(server, home, &full).json()["id"].as_u64().unwrap()
}

fn show_json(server: &Server, home: &Home, id: u64) -> Value {
    ok(server, home, &["show", &id.to_string(), "--json"]).json()
}

#[test]
fn new_prints_created_id_and_json_gives_the_object() {
    let (server, home) = setup("obj-new");
    let out = ok(
        &server,
        &home,
        &[
            "new",
            "note",
            "Buy",
            "milk",
            "--tag",
            "Shop/Food",
            "--tag",
            "home",
        ],
    );
    assert_eq!(out.stdout.trim(), "created 1");

    let out = ok(
        &server,
        &home,
        &[
            "new",
            "task",
            "Call",
            "--prop",
            "n=3",
            "--prop",
            "x=1.5",
            "--prop",
            "done=false",
            "--prop",
            "who=Ann",
            "--json",
        ],
    );
    let o = out.json();
    assert_eq!(o["id"], 2);
    assert_eq!(o["type"], "task");
    assert_eq!(o["title"], "Call");
    assert_eq!(
        o["properties"],
        json!({"n": 3, "x": 1.5, "done": false, "who": "Ann"})
    );
    assert_eq!(
        show_json(&server, &home, 1)["tags"],
        json!(["home", "shop/food"])
    );
}

#[test]
fn prop_without_equals_exits_2() {
    let (server, home) = setup("obj-prop");
    let out = run(home.polykov().args([
        "new",
        "note",
        "x",
        "--prop",
        "oops",
        "--server",
        &server.url,
    ]));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("expected key=value"), "{}", out.stderr);
    let out = ok(&server, &home, &["ls", "--json"]);
    assert_eq!(out.json()["total"], 0);
}

#[test]
fn show_prints_fields_and_properties() {
    let (server, home) = setup("obj-show");
    let id = create(
        &server,
        &home,
        &["note", "Groceries", "--tag", "shop", "--prop", "aisle=3"],
    );
    ok(&server, &home, &["pin", &id.to_string()]);
    let out = ok(&server, &home, &["show", &id.to_string()]);
    let lines: Vec<&str> = out.stdout.lines().collect();
    assert_eq!(lines[0], "Groceries");
    assert_eq!(lines[1], "  id        1");
    assert_eq!(lines[2], "  type      note");
    assert_eq!(lines[3], "  tags      #shop");
    assert_eq!(lines[4], "  pinned    yes");
    assert_eq!(lines[5], "  archived  no");
    assert!(lines[6].starts_with("  created   20"), "{}", lines[6]);
    assert!(lines[7].starts_with("  updated   20"), "{}", lines[7]);
    assert_eq!(lines[8], "  .aisle    3");
    assert_eq!(lines.len(), 9);

    let o = show_json(&server, &home, id);
    assert_eq!(o["pinned"], true);
    assert_eq!(o["properties"], json!({"aisle": 3}));
}

#[test]
fn ls_filters_and_marks_pinned() {
    let (server, home) = setup("obj-ls");
    let a = create(&server, &home, &["note", "Alpha", "--tag", "work/a"]);
    let b = create(&server, &home, &["task", "Beta"]);
    let c = create(&server, &home, &["note", "Gamma", "--tag", "workshop"]);
    ok(&server, &home, &["pin", &a.to_string()]);
    ok(&server, &home, &["archive", &c.to_string()]);

    let out = ok(&server, &home, &["ls"]);
    assert_eq!(
        out.stdout,
        format!("*{a}  note  Alpha  #work/a\n{b}  task  Beta\n")
    );

    let ids = |args: &[&str]| -> Vec<u64> {
        let mut full = vec!["ls", "--json"];
        full.extend_from_slice(args);
        let body = ok(&server, &home, &full).json();
        let ids: Vec<u64> = body["objects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["id"].as_u64().unwrap())
            .collect();
        assert_eq!(body["total"].as_u64(), Some(ids.len() as u64));
        ids
    };
    assert_eq!(ids(&["--type", "task"]), [b]);
    assert_eq!(ids(&["--tag", "work"]), [a]);
    assert_eq!(ids(&["--pinned"]), [a]);
    assert_eq!(ids(&["--archived"]), [c]);
    assert_eq!(ids(&["--all"]), [a, c, b]);

    let body = ok(&server, &home, &["ls", "--all", "--limit", "1", "--json"]).json();
    assert_eq!(body["objects"].as_array().unwrap().len(), 1);
    assert_eq!(body["total"], 3);
    let out = ok(&server, &home, &["ls", "--all", "--limit", "1"]);
    assert_eq!(
        out.stdout,
        format!("*{a}  note  Alpha  #work/a\n(1 of 3 shown)\n")
    );
    let out = ok(&server, &home, &["ls", "--type", "event"]);
    assert_eq!(out.stdout, "no objects\n");

    let out = run(home
        .polykov()
        .args(["ls", "--archived", "--all", "--server", &server.url]));
    assert_eq!(out.code, 2);
}

#[test]
fn edit_merges_into_one_patch() {
    let (server, home) = setup("obj-edit");
    let id = create(
        &server,
        &home,
        &[
            "note", "Old", "--tag", "a", "--tag", "work", "--prop", "k=1", "--prop", "drop=x",
        ],
    );
    let ids = id.to_string();
    let out = ok(
        &server,
        &home,
        &[
            "edit",
            &ids,
            "--title",
            "New",
            "--add-tag",
            "b",
            "--rm-tag",
            "#Work",
            "--prop",
            "k=2",
            "--prop",
            "flag=true",
            "--unset",
            "drop",
        ],
    );
    assert_eq!(out.stdout.trim(), format!("updated {id}"));
    let o = show_json(&server, &home, id);
    assert_eq!(o["title"], "New");
    assert_eq!(o["tags"], json!(["a", "b"]));
    assert_eq!(o["properties"], json!({"k": 2, "flag": true}));

    let o = ok(&server, &home, &["edit", &ids, "--prop", "n=x", "--json"]).json();
    assert_eq!(o["title"], "New");
    assert_eq!(o["tags"], json!(["a", "b"]));
    assert_eq!(o["properties"], json!({"k": 2, "flag": true, "n": "x"}));
}

#[test]
fn edit_without_change_flags_is_a_usage_error_and_sends_nothing() {
    let (server, home) = setup("obj-edit-none");
    let id = create(&server, &home, &["note", "Same"]);
    let before = show_json(&server, &home, id)["updated"].clone();
    let out = run(home
        .polykov()
        .args(["edit", &id.to_string(), "--server", &server.url]));
    assert_eq!(out.code, 2);
    assert_eq!(out.stderr.lines().count(), 1, "{}", out.stderr);
    for flag in ["--title", "--add-tag", "--rm-tag", "--prop", "--unset"] {
        assert!(out.stderr.contains(flag), "{flag}: {}", out.stderr);
    }
    assert!(out.stdout.is_empty());
    assert_eq!(show_json(&server, &home, id)["updated"], before);
}

#[test]
fn pin_unpin_archive_unarchive() {
    let (server, home) = setup("obj-flags");
    let id = create(&server, &home, &["note", "Flags"]);
    let ids = id.to_string();
    for (cmd, said, field, value) in [
        ("pin", "pinned", "pinned", true),
        ("unpin", "unpinned", "pinned", false),
        ("archive", "archived", "archived", true),
        ("unarchive", "unarchived", "archived", false),
    ] {
        let out = ok(&server, &home, &[cmd, &ids]);
        assert_eq!(out.stdout.trim(), format!("{said} {id}"));
        assert_eq!(show_json(&server, &home, id)[field], value, "{cmd}");
        let o = ok(&server, &home, &[cmd, &ids, "--json"]).json();
        assert_eq!(o["id"], id);
        assert_eq!(o[field], value, "{cmd}");
    }
}

#[test]
fn rm_trashes_the_object() {
    let (server, home) = setup("obj-rm");
    let keep = create(&server, &home, &["note", "Keep"]);
    let gone = create(&server, &home, &["note", "Gone"]);
    let out = ok(&server, &home, &["rm", &gone.to_string()]);
    assert_eq!(out.stdout.trim(), format!("trashed {gone}"));
    let out = ok(&server, &home, &["rm", &keep.to_string(), "--json"]);
    assert_eq!(out.json(), json!({"id": keep, "trashed": true}));
    assert_eq!(ok(&server, &home, &["ls", "--json"]).json()["total"], 0);
}

#[test]
fn unknown_id_prints_server_404_and_exits_1() {
    let (server, home) = setup("obj-404");
    for args in [
        vec!["show", "99"],
        vec!["edit", "99", "--title", "x"],
        vec!["rm", "99"],
        vec!["pin", "99"],
        vec!["archive", "99"],
    ] {
        let out = run(home.polykov().args(&args).args(["--server", &server.url]));
        assert_eq!(out.code, 1, "{args:?}");
        assert_eq!(
            out.stderr.trim(),
            "polykov: server returned 404: object not found"
        );
        assert!(out.stdout.is_empty());

        let out = run(home
            .polykov()
            .args(&args)
            .args(["--json", "--server", &server.url]));
        assert_eq!(out.code, 1, "{args:?}");
        assert_eq!(
            out.err_json(),
            json!({"error": "server returned 404: object not found"})
        );
    }
}

#[test]
fn server_400_prints_its_message() {
    let (server, home) = setup("obj-400");
    let out = run(home
        .polykov()
        .args(["new", "gadget", "x", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .starts_with("polykov: server returned 400: type must be one of: task"),
        "{}",
        out.stderr
    );
}

#[test]
fn object_commands_need_a_session() {
    let server = Server::start("obj-anon");
    let home = Home::new("obj-anon");
    let out = run(home.polykov().args(["ls", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "polykov: session missing or expired; run polykov login"
    );
}
