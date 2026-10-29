//! The NL parser benchmark (spec §5): every candidate over `parse/bench/cases.jsonl`.
//!
//! `cargo run -p polykov-parse --release --example bench` prints, per candidate and slice,
//! whole-parse exact match, per-field accuracy, the false-token rate on lines with nothing to
//! extract, p50/p99 latency, peak RAM and model size. `-- --misses <candidate>` lists that
//! candidate's wrong parses instead, and `-- --save-model` retrains the tagger and writes the
//! shipped `src/tagger/model.bin`.

#[path = "../bench/score.rs"]
mod score;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use polykov_parse::{Labeled, Parser, RulesParser, TaggerParser};
use score::{Case, FIELDS};

/// Counts live heap bytes so the bench can report each candidate's peak.
struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(live, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// A candidate: its name and how to build it, returning the parser and its model size.
struct Candidate {
    name: &'static str,
    build: fn() -> (Box<dyn Parser>, usize),
}

const CANDIDATES: &[Candidate] = &[
    Candidate {
        name: "rules",
        build: || (Box::new(RulesParser), 0),
    },
    Candidate {
        name: "tagger",
        build: || {
            let tagger = train();
            let bytes = tagger.model_bytes();
            (Box::new(tagger), bytes)
        },
    },
    Candidate {
        name: "default",
        build: || (polykov_parse::default_parser(), MODEL_BIN.len()),
    },
];

/// The shipped model, as `default_parser()` bundles it.
const MODEL_BIN: &[u8] = include_bytes!("../src/tagger/model.bin");

/// The tagger trained on `train.jsonl` (8 passes).
fn train() -> TaggerParser {
    let data: Vec<Labeled> = score::load::<score::TrainCase>("train.jsonl")
        .iter()
        .map(|t| Labeled {
            input: t.case.input.clone(),
            ctx: t.case.context(),
            labels: t.labels(),
        })
        .collect();
    TaggerParser::train(&data, 8)
}

/// Timed repetitions of each case; its latency is their median, which keeps a busy host's
/// preemptions out of the percentiles.
const REPS: usize = 20;
const SLICES: [&str; 5] = ["all", "casual", "standard", "cold", "long"];

fn main() {
    let cases: Vec<Case> = score::load("cases.jsonl");
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--save-model") {
        let path = format!("{}/src/tagger/model.bin", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(&path, train().to_bytes()).unwrap_or_else(|e| panic!("{path}: {e}"));
        return println!("wrote {path}");
    }
    if let [_, flag, name] = args.as_slice()
        && flag == "--misses"
    {
        return misses(&cases, name);
    }
    println!("{} cases\n", cases.len());
    println!(
        "| candidate | slice | n | exact | false-token | p50 µs | p99 µs | peak RAM KiB | model KiB |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    let mut fields = Vec::new();
    for c in CANDIDATES {
        let base = LIVE.load(Ordering::Relaxed);
        PEAK.store(base, Ordering::Relaxed);
        let (parser, model) = (c.build)();
        let mut runs = Vec::with_capacity(cases.len());
        for case in &cases {
            let ctx = case.context();
            let mut times = Vec::with_capacity(REPS);
            let mut parsed = parser.parse(&case.input, &ctx);
            for _ in 0..REPS {
                let t = Instant::now();
                parsed = parser.parse(&case.input, &ctx);
                times.push(t.elapsed().as_secs_f64() * 1e6);
            }
            times.sort_by(f64::total_cmp);
            runs.push((case, parsed, times[REPS / 2]));
        }
        let peak = PEAK.load(Ordering::Relaxed).saturating_sub(base);
        drop(parser);
        for slice in SLICES {
            let rows: Vec<_> = runs
                .iter()
                .filter(|(case, _, _)| slice == "all" || case.slice == slice)
                .collect();
            let n = rows.len() as f64;
            let mut field = [0.0; 9];
            let (mut exact, mut plain, mut false_tok) = (0.0, 0.0, 0.0);
            let mut times: Vec<f64> = Vec::new();
            for (case, parsed, t) in &rows {
                let hits = case.expected.hits(parsed);
                for (f, h) in field.iter_mut().zip(hits) {
                    *f += f64::from(u8::from(h));
                }
                exact += f64::from(u8::from(hits.iter().all(|h| *h)));
                if case.expected.no_token() {
                    plain += 1.0;
                    false_tok += f64::from(u8::from(!parsed.tokens.is_empty()));
                }
                times.push(*t);
            }
            times.sort_by(f64::total_cmp);
            let pct = |q: f64| times[((times.len() - 1) as f64 * q).round() as usize];
            println!(
                "| {} | {slice} | {n} | {:.1} | {} | {:.1} | {:.1} | {} | {} |",
                c.name,
                100.0 * exact / n,
                if plain > 0.0 {
                    format!("{:.1} ({plain})", 100.0 * false_tok / plain)
                } else {
                    "-".into()
                },
                pct(0.5),
                pct(0.99),
                peak / 1024,
                model / 1024,
            );
            fields.push((c.name, slice, field.map(|f| 100.0 * f / n)));
        }
    }
    println!("\n| candidate | slice | {} |", FIELDS.join(" | "));
    println!("|---|---|{}", "---|".repeat(FIELDS.len()));
    for (name, slice, f) in fields {
        let cells: Vec<String> = f.iter().map(|x| format!("{x:.1}")).collect();
        println!("| {name} | {slice} | {} |", cells.join(" | "));
    }
}

/// Prints each case the named candidate gets wrong, with the fields that differ.
fn misses(cases: &[Case], name: &str) {
    let Some(c) = CANDIDATES.iter().find(|c| c.name == name) else {
        let names: Vec<_> = CANDIDATES.iter().map(|c| c.name).collect();
        eprintln!("unknown candidate {name}; one of {names:?}");
        std::process::exit(2);
    };
    let (parser, _) = (c.build)();
    for case in cases {
        let parsed = parser.parse(&case.input, &case.context());
        let hits = case.expected.hits(&parsed);
        let wrong: Vec<_> = FIELDS.iter().zip(hits).filter(|(_, h)| !h).collect();
        if !wrong.is_empty() {
            let names: Vec<_> = wrong.iter().map(|(f, _)| **f).collect();
            println!("[{}] {:?} ({})", case.slice, case.input, names.join(","));
            println!("    want {:?}\n    got  {parsed:?}", case.expected);
        }
    }
}
