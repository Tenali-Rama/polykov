use super::*;

fn d(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
}

fn after(rule: &str, from: &str) -> String {
    next(&Rule::parse(rule).unwrap(), d(from)).to_string()
}

/// The first `n` dates of `rule` after `from`, each counted from the one before.
fn series(rule: &str, from: &str, n: usize) -> Vec<String> {
    let rule = Rule::parse(rule).unwrap();
    let mut date = d(from);
    (0..n)
        .map(|_| {
            date = next(&rule, date);
            date.to_string()
        })
        .collect()
}

#[test]
fn every_freq_with_and_without_interval() {
    for (rule, from, want) in [
        ("FREQ=DAILY", "2026-10-30", "2026-10-31"),
        ("FREQ=DAILY;INTERVAL=3", "2026-10-30", "2026-11-02"),
        ("FREQ=DAILY;INTERVAL=365", "2026-01-01", "2027-01-01"),
        ("FREQ=WEEKLY", "2026-10-30", "2026-11-06"),
        ("FREQ=WEEKLY;INTERVAL=2", "2026-10-30", "2026-11-13"),
        ("FREQ=MONTHLY", "2026-10-30", "2026-11-30"),
        ("FREQ=MONTHLY;INTERVAL=4", "2026-10-15", "2027-02-15"),
        ("FREQ=YEARLY", "2026-10-30", "2027-10-30"),
        ("FREQ=YEARLY;INTERVAL=5", "2026-10-30", "2031-10-30"),
    ] {
        assert_eq!(after(rule, from), want, "{rule} after {from}");
    }
}

#[test]
fn byday_with_several_days_crosses_week_boundaries() {
    // 2026-10-30 is a Friday.
    assert_eq!(
        series("FREQ=WEEKLY;BYDAY=MO,WE,FR", "2026-10-28", 5),
        [
            "2026-10-30",
            "2026-11-02",
            "2026-11-04",
            "2026-11-06",
            "2026-11-09"
        ]
    );
    assert_eq!(after("FREQ=WEEKLY;BYDAY=SU", "2026-11-01"), "2026-11-08");
    assert_eq!(after("FREQ=WEEKLY;BYDAY=SU", "2026-10-30"), "2026-11-01");
}

#[test]
fn byday_with_interval_skips_whole_weeks() {
    assert_eq!(
        series("FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,FR", "2026-10-28", 4),
        ["2026-10-30", "2026-11-10", "2026-11-13", "2026-11-24"]
    );
}

#[test]
fn month_day_past_the_end_clamps_to_the_last_day() {
    assert_eq!(
        series("FREQ=MONTHLY;BYMONTHDAY=31", "2026-12-31", 4),
        ["2027-01-31", "2027-02-28", "2027-03-31", "2027-04-30"]
    );
    assert_eq!(
        after("FREQ=MONTHLY;BYMONTHDAY=31", "2028-01-31"),
        "2028-02-29"
    );
    assert_eq!(after("FREQ=MONTHLY", "2026-01-31"), "2026-02-28");
}

#[test]
fn month_day_later_this_month_comes_first() {
    assert_eq!(
        after("FREQ=MONTHLY;BYMONTHDAY=15", "2026-10-10"),
        "2026-10-15"
    );
    assert_eq!(
        after("FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=15", "2026-10-15"),
        "2026-12-15"
    );
}

#[test]
fn feb_29_yearly_clamps_in_common_years() {
    assert_eq!(after("FREQ=YEARLY", "2028-02-29"), "2029-02-28");
    assert_eq!(after("FREQ=YEARLY;INTERVAL=4", "2028-02-29"), "2032-02-29");
}

#[test]
fn year_rollover() {
    assert_eq!(after("FREQ=DAILY", "2026-12-31"), "2027-01-01");
    assert_eq!(after("FREQ=WEEKLY;BYDAY=MO", "2026-12-31"), "2027-01-04");
    assert_eq!(after("FREQ=MONTHLY", "2026-12-15"), "2027-01-15");
    assert_eq!(after("FREQ=MONTHLY;INTERVAL=3", "2026-11-30"), "2027-02-28");
}

#[test]
fn next_saturates_at_the_last_date() {
    let rule = Rule::parse("FREQ=YEARLY").unwrap();
    assert_eq!(next(&rule, NaiveDate::MAX), NaiveDate::MAX);
}

#[test]
fn parse_errors_name_the_reason() {
    for (text, reason) in [
        ("", "expected KEY=VALUE, got \"\""),
        ("INTERVAL=2", "FREQ is required"),
        (
            "FREQ=HOURLY",
            "FREQ must be DAILY, WEEKLY, MONTHLY or YEARLY",
        ),
        ("FREQ=DAILY;FREQ=WEEKLY", "FREQ given twice"),
        ("FREQ=DAILY;INTERVAL=0", "INTERVAL must be 1 to 365"),
        ("FREQ=DAILY;INTERVAL=366", "INTERVAL must be 1 to 365"),
        ("FREQ=DAILY;INTERVAL=x", "INTERVAL must be 1 to 365"),
        ("FREQ=DAILY;COUNT=3", "unknown part COUNT"),
        ("FREQ=WEEKLY;BYDAY=XX", "BYDAY has an unknown day \"XX\""),
        ("FREQ=WEEKLY;BYDAY=", "BYDAY has an unknown day \"\""),
        ("FREQ=DAILY;BYDAY=MO", "BYDAY needs FREQ=WEEKLY"),
        ("FREQ=MONTHLY;BYMONTHDAY=0", "BYMONTHDAY must be 1 to 31"),
        ("FREQ=MONTHLY;BYMONTHDAY=32", "BYMONTHDAY must be 1 to 31"),
        ("FREQ=YEARLY;BYMONTHDAY=1", "BYMONTHDAY needs FREQ=MONTHLY"),
        ("FREQ=DAILY;", "expected KEY=VALUE, got \"\""),
    ] {
        assert_eq!(Rule::parse(text), Err(reason.to_owned()), "{text:?}");
    }
}

#[test]
fn canonical_form_round_trips() {
    for (text, canonical) in [
        ("FREQ=DAILY", "FREQ=DAILY"),
        ("freq=daily;interval=1", "FREQ=DAILY"),
        ("INTERVAL=2;FREQ=WEEKLY", "FREQ=WEEKLY;INTERVAL=2"),
        (
            "FREQ=WEEKLY;BYDAY=SU,MO,SU,FR",
            "FREQ=WEEKLY;BYDAY=MO,FR,SU",
        ),
        (
            "BYMONTHDAY=31;FREQ=MONTHLY;INTERVAL=3",
            "FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=31",
        ),
        ("FREQ=YEARLY;INTERVAL=365", "FREQ=YEARLY;INTERVAL=365"),
    ] {
        let rule = Rule::parse(text).unwrap();
        assert_eq!(rule.to_string(), canonical, "{text}");
        assert_eq!(Rule::parse(canonical), Ok(rule), "{canonical}");
    }
}
