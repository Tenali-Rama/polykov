use serde_json::Map;

use super::*;
use crate::objects::store::Object;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
}

/// A P4 task `id` with no deadline and no estimate.
fn task(id: i64) -> Task {
    Task {
        object: Object {
            id,
            r#type: "task".into(),
            title: format!("t{id}"),
            created: String::new(),
            updated: String::new(),
            archived: false,
            pinned: false,
            tags: vec![],
            properties: Map::new(),
        },
        task: Fields {
            status: "open".into(),
            priority: 4,
            scheduled: None,
            deadline: None,
            estimate: None,
            someday: false,
            parent_id: None,
            project_id: None,
            section_id: None,
            area_id: None,
            checklist: vec![],
            recurrence: None,
            completed: None,
        },
    }
}

fn with(id: i64, edit: impl FnOnce(&mut Fields)) -> Task {
    let mut t = task(id);
    edit(&mut t.task);
    t
}

fn order(mut tasks: Vec<Task>) -> Vec<i64> {
    sort(&mut tasks, today());
    tasks.iter().map(|t| t.object.id).collect()
}

#[test]
fn higher_priority_ranks_first() {
    let tasks = vec![
        with(1, |f| f.priority = 3),
        with(2, |f| f.priority = 1),
        with(3, |f| f.priority = 2),
    ];
    assert_eq!(order(tasks), [2, 3, 1]);
}

#[test]
fn overdue_ranks_before_closer_deadlines_before_later_ones() {
    let tasks = vec![
        with(1, |f| f.deadline = Some("2026-11-20".into())),
        with(2, |f| f.deadline = None),
        with(3, |f| f.deadline = Some("2026-11-02T09:00".into())),
        with(4, |f| f.deadline = Some("2026-10-30".into())),
        with(5, |f| f.deadline = Some("2026-11-01".into())),
    ];
    assert_eq!(order(tasks), [4, 5, 3, 1, 2]);
}

#[test]
fn deadline_past_the_horizon_adds_nothing() {
    let far = with(1, |f| f.deadline = Some("2027-06-01".into()));
    assert_eq!(score(&far.task, today()), score(&task(2).task, today()));
}

#[test]
fn shorter_estimate_ranks_slightly_higher() {
    let tasks = vec![
        with(1, |f| f.estimate = Some(90)),
        with(2, |f| f.estimate = Some(10)),
        with(3, |f| f.estimate = Some(45)),
    ];
    assert_eq!(order(tasks), [2, 3, 1]);
    // "Slightly": the largest quick-win bonus is less than one priority level.
    let quick = with(4, |f| f.estimate = Some(1));
    let p3 = with(5, |f| f.priority = 3);
    assert_eq!(order(vec![quick, p3]), [5, 4]);
}

#[test]
fn overdue_p3_ranks_before_p1_due_in_30_days() {
    let p3 = with(1, |f| {
        f.priority = 3;
        f.deadline = Some("2026-10-31".into());
    });
    let p1 = with(2, |f| {
        f.priority = 1;
        f.deadline = Some("2026-12-01".into());
    });
    assert_eq!(score(&p3.task, today()), 50.0);
    assert_eq!(score(&p1.task, today()), 30.0);
    assert_eq!(order(vec![p1, p3]), [1, 2]);
}

#[test]
fn no_deadline_and_no_estimate_scores_priority_only() {
    for (p, points) in [(1, 30.0), (2, 20.0), (3, 10.0), (4, 0.0)] {
        assert_eq!(score(&with(1, |f| f.priority = p).task, today()), points);
    }
    // An estimate at or past the cap adds as little as none.
    let long = with(2, |f| f.estimate = Some(600));
    assert_eq!(score(&long.task, today()), 0.0);
}

#[test]
fn ties_break_by_id() {
    let tasks = vec![task(9), task(3), task(7), task(1)];
    assert_eq!(order(tasks), [1, 3, 7, 9]);
    let equal = |id| with(id, |f| f.priority = 2);
    assert_eq!(order(vec![equal(5), equal(2), equal(4)]), [2, 4, 5]);
}

#[test]
fn score_is_deterministic() {
    let t = with(1, |f| {
        f.priority = 2;
        f.deadline = Some("2026-11-10".into());
        f.estimate = Some(30);
    });
    assert_eq!(score(&t.task, today()), score(&t.task, today()));
    assert!((score(&t.task, today()) - (20.0 + 21.0 + 3.75)).abs() < 1e-9);
}
