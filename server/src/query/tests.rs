use super::*;

fn d(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
}

fn term(filter: Filter) -> Term {
    Term {
        negated: false,
        filter,
    }
}

fn not(filter: Filter) -> Term {
    Term {
        negated: true,
        filter,
    }
}

/// The single term of `input`.
fn one(input: &str) -> Term {
    let query = parse(input).unwrap();
    assert_eq!(query.terms.len(), 1, "{input}");
    query.terms.into_iter().next().unwrap()
}

/// The error of `input` as (offset, message).
fn fail(input: &str) -> (usize, String) {
    let e = parse(input).unwrap_err();
    (e.offset, e.message)
}

fn cmp<T>(op: Op, value: T) -> Cmp<T> {
    Cmp { op, value }
}

fn rel(n: i32, unit: Unit) -> DateValue {
    DateValue::Relative(n, unit)
}

#[test]
fn spec_example() {
    let query = parse("type:task priority:<=2 due:<7d tag:work/* -status:done").unwrap();
    assert_eq!(
        query,
        Query {
            terms: vec![
                term(Filter::Type(vec!["task".into()])),
                term(Filter::Priority(vec![cmp(Op::Le, 2)])),
                term(Filter::Date(
                    DateField::Due,
                    vec![cmp(Op::Lt, rel(7, Unit::Day))]
                )),
                term(Filter::Tag(vec![TagPattern {
                    tag: "work".into(),
                    subtree: true
                }])),
                not(Filter::Status(vec![Status::Done])),
            ],
            archived: Archived::No,
        }
    );
}

#[test]
fn empty_query_has_no_terms_and_hides_archived() {
    for input in ["", "   "] {
        let query = parse(input).unwrap();
        assert!(query.terms.is_empty());
        assert_eq!(query.archived, Archived::No);
    }
}

#[test]
fn type_tag_status_fields() {
    assert_eq!(
        one("type:journal_entry").filter,
        Filter::Type(vec!["journal_entry".into()])
    );
    assert_eq!(
        one("tag:#Work/Deep").filter,
        Filter::Tag(vec![TagPattern {
            tag: "work/deep".into(),
            subtree: false
        }])
    );
    assert_eq!(
        one("status:open").filter,
        Filter::Status(vec![Status::Open])
    );
}

#[test]
fn priority_with_every_operator() {
    for (input, op) in [
        ("priority:1", Op::Eq),
        ("priority:=1", Op::Eq),
        ("priority:<1", Op::Lt),
        ("priority:<=1", Op::Le),
        ("priority:>1", Op::Gt),
        ("priority:>=1", Op::Ge),
    ] {
        assert_eq!(
            one(input).filter,
            Filter::Priority(vec![cmp(op, 1)]),
            "{input}"
        );
    }
}

#[test]
fn every_date_field_with_every_operator() {
    for (name, field) in [
        ("due", DateField::Due),
        ("scheduled", DateField::Scheduled),
        ("created", DateField::Created),
        ("updated", DateField::Updated),
        ("completed", DateField::Completed),
    ] {
        for (op_text, op) in [
            ("", Op::Eq),
            ("=", Op::Eq),
            ("<", Op::Lt),
            ("<=", Op::Le),
            (">", Op::Gt),
            (">=", Op::Ge),
        ] {
            let input = format!("{name}:{op_text}2026-10-31");
            assert_eq!(
                one(&input).filter,
                Filter::Date(field, vec![cmp(op, DateValue::Absolute(d("2026-10-31")))]),
                "{input}"
            );
        }
    }
}

#[test]
fn every_date_value() {
    for (value, want) in [
        ("today", rel(0, Unit::Day)),
        ("Tomorrow", rel(1, Unit::Day)),
        ("yesterday", rel(-1, Unit::Day)),
        ("7d", rel(7, Unit::Day)),
        ("-3d", rel(-3, Unit::Day)),
        ("+3d", rel(3, Unit::Day)),
        ("2w", rel(2, Unit::Week)),
        ("1m", rel(1, Unit::Month)),
        ("none", DateValue::None),
        ("2027-02-28", DateValue::Absolute(d("2027-02-28"))),
    ] {
        let input = format!("scheduled:{value}");
        assert_eq!(
            one(&input).filter,
            Filter::Date(DateField::Scheduled, vec![cmp(Op::Eq, want)]),
            "{input}"
        );
    }
}

#[test]
fn relative_dates_resolve_against_a_given_today() {
    let today = d("2026-01-31");
    for (value, want) in [
        (rel(0, Unit::Day), "2026-01-31"),
        (rel(-1, Unit::Day), "2026-01-30"),
        (rel(7, Unit::Day), "2026-02-07"),
        (rel(2, Unit::Week), "2026-02-14"),
        (rel(-1, Unit::Week), "2026-01-24"),
        (rel(1, Unit::Month), "2026-02-28"),
        (rel(-2, Unit::Month), "2025-11-30"),
        (DateValue::Absolute(d("2020-05-01")), "2020-05-01"),
    ] {
        assert_eq!(value.resolve(today), Some(d(want)), "{value:?}");
    }
    assert_eq!(DateValue::None.resolve(today), None);
    assert_eq!(rel(i32::MAX, Unit::Week).resolve(today), None);
    assert_eq!(
        rel(-12345678, Unit::Day).resolve(today),
        None,
        "before year 0000"
    );
    assert_eq!(
        rel(3000000, Unit::Day).resolve(today),
        None,
        "after year 9999"
    );
    let month = |n| rel(n, Unit::Month).resolve(today);
    assert_eq!(month(-2026 * 12), Some(d("0000-01-31")));
    assert_eq!(month(-2026 * 12 - 1), None);
    assert_eq!(month(7973 * 12 + 11), Some(d("9999-12-31")));
    assert_eq!(month(7974 * 12), None);
}

