//! Recurrence phrases, turned into the 027 RRULE subset. `mod.rs` matches the phrase,
//! `weekday.rs` reads weekday lists, `rule.rs` writes the rule and its first occurrence.

mod rule;
mod weekday;

use chrono::NaiveDate;

use self::rule::{Freq, build};
use self::weekday::{WORKDAYS, on_days, weekday_list};
use super::when::day;
use crate::words::Run;
use crate::{Anchor, Recurrence};

/// A matched recurrence and its first occurrence on or after today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Repeat {
    pub recurrence: Recurrence,
    pub first: NaiveDate,
}

/// A recurrence phrase at the start of `run`: `daily`…`yearly`, or `every`/`every!` followed by
/// `day|weekday|week|month|year`, `N days|weeks|months|years`, `other week`, `15th`, or a
/// weekday list. `week`, `N weeks` and `other week` take an optional `on <weekday list>`.
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
        return Some(weekly_on(run, 3, Freq::Weekly, 2, from, today));
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

#[cfg(test)]
mod tests;
