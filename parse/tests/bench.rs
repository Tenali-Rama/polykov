//! Regression gate for the shipped parser: `default_parser()` over the 030 benchmark must stay
//! within one point of the whole-parse exact match recorded in `docs/models/nl-parser.md`.

#[path = "../bench/score.rs"]
#[allow(dead_code)] // shared with the bench example, which uses all of it
mod score;

use chrono::NaiveDate;
use polykov_parse::{Context, RulesParser, default_parser};
use score::Case;

/// Whole-parse exact match of the winner, in percent, as recorded in the doc.
const RECORDED_EXACT: f64 = 91.6;

fn exact(parser: &dyn polykov_parse::Parser, cases: &[Case]) -> f64 {
    let hits = cases
        .iter()
        .filter(|c| {
            let parsed = parser.parse(&c.input, &c.context());
            c.expected.hits(&parsed).iter().all(|h| *h)
        })
        .count();
    100.0 * hits as f64 / cases.len() as f64
}

#[test]
fn default_parser_keeps_its_recorded_score() {
    let cases: Vec<Case> = score::load("cases.jsonl");
    let got = exact(default_parser().as_ref(), &cases);
    assert!(
        got >= RECORDED_EXACT - 1.0,
        "exact match {got:.1}% fell below the recorded {RECORDED_EXACT}% minus 1 point"
    );
}

#[test]
fn default_parser_reads_yesterday_and_every_n_weeks_on() {
    let ctx = Context {
        today: NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(),
        projects: vec![],
        areas: vec![],
    };
    let p = default_parser().parse("call bob yesterday", &ctx);
    assert_eq!(
        (p.title.as_str(), p.scheduled.as_deref()),
        ("call bob", Some("2026-10-29"))
    );
    let p = default_parser().parse("standup every 2 weeks on tuesday 9am", &ctx);
    assert_eq!(p.title, "standup");
    assert_eq!(p.scheduled.as_deref(), Some("2026-11-03T09:00"));
    assert_eq!(
        p.recurrence.map(|r| r.rule).as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=2;BYDAY=TU")
    );
}

#[test]
fn default_parser_reads_bare_every_other_week() {
    let ctx = Context {
        today: NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(),
        projects: vec![],
        areas: vec![],
    };
    let p = default_parser().parse("water plants every other week", &ctx);
    assert_eq!(p.title, "water plants");
    assert_eq!(p.scheduled.as_deref(), Some("2026-10-30"));
    assert_eq!(
        p.recurrence.map(|r| r.rule).as_deref(),
        Some("FREQ=WEEKLY;INTERVAL=2")
    );
    let p = default_parser().parse("visit every other cousin", &ctx);
    assert_eq!(p.title, "visit every other cousin");
    assert_eq!(p.recurrence, None);
}

#[test]
fn default_parser_beats_the_baseline() {
    let cases: Vec<Case> = score::load("cases.jsonl");
    let winner = exact(default_parser().as_ref(), &cases);
    let baseline = exact(&RulesParser, &cases);
    assert!(winner > baseline, "{winner:.1}% vs baseline {baseline:.1}%");
}

#[test]
fn benchmark_has_the_required_slices() {
    let cases: Vec<Case> = score::load("cases.jsonl");
    let count = |s: &str| cases.iter().filter(|c| c.slice == s).count();
    assert!(cases.len() >= 300, "{} cases", cases.len());
    assert!(count("casual") >= 60);
    assert!(
        cases
            .iter()
            .filter(|c| c.slice == "cold")
            .all(|c| c.projects.is_empty() && c.areas.is_empty())
    );
    assert!(
        cases
            .iter()
            .filter(|c| c.slice == "long")
            .all(|c| c.projects.len() >= 50 && c.areas.len() >= 50)
    );
    assert!(count("cold") > 0 && count("long") > 0 && count("standard") > 0);
}