#[test]
fn a_ten_char_signed_offset_is_relative_not_a_date() {
    assert_eq!(
        one("due:-12345678d").filter,
        Filter::Date(DateField::Due, vec![cmp(Op::Eq, rel(-12345678, Unit::Day))])
    );
    assert_eq!(
        one("due:2026-10-31").filter,
        Filter::Date(
            DateField::Due,
            vec![cmp(Op::Eq, DateValue::Absolute(d("2026-10-31")))]
        )
    );
    let err = compile(&parse("due:-12345678d").unwrap(), d("2026-10-31")).unwrap_err();
    assert!(
        matches!(&err, crate::http::ApiError::Client(status, message)
            if status.as_u16() == 400 && message == "date is out of range"),
        "{err:?}"
    );
}

#[test]
fn project_and_area_take_an_id_or_a_quoted_title() {
    assert_eq!(one("project:12").filter, Filter::Project(vec![Ref::Id(12)]));
    assert_eq!(
        one("area:\"Home life\"").filter,
        Filter::Area(vec![Ref::Title("Home life".into())])
    );
    assert_eq!(
        one("project:\"42\",7").filter,
        Filter::Project(vec![Ref::Title("42".into()), Ref::Id(7)])
    );
}

#[test]
fn pinned_and_archived() {
    assert_eq!(one("pinned:true").filter, Filter::Pinned(true));
    assert_eq!(one("pinned:FALSE").filter, Filter::Pinned(false));
    for (input, want) in [
        ("archived:true", Archived::Yes),
        ("archived:false", Archived::No),
        ("archived:any", Archived::Any),
        ("-archived:true", Archived::No),
        ("-archived:false", Archived::Yes),
    ] {
        let query = parse(input).unwrap();
        assert!(query.terms.is_empty(), "{input}");
        assert_eq!(query.archived, want, "{input}");
    }
}

#[test]
fn negation_quotes_and_text() {
    let query = parse("plan -draft \"weekly review\" -\"a:b, c\" -tag:x").unwrap();
    assert_eq!(
        query.terms,
        vec![
            term(Filter::Text("plan".into())),
            not(Filter::Text("draft".into())),
            term(Filter::Text("weekly review".into())),
            not(Filter::Text("a:b, c".into())),
            not(Filter::Tag(vec![TagPattern {
                tag: "x".into(),
                subtree: false
            }])),
        ]
    );
}

#[test]
fn comma_lists_are_or_within_a_field() {
    assert_eq!(
        one("status:open,done").filter,
        Filter::Status(vec![Status::Open, Status::Done])
    );
    assert_eq!(
        one("priority:1,>=3").filter,
        Filter::Priority(vec![cmp(Op::Eq, 1), cmp(Op::Ge, 3)])
    );
    assert_eq!(
        one("type:task,note").filter,
        Filter::Type(vec!["task".into(), "note".into()])
    );
    assert_eq!(
        one("due:none,<today").filter,
        Filter::Date(
            DateField::Due,
            vec![cmp(Op::Eq, DateValue::None), cmp(Op::Lt, rel(0, Unit::Day))]
        )
    );
    assert_eq!(
        one("tag:a,b/*").filter,
        Filter::Tag(vec![
            TagPattern {
                tag: "a".into(),
                subtree: false
            },
            TagPattern {
                tag: "b".into(),
                subtree: true
            },
        ])
    );
}

#[test]
fn errors_name_the_offset() {
    for (input, offset, message) in [
        ("type:task foo:bar", 10, "unknown field \"foo\""),
        ("-colour:red", 1, "unknown field \"colour\""),
        ("tag:<x", 4, "tag takes no operator"),
        ("status:>=open", 7, "status takes no operator"),
        ("project:\"a\",,1", 12, "empty value for project"),
        ("due:2026-13-01", 4, "invalid date"),
        ("due:<soon", 5, "invalid date"),
        ("due:3x", 4, "invalid date"),
        ("due:d", 4, "invalid date"),
        ("due:<none", 4, "due:none takes no operator"),
        ("priority:5", 9, "priority must be 1 to 4"),
        ("priority:<=0", 11, "priority must be 1 to 4"),
        ("a \"open quote", 2, "unclosed quote"),
        ("tag:", 4, "empty value for tag"),
        ("status:open,", 12, "empty value for status"),
        ("status:,open", 7, "empty value for status"),
        ("priority:<=", 11, "empty value for priority"),
        ("x - y", 3, "empty term"),
        ("type:tsk", 5, "unknown type tsk"),
        ("tag:a/", 4, "invalid tag a/"),
        (
            "tag:work/*/*",
            9,
            "tag wildcard must be a single trailing /*",
        ),
        ("tag:*", 4, "tag wildcard must be a single trailing /*"),
        ("tag:wo*rk", 6, "tag wildcard must be a single trailing /*"),
        ("tag:a,b*/*", 7, "tag wildcard must be a single trailing /*"),
        ("project:inbox", 8, "project takes an id"),
        ("pinned:yes", 7, "pinned must be true or false"),
        ("pinned:true,false", 12, "pinned takes one value"),
        ("archived:maybe", 9, "archived must be true, false or any"),
        ("-archived:any", 10, "archived:any cannot be negated"),
        ("archived:true archived:any", 14, "archived given twice"),
    ] {
        let (at, got) = fail(input);
        assert!(got.starts_with(message), "{input}: {got}");
        assert_eq!(at, offset, "{input}: {got}");
    }
}

#[test]
fn offsets_count_chars_not_bytes() {
    assert_eq!(fail("café foo:x").0, 5);
}
