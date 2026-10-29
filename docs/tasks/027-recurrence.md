---
id: 027
title: Add recurring tasks with fixed-date and after-completion rules
milestone: M2
status: todo
deps: [025]
touches: [server/src/recur/, server/src/tasks/, server/src/lib.rs+, server/tests/recurrence.rs, server/API.md, docs/decisions/]
spec: ["Tasks (GTD)", "10. Non-Functional"]
---

## Goal
A task can carry a recurrence rule. Completing it creates the next occurrence, counted from the scheduled date or from the completion date.

## Acceptance criteria
- [ ] New pure module `server/src/recur/` (no DB, no HTTP), registered with one line in `lib.rs`:
  - `Rule { freq: Day|Week|Month|Year, interval: 1..=365, by_day: Vec<Weekday> (Week only), month_day: Option<1..=31> (Month only) }`.
  - `Rule::parse(&str)` reads the RRULE subset `FREQ=DAILY|WEEKLY|MONTHLY|YEARLY[;INTERVAL=n][;BYDAY=MO,TU,..][;BYMONTHDAY=n]`, and `Display` gives the canonical form.
  - `next(&Rule, after: NaiveDate) -> NaiveDate` is strictly after `after`. A month day past the month's end clamps to its last day (default — owner may change).
- [ ] Unit tests in `recur` (spec §10 asks for recurrence unit tests) cover:
  - every freq and interval
  - `BYDAY` with several days, crossing a week boundary
  - 31st → Feb 28/29
  - Feb 29 yearly
  - year rollover
  - parse errors and canonical round-trip
- [ ] The task field `recurrence` is `{"rule":"FREQ=WEEKLY;BYDAY=SU","from":"scheduled"|"completion"}` or `null`. It is accepted by `POST /api/tasks` and `PATCH /api/tasks/{id}`, and stored as JSON in `tasks.recurrence`. A bad rule gets `400 {"error":"invalid recurrence: <reason>"}`.
- [ ] `POST /api/tasks/{id}/complete[?today=YYYY-MM-DD]` on a recurring task returns `200 {"task":{...done},"next":{...}}`. `next` is a new task with:
  - the same title, tags, properties, priority, estimate, project, section, area, parent and recurrence
  - every checklist item unchecked, `someday` false
  - `scheduled` = `next(anchor)` with the same time of day, where the anchor is the scheduled date, else the deadline, else `today` for `from: scheduled`, and `today` for `from: completion`
  - `deadline` shifted by the same number of days, if it had one
  - The done task's `recurrence` is set to `null`, so reopening and re-completing it doesn't fork the series (default — owner may change).
- [ ] `cargo test -p polykov-server --test recurrence` covers both `from` modes, the anchor fallbacks, the deadline shift, checklist reset, a non-recurring complete (`next: null`) and the 400.
- [ ] `server/API.md` and a new ADR, with the next free number, `docs/decisions/NNNN-recurrence.md`, record the rule format and the spawn-on-complete behaviour.

## Out of scope
- Parsing natural-language recurrence like "every sunday" (029/031). Reminders for recurring tasks (M9).

## Notes for next
<!-- builder fills in -->
