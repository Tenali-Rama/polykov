//! Date and time phrases. Each matcher returns how many words it used and the value.

use chrono::{Datelike, Days, Months, NaiveDate, NaiveTime, Weekday};

use crate::words::Run;

/// A date phrase at the start of `run`, resolved against `today`.
pub(super) fn date(run: &Run, today: NaiveDate) -> Option<(usize, NaiveDate)> {
    let w0 = run.low(0)?;
    let w1 = run.low(1);
    match (w0.as_str(), w1.as_deref()) {
        ("next", Some("week")) => {
            return Some((2, next_on_or_after(today + Days::new(1), Weekday::Mon)));
        }
        ("next", Some("month")) => {
            let first = today.with_day(1)?.checked_add_months(Months::new(1))?;
            return Some((2, first));
        }
        ("in", Some(n)) => {
            if let Some(d) = run.low(2).and_then(|unit| in_n(today, n, &unit)) {
                return Some((3, d));
            }
        }
        _ => {}
    }
    if let Some(d) = w1.as_deref().and_then(|w1| month_day_pair(today, &w0, w1)) {
        return Some((2, d));
    }
    let d = match w0.as_str() {
        "today" | "tod" => today,
        "tomorrow" | "tmrw" | "tmr" => today + Days::new(1),
        w => match weekday(w) {
            Some(day) => next_on_or_after(today, day),
            None if w.len() == 10 => NaiveDate::parse_from_str(w, "%Y-%m-%d").ok()?,
            None => return None,
        },
    };
    Some((1, d))
}

/// A time phrase at the start of `run`: `5pm`, `5:30pm`, `17:00`, or `at` plus one of those
/// or a bare 24-hour hour (`at 17`).
pub(super) fn time(run: &Run) -> Option<(usize, NaiveTime)> {
    let w0 = run.low(0)?;
    if w0 == "at" {
        let w1 = run.low(1)?;
        let t = clock(&w1).or_else(|| hour(&w1).and_then(|h| NaiveTime::from_hms_opt(h, 0, 0)))?;
        return Some((2, t));
    }
    clock(&w0).map(|t| (1, t))
}

/// `in N days|weeks|months` (singular units too).
fn in_n(today: NaiveDate, n: &str, unit: &str) -> Option<NaiveDate> {
    let n: u32 = n.parse().ok()?;
    match unit {
        "day" | "days" => today.checked_add_days(Days::new(n.into())),
        "week" | "weeks" => today.checked_add_days(Days::new(u64::from(n) * 7)),
        "month" | "months" => today.checked_add_months(Months::new(n)),
        _ => None,
    }
}

/// `nov 12`, `november 12th` or `12 nov`: this year, or next year if that date is past.
fn month_day_pair(today: NaiveDate, a: &str, b: &str) -> Option<NaiveDate> {
    let (m, d) = match (month(a), month(b)) {
        (Some(m), None) => (m, day(b)?),
        (None, Some(m)) => (m, day(a)?),
        _ => return None,
    };
    match NaiveDate::from_ymd_opt(today.year(), m, d) {
        Some(date) if date >= today => Some(date),
        _ => NaiveDate::from_ymd_opt(today.year() + 1, m, d),
    }
}

/// The first `day` on or after `from`.
pub(super) fn next_on_or_after(from: NaiveDate, day: Weekday) -> NaiveDate {
    let ahead = (7 + day.num_days_from_monday() - from.weekday().num_days_from_monday()) % 7;
    from + Days::new(ahead.into())
}

