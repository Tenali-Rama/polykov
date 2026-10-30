//! `add`, the GTD list commands, `done` and `reopen` against the real server router.

mod common;

use chrono::{Days, Local, NaiveDate};
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

/// `PATCH /api/tasks/{id}` with the stored session, for fields the CLI can't set yet.
fn patch(server: &Server, home: &Home, id: u64, body: Value) {
    let config = home.config();
    let token = config
        .lines()
        .find_map(|l| l.strip_prefix("token = \""))
        .and_then(|l| l.strip_suffix('"'))
        .unwrap();
    let res = ureq::patch(format!("{}/api/tasks/{id}", server.url))
        .header("Authorization", format!("Bearer {token}"))
        .send_json(body)
        .unwrap();
    assert_eq!(res.status(), 200);
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn ymd(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[test]
fn add_prints_the_id_title_and_parsed_fields() {
    let (server, home) = setup("task-add");
    let out = ok(
        &server,
        &home,
        &[
            "add", "call", "mom", "tmrw", "5pm", "30m", "p2", "#family", "every", "sunday",
        ],
    );
    assert_eq!(
        out.stdout,
        "added 1  call mom\ntomorrow 17:00 · 30m · p2 · #family · every sunday\n"
    );
    let out = ok(&server, &home, &["add", "buy milk"]);
    assert_eq!(out.stdout, "added 2  buy milk\ninbox\n");

    let out = ok(&server, &home, &["add", "plain", "--json"]);
    let task = out.json();
    assert_eq!(task["id"], 3);
    assert_eq!(task["title"], "plain");
    assert_eq!(task["task"]["status"], "open");
}

#[test]
fn add_with_a_400_prints_the_server_text_and_exits_1() {
    let (server, home) = setup("task-add-400");
    let out = run(home
        .polykov()
        .args(["add", "plan +nowhere", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(out.stdout, "");
    assert_eq!(out.stderr, "polykov: unknown project: nowhere\n");

    let out = run(home
        .polykov()
        .args(["add", "+nowhere", "--json", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(out.err_json(), json!({"error": "unknown project: nowhere"}));
    let out = ok(&server, &home, &["inbox"]);
    assert_eq!(out.stdout, "nothing in inbox\n");
}

#[test]
fn each_list_shows_its_tasks_one_per_line() {
    let (server, home) = setup("task-lists");
    for name in ["today", "inbox", "upcoming", "someday", "anytime"] {
        let out = ok(&server, &home, &[name]);
        assert_eq!(out.stdout, format!("nothing in {name}\n"));
    }
    let yesterday = ymd(today() - Days::new(1));
    let tomorrow = ymd(today() + Days::new(1));
    let later = ymd(today() + Days::new(3));
    ok(&server, &home, &["add", "inbox thing"]); // 1
    ok(
        &server,
        &home,
        &["add", &format!("late bill {yesterday} p1 #home")],
    ); // 2
    ok(&server, &home, &["add", &format!("dentist {tomorrow} 9am")]); // 3
    ok(&server, &home, &["add", &format!("trip {later}")]); // 4
    ok(&server, &home, &["add", &format!("pack {tomorrow}")]); // 5
    ok(&server, &home, &["add", "learn piano"]); // 6
    patch(&server, &home, 6, json!({"task": {"someday": true}}));
    ok(&server, &home, &["add", "step one"]); // 7
    patch(&server, &home, 7, json!({"task": {"parent_id": 2}}));

    let out = ok(&server, &home, &["today"]);
    assert_eq!(
        out.stdout,
        format!("2  p1  late bill [1/1]  !{yesterday}  #home\n")
    );
    let out = ok(&server, &home, &["inbox"]);
    assert_eq!(out.stdout, "1  p4  inbox thing\n");
    let out = ok(&server, &home, &["someday"]);
    assert_eq!(out.stdout, "6  p4  learn piano\n");
    let out = ok(&server, &home, &["anytime"]);
    assert_eq!(
        out.stdout,
        format!("2  p1  late bill [1/1]  !{yesterday}  #home\n")
    );

    let out = ok(&server, &home, &["upcoming"]);
    assert_eq!(
        out.stdout,
        format!(
            "{tomorrow}\n  3  p4  dentist  {tomorrow} 09:00\n  5  p4  pack  {tomorrow}\n\
             {later}\n  4  p4  trip  {later}\n"
        )
    );
    let out = ok(&server, &home, &["upcoming", "--limit", "1"]);
    assert_eq!(
        out.stdout,
        format!("{tomorrow}\n  3  p4  dentist  {tomorrow} 09:00\n(1 of 3 shown)\n")
    );

    let out = ok(&server, &home, &["upcoming", "--json"]);
    let body = out.json();
    assert_eq!(body["total"], 3);
    let ids: Vec<&Value> = body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| &t["id"])
        .collect();
    assert_eq!(ids, [3, 5, 4]);
    let out = ok(
        &server,
        &home,
        &["today", "--project", "99", "--area", "98"],
    );
    assert_eq!(out.stdout, "nothing in today\n");
}

#[test]
fn done_reports_the_next_occurrence_and_reopen_reopens() {
    let (server, home) = setup("task-done");
    ok(&server, &home, &["add", "water plants every day"]); // 1, today
    ok(&server, &home, &["add", "one off"]); // 2
    let tomorrow = ymd(today() + Days::new(1));

    let out = ok(&server, &home, &["done", "1", "2"]);
    assert_eq!(out.stdout, format!("done 1\nnext 3  {tomorrow}\ndone 2\n"));
    let out = ok(&server, &home, &["inbox"]);
    assert_eq!(out.stdout, "nothing in inbox\n");

    let out = ok(&server, &home, &["reopen", "2"]);
    assert_eq!(out.stdout, "reopened 2\n");
    let out = ok(&server, &home, &["inbox"]);
    assert_eq!(out.stdout, "2  p4  one off\n");

    let out = ok(&server, &home, &["done", "3", "--json"]);
    let body = out.json();
    let done = body["done"].as_array().unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0]["task"]["id"], 3);
    assert_eq!(done[0]["task"]["task"]["status"], "done");
    assert_eq!(done[0]["next"]["id"], 4);

    let out = ok(&server, &home, &["reopen", "3", "--json"]);
    assert_eq!(out.json()["task"]["status"], "open");

    let out = run(home.polykov().args(["done", "99", "--server", &server.url]));
    assert_eq!(out.code, 1);
    assert_eq!(out.stderr, "polykov: server returned 404: task not found\n");
}
