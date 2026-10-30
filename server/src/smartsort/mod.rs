//! Smart sort (spec "Tasks (GTD)"): a pure, deterministic score from priority, deadline
//! proximity, estimate and goal relevance, and the order it gives. ADR 0017 records the
//! formula; the weights below are its only source.

use std::cmp::Ordering;

use chrono::NaiveDate;

use crate::tasks::store::{Fields, Task};

/// Points per priority level above P4: P1 = 30, P2 = 20, P3 = 10, P4 = 0.
pub const PRIORITY_WEIGHT: f64 = 10.0;
/// Points for a deadline before today, whatever the gap.
pub const OVERDUE: f64 = 40.0;
/// Points for a deadline today; they fall linearly to 0 at `DEADLINE_HORIZON_DAYS`.
pub const DEADLINE_WEIGHT: f64 = 30.0;
/// Days ahead at which a deadline stops adding points.
pub const DEADLINE_HORIZON_DAYS: f64 = 30.0;
/// Points for the shortest estimate; they fall linearly to 0 at `ESTIMATE_CAP_MINUTES`.
pub const QUICK_WIN_WEIGHT: f64 = 5.0;
/// Estimate, in minutes, from which an estimate adds nothing.
pub const ESTIMATE_CAP_MINUTES: f64 = 120.0;
/// Weight of goal relevance; the relevance itself is 0 until goals land in M9.
pub const GOAL_WEIGHT: f64 = 20.0;

/// The smart score of a task as of `today`; higher ranks first.
pub fn score(task: &Fields, today: NaiveDate) -> f64 {
    priority(task.priority)
        + deadline(task.deadline.as_deref(), today)
        + quick_win(task.estimate)
        + GOAL_WEIGHT * goal_relevance(task)
}

/// Sorts `tasks` by descending [`score`], ties by ascending id.
pub fn sort(tasks: &mut [Task], today: NaiveDate) {
    tasks.sort_by_cached_key(|t| (Rank(score(&t.task, today)), t.object.id));
}

/// A score ordered high to low.
#[derive(PartialEq)]
struct Rank(f64);

impl Eq for Rank {}

impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rank {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0)
    }
}

fn priority(p: i64) -> f64 {
    (4 - p.clamp(1, 4)) as f64 * PRIORITY_WEIGHT
}

/// `deadline` is `YYYY-MM-DD` or a date-time; it counts by its date.
fn deadline(deadline: Option<&str>, today: NaiveDate) -> f64 {
    let Some(date) = deadline
        .and_then(|d| d.get(..10))
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
    else {
        return 0.0;
    };
    let days = (date - today).num_days() as f64;
    if days < 0.0 {
        OVERDUE
    } else {
        DEADLINE_WEIGHT * (1.0 - days / DEADLINE_HORIZON_DAYS).max(0.0)
    }
}

fn quick_win(estimate: Option<i64>) -> f64 {
    estimate.map_or(0.0, |m| {
        QUICK_WIN_WEIGHT * (1.0 - m as f64 / ESTIMATE_CAP_MINUTES).max(0.0)
    })
}

/// How much the task serves the owner's goals, 0 to 1. Always 0: goal links land in M9.
fn goal_relevance(_: &Fields) -> f64 {
    0.0
}

#[cfg(test)]
mod tests;
