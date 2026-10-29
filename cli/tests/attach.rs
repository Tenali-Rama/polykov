//! `attach add`, `ls`, `get` and `rm` against the real server router.

mod common;

use std::fs;

use common::{Home, Run, Server, TempDir, dead_url, run};

/// A logged-in home on its own server, one object (id 1) and a scratch dir for files.
fn setup(name: &str) -> (Server, Home, TempDir) {
    let server = Server::start(name);
    let home = Home::new(name);
    home.login(&server.url);
    ok(&server, &home, &["new", "note", "Receipts"]);
    (server, home, TempDir::new(&format!("{name}-files")))
}

/// Runs `polykov <args>` against `server` and expects exit 0.
fn ok(server: &Server, home: &Home, args: &[&str]) -> Run {
    let out = run(home.polykov().args(args).args(["--server", &server.url]));
    assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
    out
}

/// Bytes that are not valid UTF-8, so a text round-trip would corrupt them.
const BYTES: &[u8] = b"%PDF-1.7\n\x00\xff\xfe binary \xc3\x28 tail";

#[test]
fn add_ls_get_round_trips_the_bytes() {
    let (server, home, files) = setup("att-roundtrip");
    let src = files.path().join("scan.pdf");
    fs::write(&src, BYTES).unwrap();
    let src = src.to_str().unwrap();

    let out = ok(&server, &home, &["attach", "add", "1", src]);
    assert_eq!(out.stdout.trim(), "attached 1");
    let out = ok(
        &server,
        &home,
        &["attach", "add", "1", src, "--name", "copy.bin"],
    );
    assert_eq!(out.stdout.trim(), "attached 2");

    let out = ok(&server, &home, &["attach", "ls", "1"]);
    let size = BYTES.len();
    assert_eq!(
        out.stdout,
        format!("1  scan.pdf  {size} B  application/pdf\n2  copy.bin  {size} B  application/pdf\n")
    );

    // To an explicit path.
    let dst = files.path().join("out.pdf");
    let out = ok(
        &server,
        &home,
        &["attach", "get", "2", "-o", dst.to_str().unwrap()],
    );
    assert_eq!(
        out.stdout.trim(),
        format!("wrote {} ({size} B)", dst.display())
    );
    assert_eq!(fs::read(&dst).unwrap(), BYTES);

    // To the server-given name in the current dir.
    let cwd = TempDir::new("att-roundtrip-cwd");
    let out = run(home.polykov().current_dir(cwd.path()).args([
        "attach",
        "get",
        "1",
        "--server",
        &server.url,
    ]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout.trim(), format!("wrote scan.pdf ({size} B)"));
    assert_eq!(fs::read(cwd.path().join("scan.pdf")).unwrap(), BYTES);
}

#[test]
fn get_refuses_to_overwrite_unless_forced() {
    let (server, home, files) = setup("att-overwrite");
    let src = files.path().join("a.txt");
    fs::write(&src, BYTES).unwrap();
    ok(
        &server,
        &home,
        &["attach", "add", "1", src.to_str().unwrap()],
    );
    let dst = files.path().join("existing.txt");
    fs::write(&dst, "keep me").unwrap();
    let dst = dst.to_str().unwrap();

    let out = run(home
        .polykov()
        .args(["attach", "get", "1", "-o", dst, "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr.trim(),
        format!("polykov: {dst} exists; use --force to overwrite")
    );
    assert_eq!(fs::read_to_string(dst).unwrap(), "keep me");

    ok(
        &server,
        &home,
        &["attach", "get", "1", "-o", dst, "--force"],
    );
    assert_eq!(fs::read(dst).unwrap(), BYTES);
}

#[test]
fn rm_deletes_the_attachment() {
    let (server, home, files) = setup("att-rm");
    let src = files.path().join("a.txt");
    fs::write(&src, "x").unwrap();
    ok(
        &server,
        &home,
        &["attach", "add", "1", src.to_str().unwrap()],
    );

    let out = ok(&server, &home, &["attach", "rm", "1"]);
    assert_eq!(out.stdout.trim(), "removed 1");
    let out = ok(&server, &home, &["attach", "ls", "1"]);
    assert_eq!(out.stdout.trim(), "no attachments");

    let dst = files.path().join("gone.txt");
    let out = run(home.polykov().args([
        "attach",
        "get",
        "1",
        "-o",
        dst.to_str().unwrap(),
        "--server",
        &server.url,
    ]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "polykov: server returned 404: attachment not found"
    );
    assert!(!dst.exists());

    let out = run(home
        .polykov()
        .args(["attach", "rm", "1", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "polykov: server returned 404: attachment not found"
    );
}

#[test]
fn unreadable_file_fails_without_a_request() {
    let home = Home::new("att-missing");
    let files = TempDir::new("att-missing-files");
    let missing = files.path().join("nope.txt");
    let missing = missing.to_str().unwrap();
    // Nothing listens there: any request would fail with "cannot reach server".
    let out = run(home
        .polykov()
        .args(["attach", "add", "1", missing, "--server", &dead_url()]));
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr.trim(),
        format!("polykov: cannot read {missing}: No such file or directory (os error 2)")
    );
}

#[test]
fn add_to_unknown_object_reports_the_server_error() {
    let (server, home, files) = setup("att-unknown");
    let src = files.path().join("a.txt");
    fs::write(&src, "x").unwrap();
    let out = run(home.polykov().args([
        "attach",
        "add",
        "99",
        src.to_str().unwrap(),
        "--server",
        &server.url,
    ]));
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.trim(),
        "polykov: server returned 404: object not found"
    );
}

#[test]
fn json_gives_the_attachments() {
    let (server, home, files) = setup("att-json");
    let src = files.path().join("notes.md");
    fs::write(&src, BYTES).unwrap();

    let out = ok(
        &server,
        &home,
        &["attach", "add", "1", src.to_str().unwrap(), "--json"],
    );
    let attachment = out.json();
    assert_eq!(attachment["id"], 1);
    assert_eq!(attachment["object_id"], 1);
    assert_eq!(attachment["name"], "notes.md");
    assert_eq!(attachment["mime"], "text/markdown");
    assert_eq!(attachment["size"], BYTES.len());

    let out = ok(&server, &home, &["attach", "ls", "1", "--json"]);
    assert_eq!(out.json()["attachments"][0], attachment);

    let dst = files.path().join("out.md");
    let dst = dst.to_str().unwrap();
    let out = ok(&server, &home, &["attach", "get", "1", "-o", dst, "--json"]);
    assert_eq!(
        out.json(),
        serde_json::json!({ "id": 1, "path": dst, "size": BYTES.len() })
    );

    let out = ok(&server, &home, &["attach", "rm", "1", "--json"]);
    assert_eq!(out.json(), serde_json::json!({ "id": 1, "deleted": true }));
}
