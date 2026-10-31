//! Writes a matched recurrence as an 027 RRULE and finds its first occurrence.

use chrono::{Datelike, Months, NaiveDate, Weekday};

use super::super::when::next_on_or_after;
use super::Repeat;
use crate::{Anchor, Recurrence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

pub(super) fn build(
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
