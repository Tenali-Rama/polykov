//! Search latency budget: `GET /api/search` answers in under 100 ms (p95) over 100k objects.
//! Run with `cargo test --release -p polykov-server --test search_perf -- --ignored
//! --nocapture`.

mod common;

use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use common::{DataDir, call, token};
use rusqlite::params;

const OBJECTS: usize = 100_000;

const VERBS: &[&str] = &[
    "Review", "Draft", "Call", "Plan", "Buy", "Fix", "Write", "Prepare", "Book", "Schedule",
    "Clean", "Update", "Send", "Organize", "Research", "Cancel", "Renew", "Read", "Pay", "Email",
];
const THINGS: &[&str] = &[
    "meeting notes",
    "groceries",
    "quarterly report",
    "dentist appointment",
    "car insurance",
    "birthday present",
    "project proposal",
    "budget spreadsheet",
    "flight tickets",
    "invoice",
    "presentation slides",
    "garden fence",
    "tax return",
    "client feedback",
    "kitchen sink",
    "team retrospective",
    "blog post",
    "library books",
    "gym membership",
    "website redesign",
    "landlord",
    "passport renewal",
    "conference talk",
    "backup drive",
    "newsletter",
];
const NAMES: &[&str] = &[
    "Alice", "Bob", "Carmen", "Dmitri", "Elena", "Farid", "Grace", "Hiro", "Ines", "Jonas",
];
const TAGS: &[&str] = &[
    "work/clienta",
    "work/clientb/design",
    "home",
    "home/garden",
    "health",
    "finance/taxes",
    "errands",
    "learning/rust",
    "travel",
    "family",
];
const TYPES: &[&str] = &["task", "note", "project", "event", "habit"];

/// Queries of every kind the search supports: prefix, substring, one typo, multi-word,
/// tags, property values and a short word.
const QUERIES: &[&str] = &[
    "meet",
    "groc",
    "ocer",
    "invoi",
    "port",
    "groceris",
    "meetimg",
    "presentaton",
    "budjet",
    "dentst%20appointment",
    "quarterly%20report",
    "clienta",
    "work%2Fclientb",
    "carmen",
    "lisbon",
    "renewal",
    "spreadsheet%20alice",
    "retro",
    "garden%20fence",
    "xyzzyq",
];

/// A deterministic pseudo-random sequence (xorshift), so every run seeds the same data.
struct Rng(u64);

impl Rng {
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        items[(self.0 % items.len() as u64) as usize]
    }
}

#[tokio::test]
#[ignore = "perf budget; run with --release -- --ignored"]
async fn search_p95_is_under_100ms_on_100k_objects() {
    let (dir, db) = DataDir::new("search-perf");
    let auth = token(&db).await;

    let seeding = Instant::now();
    let mut conn = dir.sql();
    let tx = conn.transaction().unwrap();
    {
        let mut object = tx
            .prepare("INSERT INTO objects (type, title, properties) VALUES (?1, ?2, ?3)")
            .unwrap();
        let mut tag = tx
            .prepare("INSERT OR IGNORE INTO object_tags (object_id, tag) VALUES (?1, ?2)")
            .unwrap();
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for i in 0..OBJECTS {
            let title = format!(
                "{} {} for {} #{i}",
                rng.pick(VERBS),
                rng.pick(THINGS),
                rng.pick(NAMES)
            );
            let place = rng.pick(&["Lisbon", "Kyoto", "Berlin", "Austin", "Nairobi", "Oslo"]);
            let properties = format!(
                r#"{{"place":"{place}","owner":"{}","n":{i}}}"#,
                rng.pick(NAMES)
            );
            object
                .execute(params![rng.pick(TYPES), title, properties])
                .unwrap();
            let id = tx.last_insert_rowid();
            for _ in 0..2 {
                tag.execute(params![id, rng.pick(TAGS)]).unwrap();
            }
        }
    }
    tx.commit().unwrap();
    println!("seeded {OBJECTS} objects in {:?}", seeding.elapsed());

    let mut times: Vec<Duration> = Vec::new();
    for q in QUERIES {
        let started = Instant::now();
        let res = call(
            &db,
            Method::GET,
            &format!("/api/search?q={q}"),
            Some(&auth),
            None,
        )
        .await;
        let took = started.elapsed();
        assert_eq!(res.status, StatusCode::OK, "{q}");
        println!("{q:>24}: {took:>10.2?}, total {}", res.json()["total"]);
        times.push(took);
    }
    times.sort();
    let at = |p: usize| times[(times.len() * p).div_ceil(100) - 1];
    let (p50, p95) = (at(50), at(95));
    println!("p50 {p50:.2?}, p95 {p95:.2?}");
    assert!(
        p95 < Duration::from_millis(100),
        "p95 {p95:?} is over budget"
    );
}
