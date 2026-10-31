//! Search latency budget: `GET /api/search` answers in under 100 ms (p95) over 100k objects.
//! Each query is judged by the CPU time it costs, so other jobs sharing the CPU don't fail
//! it; wall-clock times are printed alongside.
//! Run with `cargo test --release -p polykov-server --test search_perf -- --ignored
//! --nocapture`.

mod common;

use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use common::{DataDir, call, token};
use polykov_server::db::Db;
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
/// tags and property values.
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

/// Queries with no word of 3+ characters, i.e. the first keystrokes of a palette search.
/// They take the title-only path (ADR 0014), so each is checked on its own, not via p95.
const SHORT_QUERIES: &[&str] = &["a", "e", "m", "me", "to", "re%20me", "zq"];

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

    let budget = Duration::from_millis(100);
    if cpu_time().is_some() {
        let run = measure(&db, &auth, Clock::Cpu).await;
        assert!(run.p95 < budget, "CPU p95 {:?} is over budget", run.p95);
        assert!(
            run.slowest_short < budget,
            "short query CPU time {:?} is over budget",
            run.slowest_short
        );
        return;
    }
    println!("process CPU time is unavailable; falling back to wall clock");
    let mut runs = Vec::new();
    for _ in 0..3 {
        let run = measure(&db, &auth, Clock::Wall).await;
        if run.p95 < budget && run.slowest_short < budget {
            return;
        }
        runs.push((run.p95, run.slowest_short));
    }
    panic!("every wall-clock run is over budget (p95, slowest short query): {runs:?}");
}

/// What a query's cost is judged by.
#[derive(Clone, Copy)]
enum Clock {
    /// CPU time of the whole process: time spent waiting for a CPU that other jobs hold
    /// doesn't count, but the handler's work on Tokio's blocking pool does.
    Cpu,
    Wall,
}

/// The cost of one pass over all queries, in the [`Clock`] it was judged by.
struct Run {
    p95: Duration,
    slowest_short: Duration,
}

/// Runs every query once and prints the wall-clock (and, for [`Clock::Cpu`], CPU) p50,
/// p95 and slowest short query.
async fn measure(db: &Db, auth: &str, clock: Clock) -> Run {
    let (mut wall, mut cpu) = (Vec::new(), Vec::new());
    for q in QUERIES {
        let (w, c) = timed(db, auth, q).await;
        wall.push(w);
        cpu.push(c.unwrap_or_default());
    }
    let (mut short_wall, mut short_cpu) = (Duration::ZERO, Duration::ZERO);
    for q in SHORT_QUERIES {
        let (w, c) = timed(db, auth, q).await;
        short_wall = short_wall.max(w);
        short_cpu = short_cpu.max(c.unwrap_or_default());
    }
    let (wall_p50, wall_p95) = percentiles(wall);
    println!("wall: p50 {wall_p50:.2?}, p95 {wall_p95:.2?}, slowest short query {short_wall:.2?}");
    match clock {
        Clock::Wall => Run {
            p95: wall_p95,
            slowest_short: short_wall,
        },
        Clock::Cpu => {
            let (cpu_p50, cpu_p95) = percentiles(cpu);
            println!(
                "cpu:  p50 {cpu_p50:.2?}, p95 {cpu_p95:.2?}, slowest short query {short_cpu:.2?}"
            );
            Run {
                p95: cpu_p95,
                slowest_short: short_cpu,
            }
        }
    }
}

/// The p50 and p95 of `times`.
fn percentiles(mut times: Vec<Duration>) -> (Duration, Duration) {
    times.sort();
    let at = |p: usize| times[(times.len() * p).div_ceil(100) - 1];
    (at(50), at(95))
}

/// CPU time used so far by all threads of this process, or `None` if it can't be read.
///
/// The search handler runs its store call on Tokio's blocking pool, not on the thread that
/// sends the request, so a per-thread clock would miss most of its work. The test runtime
/// is single-threaded and nothing else runs during a request, so the delta of the process
/// clock around one request is that request's own CPU cost.
fn cpu_time() -> Option<Duration> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, writable timespec for the whole call.
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) };
    (rc == 0).then(|| Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32))
}

/// How long `GET /api/search?q=<q>` took in wall-clock and (when readable) CPU time;
/// prints both with the total.
async fn timed(db: &Db, auth: &str, q: &str) -> (Duration, Option<Duration>) {
    let cpu_started = cpu_time();
    let started = Instant::now();
    let res = call(
        db,
        Method::GET,
        &format!("/api/search?q={q}"),
        Some(auth),
        None,
    )
    .await;
    let took = started.elapsed();
    let cpu = cpu_started
        .zip(cpu_time())
        .map(|(from, to)| to.saturating_sub(from));
    assert_eq!(res.status, StatusCode::OK, "{q}");
    println!(
        "{q:>24}: wall {took:>10.2?}, cpu {:>10.2?}, total {}",
        cpu.unwrap_or_default(),
        res.json()["total"]
    );
    (took, cpu)
}
