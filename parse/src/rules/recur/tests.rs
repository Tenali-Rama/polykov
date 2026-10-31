//! Unit tests for recurrence phrases.

use super::*;
use crate::words::split;

fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// Parses at 2026-10-30, a Friday: (words used, rule, from, first occurrence).
fn rep(input: &str) -> Option<(usize, String, Anchor, NaiveDate)> {
    rep_on(input, ymd(2026, 10, 30))
}

fn rep_on(input: &str, today: NaiveDate) -> Option<(usize, String, Anchor, NaiveDate)> {
    let (words, _) = split(input);
    let (n, r) = repeat(
        &Run {
            input,
            words: &words,
        },
        today,
    )?;
    Some((n, r.recurrence.rule, r.recurrence.from, r.first))
}

const S: Anchor = Anchor::Scheduled;

#[test]
fn every_unit_and_ly_words() {
    let today = ymd(2026, 10, 30);
    for (input, n, rule) in [
        ("every day", 2, "FREQ=DAILY"),
        ("every week", 2, "FREQ=WEEKLY"),
        ("every month", 2, "FREQ=MONTHLY"),
        ("every year", 2, "FREQ=YEARLY"),
        ("daily", 1, "FREQ=DAILY"),
        ("weekly", 1, "FREQ=WEEKLY"),
        ("Monthly", 1, "FREQ=MONTHLY"),
        ("yearly", 1, "FREQ=YEARLY"),
    ] {
        assert_eq!(rep(input), Some((n, rule.into(), S, today)), "{input}");
    }
}

#[test]
fn every_weekday_is_monday_to_friday() {
    let r = rep("every weekday").unwrap();
    assert_eq!(r.1, "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR");
    assert_eq!(r.3, ymd(2026, 10, 30));
    assert_eq!(
        rep_on("every weekday", ymd(2026, 10, 31)).unwrap().3,
        ymd(2026, 11, 2)
    );
}

#[test]
fn every_n_units() {
    assert_eq!(rep("every 2 days").unwrap().1, "FREQ=DAILY;INTERVAL=2");
    assert_eq!(rep("every 3 weeks").unwrap().1, "FREQ=WEEKLY;INTERVAL=3");
    assert_eq!(rep("every 6 months").unwrap().1, "FREQ=MONTHLY;INTERVAL=6");
    assert_eq!(rep("every 2 years").unwrap().1, "FREQ=YEARLY;INTERVAL=2");
    assert_eq!(rep("every 1 week").unwrap().1, "FREQ=WEEKLY");
    assert_eq!(rep("every 2 apples"), None);
    assert_eq!(rep("every 0 days"), None);
    assert_eq!(rep("every 400 days"), None);
}

#[test]
fn every_weekday_list() {
    assert_eq!(
        rep("every sunday"),
        Some((2, "FREQ=WEEKLY;BYDAY=SU".into(), S, ymd(2026, 11, 1)))
    );
    assert_eq!(
        rep("every fri").unwrap().3,
        ymd(2026, 10, 30),
        "today counts"
    );
    let r = rep("every thu, and mon").unwrap();
    assert_eq!(
        (r.0, r.1.as_str(), r.3),
        (4, "FREQ=WEEKLY;BYDAY=MO,TH", ymd(2026, 11, 2))
    );
    assert_eq!(rep("every mon and wed").unwrap().0, 4);
    assert_eq!(
        rep("every mon, wed, fri").unwrap().1,
        "FREQ=WEEKLY;BYDAY=MO,WE,FR"
    );
    assert_eq!(
        rep("every mon and more").unwrap().0,
        2,
        "a trailing `and` stays text"
    );
    assert_eq!(rep("every apple"), None);
}

#[test]
fn every_n_weeks_on_weekdays() {
    assert_eq!(
        rep("every 2 weeks on tuesday"),
        Some((
            5,
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU".into(),
            S,
            ymd(2026, 11, 3)
        ))
    );
    let r = rep("every week on mon, thu").unwrap();
    assert_eq!(
        (r.0, r.1.as_str(), r.3),
        (5, "FREQ=WEEKLY;BYDAY=MO,TH", ymd(2026, 11, 2))
    );
    assert_eq!(
        rep("every other week on fri"),
        Some((
            5,
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=FR".into(),
            S,
            ymd(2026, 10, 30)
        ))
    );
    assert_eq!(
        rep("every! 3 weeks on sat and sun").map(|r| (r.0, r.1, r.3)),
        Some((
            7,
            "FREQ=WEEKLY;INTERVAL=3;BYDAY=SA,SU".into(),
            ymd(2026, 10, 31)
        ))
    );
    assert_eq!(
        rep("every 2 weeks on time").unwrap().0,
        3,
        "`on` stays text"
    );
    assert_eq!(
        rep("every 2 days on tue").unwrap().1,
        "FREQ=DAILY;INTERVAL=2"
    );
    assert_eq!(rep("every 0 weeks on tue"), None);
}

#[test]
fn every_other_week_is_every_two_weeks() {
    let two = rep("every 2 weeks");
    assert_eq!(
        two,
        Some((3, "FREQ=WEEKLY;INTERVAL=2".into(), S, ymd(2026, 10, 30)))
    );
    assert_eq!(rep("every other week"), two);
    assert_eq!(rep("every other week on"), two, "a bare `on` stays text");
    assert_eq!(rep("every! other week").unwrap().2, Anchor::Completion);
    assert_eq!(rep("every other"), None);
    assert_eq!(rep("every other month"), None);
}

#[test]
fn malformed_counts_span_the_phrase() {
    let span = |input: &str| {
        let (words, _) = split(input);
        malformed(&Run {
            input,
            words: &words,
        })
    };
    assert_eq!(span("every 0 weeks on tue"), Some(5));
    assert_eq!(span("every! two week on mon and fri x"), Some(7));
    assert_eq!(span("every 0 days on tue"), None);
    assert_eq!(span("every 0 weeks tue"), None);
}

#[test]
fn every_month_day() {
    assert_eq!(
        rep("every 15th"),
        Some((2, "FREQ=MONTHLY;BYMONTHDAY=15".into(), S, ymd(2026, 11, 15)))
    );
    assert_eq!(rep("every 30th").unwrap().3, ymd(2026, 10, 30));
    assert_eq!(rep("every 31st").unwrap().3, ymd(2026, 10, 31));
    assert_eq!(
        rep_on("every 31st", ymd(2026, 11, 2)).unwrap().3,
        ymd(2026, 11, 30),
        "clamps"
    );
    assert_eq!(
        rep_on("every 30th", ymd(2027, 2, 1)).unwrap().3,
        ymd(2027, 2, 28)
    );
    assert_eq!(rep("every 32nd"), None);
}

#[test]
fn bang_counts_from_completion() {
    assert_eq!(rep("every! 3 days").unwrap().2, Anchor::Completion);
    assert_eq!(rep("every! week").unwrap().2, Anchor::Completion);
    assert_eq!(rep("every! sun").unwrap().2, Anchor::Completion);
    assert_eq!(rep("daily").unwrap().2, Anchor::Scheduled);
}
