//! Weekday lists inside recurrence phrases.

use chrono::Weekday;

use super::super::when::weekday;
use crate::words::Run;

pub(super) const WORKDAYS: [Weekday; 5] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
];

/// `on <weekday list>`: the words used and the days.
pub(super) fn on_days(run: &Run) -> Option<(usize, Vec<Weekday>)> {
    if run.low(0)? != "on" {
        return None;
    }
    let (n, days) = weekday_list(&run.skip(1))?;
    Some((1 + n, days))
}

/// `sun`, `mon and thu`, `mon, wed, and fri`: the words used and the days.
pub(super) fn weekday_list(run: &Run) -> Option<(usize, Vec<Weekday>)> {
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
