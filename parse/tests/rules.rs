//! Whole-line behaviour of `RulesParser`. 2026-10-30, the `today` here, is a Friday.

use chrono::NaiveDate;
use polykov_parse::{Anchor, Context, Kind, Parsed, Parser, Recurrence, RulesParser};

fn ctx() -> Context {
    Context {
        today: NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(),
        projects: vec!["Home Reno".into(), "Polykov".into()],
        areas: vec!["Health".into(), "Side Work".into()],
    }
}

fn parse(input: &str) -> Parsed {
    RulesParser.parse(input, &ctx())
}

fn kinds(p: &Parsed) -> Vec<(Kind, &str)> {
    p.tokens.iter().map(|t| (t.kind, t.text.as_str())).collect()
}

#[test]
fn spec_example() {
    let input = "call mom tmrw 5pm 30m p2 #family every sunday";
    let p = parse(input);
    assert_eq!(p.title, "call mom");
    assert_eq!(p.scheduled.as_deref(), Some("2026-10-31T17:00"));
    assert_eq!(p.estimate_min, Some(30));
    assert_eq!(p.priority, Some(2));
    assert_eq!(p.tags, ["family"]);
    assert_eq!(
        p.recurrence,
        Some(Recurrence {
            rule: "FREQ=WEEKLY;BYDAY=SU".into(),
            from: Anchor::Scheduled
        })
    );
    assert_eq!((&p.deadline, &p.project, &p.area), (&None, &None, &None));
    let want = [
        (Kind::Date, "tmrw"),
        (Kind::Time, "5pm"),
        (Kind::Duration, "30m"),
        (Kind::Priority, "p2"),
        (Kind::Tag, "#family"),
        (Kind::Recurrence, "every sunday"),
    ];
    assert_eq!(kinds(&p), want);
    for t in &p.tokens {
        assert_eq!(&input[t.start..t.end], t.text);
    }
}

#[test]
fn plain_titles_stay_plain() {
    for input in ["read 1984", "buy 2 apples", "email may"] {
        let p = parse(input);
        assert_eq!(
            p,
            Parsed {
                title: input.into(),
                ..Parsed::default()
            },
            "{input}"
        );
    }
}

#[test]
fn sunday_in_a_title_is_read_as_a_date() {
    let p = parse("watch sunday times");
    assert_eq!(p.title, "watch times");
    assert_eq!(kinds(&p), [(Kind::Date, "sunday")]);
    assert_eq!(p.scheduled.as_deref(), Some("2026-11-01"));
}

#[test]
fn time_alone_means_today() {
    assert_eq!(
        parse("standup at 17").scheduled.as_deref(),
        Some("2026-10-30T17:00")
    );
    assert_eq!(
        parse("standup 9:15am").scheduled.as_deref(),
        Some("2026-10-30T09:15")
    );
}

#[test]
fn date_alone_has_no_time() {
    let p = parse("pay rent nov 12");
    assert_eq!(
        (p.title.as_str(), p.scheduled.as_deref()),
        ("pay rent", Some("2026-11-12"))
    );
    assert_eq!(kinds(&p), [(Kind::Date, "nov 12")]);
}

#[test]
fn recurrence_without_a_date_schedules_its_first_occurrence() {
    let p = parse("water plants every sunday");
    assert_eq!(p.scheduled.as_deref(), Some("2026-11-01"));
    let p = parse("gym every mon and thu 7am");
    assert_eq!(p.recurrence.unwrap().rule, "FREQ=WEEKLY;BYDAY=MO,TH");
    assert_eq!(p.scheduled.as_deref(), Some("2026-11-02T07:00"));
    let p = parse("stretch daily");
    assert_eq!(p.scheduled.as_deref(), Some("2026-10-30"));
}

#[test]
fn yesterday_is_the_day_before_today() {
    let p = parse("call bob yesterday");
    assert_eq!(
        (p.title.as_str(), p.scheduled.as_deref()),
        ("call bob", Some("2026-10-29"))
    );
    assert_eq!(kinds(&p), [(Kind::Date, "yesterday")]);
}

#[test]
fn every_n_weeks_on_weekdays() {
    let p = parse("standup every 2 weeks on tuesday 9am");
    assert_eq!(p.title, "standup");
    assert_eq!(
        p.recurrence.unwrap().rule,
        "FREQ=WEEKLY;INTERVAL=2;BYDAY=TU"
    );
    assert_eq!(p.scheduled.as_deref(), Some("2026-11-03T09:00"));
    let p = parse("gym every week on mon, thu");
    assert_eq!(p.recurrence.unwrap().rule, "FREQ=WEEKLY;BYDAY=MO,TH");
    assert_eq!(
        (p.title.as_str(), p.scheduled.as_deref()),
        ("gym", Some("2026-11-02"))
    );
    let p = parse("payroll every other week on fri");
    assert_eq!(
        p.recurrence.unwrap().rule,
        "FREQ=WEEKLY;INTERVAL=2;BYDAY=FR"
    );
    assert_eq!(p.scheduled.as_deref(), Some("2026-10-30"), "today counts");
}

