//! Recurrence rules (spec §3 "Tasks (GTD)"): a pure RRULE subset and the date after a given
//! one. No database, no HTTP; `tasks` stores and applies the rules.

use std::fmt;

use chrono::{Datelike, Days, Months, NaiveDate, Weekday};

/// Largest `INTERVAL`.
pub const MAX_INTERVAL: u32 = 365;

/// How often a rule repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Day,
    Week,
    Month,
    Year,
}

/// A parsed rule. `by_day` (sorted Monday first, no repeats) is only set for [`Freq::Week`]
/// and `month_day` only for [`Freq::Month`]; [`Rule::parse`] enforces both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub freq: Freq,
    pub interval: u32,
    pub by_day: Vec<Weekday>,
    pub month_day: Option<u32>,
}

const DAYS: [(&str, Weekday); 7] = [
    ("MO", Weekday::Mon),
    ("TU", Weekday::Tue),
    ("WE", Weekday::Wed),
    ("TH", Weekday::Thu),
    ("FR", Weekday::Fri),
    ("SA", Weekday::Sat),
    ("SU", Weekday::Sun),
];

impl Rule {
    /// Reads `FREQ=DAILY|WEEKLY|MONTHLY|YEARLY[;INTERVAL=n][;BYDAY=MO,TU,..][;BYMONTHDAY=n]`
    /// (any part order, any letter case). The error is a short reason.
    pub fn parse(text: &str) -> Result<Rule, String> {
        let mut freq = None;
        let mut interval = None;
        let mut by_day = None;
        let mut month_day = None;
        for part in text.trim().to_ascii_uppercase().split(';') {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("expected KEY=VALUE, got {part:?}"))?;
            let duplicate = match key {
                "FREQ" => freq.replace(parse_freq(value)?).is_some(),
                "INTERVAL" => interval
                    .replace(parse_number(key, value, 1, MAX_INTERVAL)?)
                    .is_some(),
                "BYDAY" => by_day.replace(parse_days(value)?).is_some(),
                "BYMONTHDAY" => month_day
                    .replace(parse_number(key, value, 1, 31)?)
                    .is_some(),
                _ => return Err(format!("unknown part {key}")),
            };
            if duplicate {
                return Err(format!("{key} given twice"));
            }
        }
        let freq = freq.ok_or("FREQ is required")?;
        if by_day.is_some() && freq != Freq::Week {
            return Err("BYDAY needs FREQ=WEEKLY".into());
        }
        if month_day.is_some() && freq != Freq::Month {
            return Err("BYMONTHDAY needs FREQ=MONTHLY".into());
        }
        Ok(Rule {
            freq,
            interval: interval.unwrap_or(1),
            by_day: by_day.unwrap_or_default(),
            month_day,
        })
    }
}

fn parse_freq(value: &str) -> Result<Freq, String> {
    match value {
        "DAILY" => Ok(Freq::Day),
        "WEEKLY" => Ok(Freq::Week),
        "MONTHLY" => Ok(Freq::Month),
        "YEARLY" => Ok(Freq::Year),
        _ => Err("FREQ must be DAILY, WEEKLY, MONTHLY or YEARLY".into()),
    }
}

fn parse_number(key: &str, value: &str, min: u32, max: u32) -> Result<u32, String> {
    value
        .parse()
        .ok()
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| format!("{key} must be {min} to {max}"))
}

fn parse_days(value: &str) -> Result<Vec<Weekday>, String> {
    let mut days = Vec::new();
    for code in value.split(',') {
        let (_, day) = DAYS
            .iter()
            .find(|(c, _)| *c == code)
            .ok_or_else(|| format!("BYDAY has an unknown day {code:?}"))?;
        if !days.contains(day) {
            days.push(*day);
        }
    }
    days.sort_by_key(Weekday::num_days_from_monday);
    Ok(days)
}

impl fmt::Display for Freq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Freq::Day => "DAILY",
            Freq::Week => "WEEKLY",
            Freq::Month => "MONTHLY",
            Freq::Year => "YEARLY",
        })
    }
}

/// The canonical form: `FREQ`, then `INTERVAL` (only if not 1), `BYDAY`, `BYMONTHDAY`.
impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FREQ={}", self.freq)?;
        if self.interval != 1 {
            write!(f, ";INTERVAL={}", self.interval)?;
        }
        if !self.by_day.is_empty() {
            let codes: Vec<&str> = self
                .by_day
                .iter()
                .filter_map(|d| DAYS.iter().find(|(_, day)| day == d).map(|(c, _)| *c))
                .collect();
            write!(f, ";BYDAY={}", codes.join(","))?;
        }
        if let Some(day) = self.month_day {
            write!(f, ";BYMONTHDAY={day}")?;
        }
        Ok(())
    }
}

/// The first date of `rule`'s series strictly after `after`, with `after` as the series
/// start. Weeks start on Monday. A month day past the month's end (also Feb 29 in a common
/// year) clamps to the month's last day. Saturates at `NaiveDate::MAX`.
pub fn next(rule: &Rule, after: NaiveDate) -> NaiveDate {
    let interval = rule.interval.max(1);
    let next = match rule.freq {
        Freq::Day => after.checked_add_days(Days::new(interval.into())),
        Freq::Week if rule.by_day.is_empty() => {
            after.checked_add_days(Days::new(u64::from(interval) * 7))
        }
        Freq::Week => next_weekday(rule, interval, after),
        Freq::Month => {
            let day = rule.month_day.unwrap_or(after.day());
            Some(on_day(after, day))
                .filter(|&same_month| same_month > after)
                .or_else(|| month_start(after, interval).map(|m| on_day(m, day)))
        }
        Freq::Year => month_start(after, interval * 12).map(|m| on_day(m, after.day())),
    };
    next.unwrap_or(NaiveDate::MAX)
}

/// The next listed weekday in a week that is a multiple of `interval` weeks from `after`'s.
fn next_weekday(rule: &Rule, interval: u32, after: NaiveDate) -> Option<NaiveDate> {
    let monday = |d: NaiveDate| d.week(Weekday::Mon).first_day();
    let start = monday(after);
    (1..=u64::from(interval) * 7 + 7)
        .filter_map(|n| after.checked_add_days(Days::new(n)))
        .find(|&d| {
            let weeks = (monday(d) - start).num_weeks();
            weeks % i64::from(interval) == 0 && rule.by_day.contains(&d.weekday())
        })
}

/// The first day of the month `months` after `date`'s month.
fn month_start(date: NaiveDate, months: u32) -> Option<NaiveDate> {
    date.with_day(1)?.checked_add_months(Months::new(months))
}

/// Day `day` of `date`'s month, clamped to the month's last day.
fn on_day(date: NaiveDate, day: u32) -> NaiveDate {
    let first = date.with_day(1).unwrap_or(date);
    let last = first
        .checked_add_months(Months::new(1))
        .and_then(|d| d.pred_opt())
        .map_or(28, |d| d.day());
    first.with_day(day.min(last)).unwrap_or(first)
}

#[cfg(test)]
mod tests;
