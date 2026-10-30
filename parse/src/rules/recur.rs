//! Recurrence phrases, turned into the 027 RRULE subset.

use chrono::{Datelike, Months, NaiveDate, Weekday};

use super::when::{day, next_on_or_after, weekday};
use crate::words::Run;
use crate::{Anchor, Recurrence};

/// A matched recurrence and its first occurrence on or after today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Repeat {
    pub recurrence: Recurrence,
    pub first: NaiveDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

const WORKDAYS: [Weekday; 5] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
];

/// A recurrence phrase at the start of `run`: `daily`…`yearly`, or `every`/`every!` followed by
/// `day|weekday|week|month|year`, `N days|weeks|months|years`, `15th`, or a weekday list.
/// `week`, `N weeks` and `other week` take `on <weekday list>`; `other week` needs it.
pub(super) fn repeat(run: &Run, today: NaiveDate) -> Option<(usize, Repeat)> {
    let w0 = run.low(0)?;
    let simple = match w0.as_str() {
        "daily" => Some(Freq::Daily),
        "weekly" => Some(Freq::Weekly),
        "monthly" => Some(Freq::Monthly),
        "yearly" => Some(Freq::Yearly),
        _ => None,
    };
    if let Some(freq) = simple {
        return Some((1, build(freq, 1, &[], None, Anchor::Scheduled, today)));
    }
    let from = match w0.as_str() {
        "every" => Anchor::Scheduled,
        "every!" => Anchor::Completion,
        _ => return None,
    };
    let w1 = run.low(1)?;
    let unit = |w: &str| match w {
        "day" => Some(Freq::Daily),
        "week" => Some(Freq::Weekly),
        "month" => Some(Freq::Monthly),
        "year" => Some(Freq::Yearly),
        _ => None,
    };
    if let Some(freq) = unit(&w1) {
        return Some(weekly_on(run, 2, freq, 1, from, today));
    }
    if w1 == "weekday" {
        return Some((2, build(Freq::Weekly, 1, &WORKDAYS, None, from, today)));
    }
    if let Some(n) = w1.parse::<u32>().ok().filter(|n| (1..=365).contains(n)) {
        let w2 = run.low(2)?;
        let freq = unit(w2.strip_suffix('s').unwrap_or(&w2))?;
        return Some(weekly_on(run, 3, freq, n, from, today));
    }
    if w1 == "other" && run.low(2).as_deref() == Some("week") {
        let (n, days) = on_days(&run.skip(3))?;
        return Some((3 + n, build(Freq::Weekly, 2, &days, None, from, today)));
    }
    if let Some(d) = day(&w1) {
        return Some((2, build(Freq::Monthly, 1, &[], Some(d), from, today)));
    }
    let (n, days) = weekday_list(&run.skip(1))?;
    Some((1 + n, build(Freq::Weekly, 1, &days, None, from, today)))
}

/// `every <x> week[s] on <weekday list>` whose count `<x>` [`repeat`] rejects (`every 0 weeks
/// on tue`): the words it spans, which stay title text rather than leaving a stray date.
pub(super) fn malformed(run: &Run) -> Option<usize> {
    let every = matches!(run.low(0)?.as_str(), "every" | "every!");
    if !every || !matches!(run.low(2)?.as_str(), "week" | "weeks") {
        return None;
    }
    let (n, _) = on_days(&run.skip(3))?;
    Some(3 + n)
}

/// A `used`-word phrase, plus `on <weekday list>` after it when `freq` is weekly.
fn weekly_on(
    run: &Run,
    used: usize,
    freq: Freq,
    interval: u32,
    from: Anchor,
    today: NaiveDate,
) -> (usize, Repeat) {
    let on = (freq == Freq::Weekly)
        .then(|| on_days(&run.skip(used)))
        .flatten();
    let (n, days) = on.unwrap_or_default();
    (used + n, build(freq, interval, &days, None, from, today))
}

/// `on <weekday list>`: the words used and the days.
fn on_days(run: &Run) -> Option<(usize, Vec<Weekday>)> {
    if run.low(0)? != "on" {
        return None;
    }
    let (n, days) = weekday_list(&run.skip(1))?;
    Some((1 + n, days))
}

/// `sun`, `mon and thu`, `mon, wed, and fri`: the words used and the days.
fn weekday_list(run: &Run) -> Option<(usize, Vec<Weekday>)> {
    let mut days = Vec::new();
    let mut used = 0;
    let mut k = 0;
    while let Some(w) = run.low(k) {
        let w = w.strip_suffix(',').unwrap_or(&w);
        if let Some(d) = weekday(w) {
            days.push(d);
            k += 1;
            used = k;
        } else if !days.is_empty() && (w.is_empty() || w == "and") {
            k += 1;
        } else {
            break;
        }
    }
    (!days.is_empty()).then_some((used, days))
}

fn build(
    freq: Freq,
    interval: u32,
    days: &[Weekday],
    month_day: Option<u32>,
    from: Anchor,
    today: NaiveDate,
) -> Repeat {
    let mut days = days.to_vec();
    days.sort_by_key(Weekday::num_days_from_monday);
    days.dedup();
    let mut rule = format!("FREQ={}", freq_name(freq));
    if interval > 1 {
        rule += &format!(";INTERVAL={interval}");
    }
    if !days.is_empty() {
        let codes: Vec<&str> = days.iter().map(|d| day_code(*d)).collect();
        rule += &format!(";BYDAY={}", codes.join(","));
    }
    if let Some(d) = month_day {
        rule += &format!(";BYMONTHDAY={d}");
    }
    let first = match month_day {
        Some(d) => first_month_day(today, d),
        None => days
            .iter()
            .map(|d| next_on_or_after(today, *d))
            .min()
            .unwrap_or(today),
    };
    Repeat {
        recurrence: Recurrence { rule, from },
        first,
    }
}

/// The first date on or after `today` with day `d`, clamped to the month's end like 027.
fn first_month_day(today: NaiveDate, d: u32) -> NaiveDate {
    let in_month = |first: NaiveDate| {
        (1..=d)
            .rev()
            .find_map(|x| first.with_day(x))
            .unwrap_or(first)
    };
    let this = in_month(today.with_day(1).unwrap_or(today));
    if this >= today {
        return this;
    }
    let next = today
        .with_day(1)
        .and_then(|f| f.checked_add_months(Months::new(1)))
        .unwrap_or(today);
    in_month(next)
}

fn freq_name(freq: Freq) -> &'static str {
    match freq {
        Freq::Daily => "DAILY",
        Freq::Weekly => "WEEKLY",
        Freq::Monthly => "MONTHLY",
        Freq::Yearly => "YEARLY",
    }
}

fn day_code(d: Weekday) -> &'static str {
    match d {
        Weekday::Mon => "MO",
        Weekday::Tue => "TU",
        Weekday::Wed => "WE",
        Weekday::Thu => "TH",
        Weekday::Fri => "FR",
        Weekday::Sat => "SA",
        Weekday::Sun => "SU",
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(rep("every other week"), None);
        assert_eq!(rep("every other week on"), None);
        assert_eq!(rep("every 0 weeks on tue"), None);
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
}