#[test]
fn bare_every_other_week_is_every_two_weeks() {
    let p = parse("water plants every other week");
    let two = parse("water plants every 2 weeks");
    assert_eq!(p.title, "water plants");
    assert_eq!(
        p.recurrence.as_ref().map(|r| r.rule.as_str()),
        Some("FREQ=WEEKLY;INTERVAL=2")
    );
    assert_eq!(
        (&p.recurrence, &p.scheduled),
        (&two.recurrence, &two.scheduled)
    );
    assert_eq!(p.scheduled.as_deref(), Some("2026-10-30"));
    assert_eq!(kinds(&p), [(Kind::Recurrence, "every other week")]);
}

#[test]
fn every_other_without_week_stays_title_text() {
    for input in ["call every other", "visit every other cousin"] {
        let p = parse(input);
        assert_eq!(p.title, input);
        assert_eq!((&p.recurrence, &p.scheduled), (&None, &None), "{input}");
        assert!(p.tokens.is_empty(), "{input}");
    }
}

#[test]
fn zero_weeks_on_a_weekday_stays_title_text() {
    let p = parse("sync every 0 weeks on tue p2");
    assert_eq!(p.title, "sync every 0 weeks on tue");
    assert_eq!(kinds(&p), [(Kind::Priority, "p2")]);
    assert_eq!((p.recurrence, p.scheduled), (None, None));
}

#[test]
fn bang_recurrence_counts_from_completion() {
    let p = parse("haircut every! 6 weeks");
    assert_eq!(p.title, "haircut");
    assert_eq!(
        p.recurrence,
        Some(Recurrence {
            rule: "FREQ=WEEKLY;INTERVAL=6".into(),
            from: Anchor::Completion
        })
    );
}

#[test]
fn deadlines() {
    let p = parse("file taxes due nov 15");
    assert_eq!(
        (p.title.as_str(), p.deadline.as_deref()),
        ("file taxes", Some("2026-11-15"))
    );
    assert_eq!(p.scheduled, None, "a deadline is not a scheduled date");
    assert_eq!(kinds(&p), [(Kind::Deadline, "due nov 15")]);
    let p = parse("report by fri 5pm");
    assert_eq!(p.deadline.as_deref(), Some("2026-10-30T17:00"));
    assert_eq!(p.scheduled, None);
    let p = parse("go by bus");
    assert_eq!((p.title.as_str(), p.deadline), ("go by bus", None));
}

#[test]
fn projects_and_areas() {
    let p = parse("paint walls +home-reno @health");
    assert_eq!(p.title, "paint walls");
    assert_eq!(p.project.as_deref(), Some("Home Reno"));
    assert_eq!(p.area.as_deref(), Some("Health"));
    let p = parse("draft +garden @side_work");
    assert_eq!(
        kinds(&p),
        [(Kind::Project, "+garden"), (Kind::Area, "@side_work")]
    );
    assert_eq!(
        p.project, None,
        "unknown project is a token without a value"
    );
    assert_eq!(p.area.as_deref(), Some("Side Work"));
    assert_eq!(
        parse("ship +garden +POLYKOV").project.as_deref(),
        Some("Polykov")
    );
}

#[test]
fn tags_dedup_in_input_order() {
    let p = parse("plan #work/clientA #home #Work/clienta");
    assert_eq!(p.tags, ["work/clienta", "home"]);
    assert_eq!(p.title, "plan");
}

#[test]
fn first_value_wins_but_every_match_is_a_token() {
    let p = parse("x p1 p3 2h 30m today tomorrow");
    assert_eq!((p.priority, p.estimate_min), (Some(1), Some(120)));
    assert_eq!(p.scheduled.as_deref(), Some("2026-10-30"));
    assert_eq!((p.title.as_str(), p.tokens.len()), ("x", 6));
}

#[test]
fn quoted_text_is_never_parsed() {
    let p = parse(r#"read "next week tomorrow #tag" tmrw"#);
    assert_eq!(p.title, "read next week tomorrow #tag");
    assert_eq!(kinds(&p), [(Kind::Date, "tmrw")]);
    let p = parse(r#"watch "every day" p1"#);
    assert_eq!(
        (p.title.as_str(), p.recurrence, p.priority),
        ("watch every day", None, Some(1))
    );
}

#[test]
fn a_phrase_does_not_reach_across_quotes() {
    let p = parse(r#"meet in "3 days" ok"#);
    assert_eq!((p.title.as_str(), p.scheduled), ("meet in 3 days ok", None));
}

#[test]
fn offsets_are_bytes_and_whitespace_collapses() {
    let input = "  café   tmrw\t ünïcode  ";
    let p = parse(input);
    assert_eq!(p.title, "café ünïcode");
    let t = &p.tokens[0];
    assert_eq!((t.start, t.end), (10, 14));
    assert_eq!(&input[t.start..t.end], "tmrw");
}

#[test]
fn empty_input() {
    assert_eq!(parse(""), Parsed::default());
    assert_eq!(parse("   "), Parsed::default());
}