/// A weekday's full English name or its 3-letter abbreviation.
pub(super) fn weekday(w: &str) -> Option<Weekday> {
    let day = match w {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    };
    Some(day)
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// A month's full English name or its 3-letter abbreviation, as 1–12.
fn month(w: &str) -> Option<u32> {
    let i = MONTHS
        .iter()
        .position(|m| *m == w || (w.len() == 3 && m.starts_with(w)))?;
    u32::try_from(i + 1).ok()
}

/// A day of the month, 1–31, optionally with an ordinal suffix (`12`, `12th`, `1st`).
pub(super) fn day(w: &str) -> Option<u32> {
    let digits = ["st", "nd", "rd", "th"]
        .iter()
        .find_map(|s| w.strip_suffix(s))
        .unwrap_or(w);
    let d: u32 = number(digits)?;
    (1..=31).contains(&d).then_some(d)
}

/// `5pm`, `12am`, `5:30pm` (12-hour) or `17:00` (24-hour).
fn clock(w: &str) -> Option<NaiveTime> {
    let (body, pm) = match (w.strip_suffix("am"), w.strip_suffix("pm")) {
        (Some(b), _) => (b, Some(false)),
        (_, Some(b)) => (b, Some(true)),
        _ => (w, None),
    };
    let (h, m) = match body.split_once(':') {
        Some((h, m)) if m.len() == 2 => (number(h)?, number(m)?),
        Some(_) => return None,
        None if pm.is_some() => (number(body)?, 0),
        None => return None,
    };
    let h = match pm {
        Some(pm) if (1..=12).contains(&h) => h % 12 + if pm { 12 } else { 0 },
        Some(_) => return None,
        None => h,
    };
    NaiveTime::from_hms_opt(h, m, 0)
}

/// A bare 24-hour hour, 0–23.
fn hour(w: &str) -> Option<u32> {
    number(w).filter(|h| *h < 24)
}

/// Digits only (no sign), at most 4 of them.
fn number(w: &str) -> Option<u32> {
    let ok = !w.is_empty() && w.len() <= 4 && w.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| w.parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::split;

    /// 2026-10-30 is a Friday.
    const TODAY: (i32, u32, u32) = (2026, 10, 30);

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn today() -> NaiveDate {
        ymd(TODAY.0, TODAY.1, TODAY.2)
    }

    fn on(input: &str) -> Option<(usize, NaiveDate)> {
        let (words, _) = split(input);
        date(
            &Run {
                input,
                words: &words,
            },
            today(),
        )
    }

    fn at(input: &str) -> Option<(usize, String)> {
        let (words, _) = split(input);
        time(&Run {
            input,
            words: &words,
        })
        .map(|(n, t)| (n, t.format("%H:%M").to_string()))
    }

    #[test]
    fn relative_days() {
        for w in ["today", "tod", "Today"] {
            assert_eq!(on(w), Some((1, today())), "{w}");
        }
        for w in ["tomorrow", "tmrw", "tmr"] {
            assert_eq!(on(w), Some((1, ymd(2026, 10, 31))), "{w}");
        }
    }

    #[test]
    fn weekdays_are_the_next_such_day_today_included() {
        assert_eq!(on("fri"), Some((1, today())));
        assert_eq!(on("friday"), Some((1, today())));
        assert_eq!(on("sat"), Some((1, ymd(2026, 10, 31))));
        assert_eq!(on("sunday"), Some((1, ymd(2026, 11, 1))));
        assert_eq!(on("thu"), Some((1, ymd(2026, 11, 5))));
        assert_eq!(on("tues"), None);
    }

    #[test]
    fn next_week_and_next_month() {
        assert_eq!(on("next week"), Some((2, ymd(2026, 11, 2))));
        assert_eq!(on("next month"), Some((2, ymd(2026, 11, 1))));
        let monday = ymd(2026, 11, 2);
        let (words, _) = split("next week");
        let run = Run {
            input: "next week",
            words: &words,
        };
        assert_eq!(date(&run, monday), Some((2, ymd(2026, 11, 9))));
        let dec = ymd(2026, 12, 31);
        let (words, _) = split("next month");
        let run = Run {
            input: "next month",
            words: &words,
        };
        assert_eq!(date(&run, dec), Some((2, ymd(2027, 1, 1))));
    }

    #[test]
    fn in_n_units() {
        assert_eq!(on("in 3 days"), Some((3, ymd(2026, 11, 2))));
        assert_eq!(on("in 1 day"), Some((3, ymd(2026, 10, 31))));
        assert_eq!(on("in 2 weeks"), Some((3, ymd(2026, 11, 13))));
        assert_eq!(on("in 4 months"), Some((3, ymd(2027, 2, 28))));
        assert_eq!(on("in two days"), None);
        assert_eq!(on("in 3 apples"), None);
    }

    #[test]
    fn month_and_day_in_either_order() {
        assert_eq!(on("nov 12"), Some((2, ymd(2026, 11, 12))));
        assert_eq!(on("12 nov"), Some((2, ymd(2026, 11, 12))));
        assert_eq!(on("november 12th"), Some((2, ymd(2026, 11, 12))));
        assert_eq!(on("oct 30"), Some((2, today())));
        assert_eq!(
            on("oct 29"),
            Some((2, ymd(2027, 10, 29))),
            "past rolls to next year"
        );
        assert_eq!(on("feb 30"), None);
        assert_eq!(on("may 2026"), None);
    }

    #[test]
    fn iso_dates() {
        assert_eq!(on("2026-11-12"), Some((1, ymd(2026, 11, 12))));
        assert_eq!(on("2026-02-30"), None);
        assert_eq!(on("1984"), None);
    }

    #[test]
    fn times() {
        assert_eq!(at("5pm"), Some((1, "17:00".into())));
        assert_eq!(at("5:30pm"), Some((1, "17:30".into())));
        assert_eq!(at("12am"), Some((1, "00:00".into())));
        assert_eq!(at("12pm"), Some((1, "12:00".into())));
        assert_eq!(at("17:00"), Some((1, "17:00".into())));
        assert_eq!(at("at 17"), Some((2, "17:00".into())));
        assert_eq!(at("at 9am"), Some((2, "09:00".into())));
    }

    #[test]
    fn not_times() {
        for w in [
            "13pm", "0am", "17", "25:00", "5:3pm", "at 24", "at noon", "pm",
        ] {
            assert_eq!(at(w), None, "{w}");
        }
    }
}
