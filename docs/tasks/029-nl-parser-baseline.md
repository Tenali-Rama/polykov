---
id: 029
title: Add the polykov-parse crate with the parser trait and a rules baseline
milestone: M2
status: todo
deps: []
touches: [parse/, Cargo.toml, Cargo.lock, Dockerfile, docs/decisions/]
spec: ["Capture", "5. Intelligence"]
---

## Goal
A new workspace crate parses a smart-add line into a title plus typed tokens, behind a stable trait. Its first implementation is a hand-written rules parser, which serves as the experiment baseline (spec §5).

## Acceptance criteria
- [ ] New member `parse/` (package `polykov-parse`) in the root `Cargo.toml`, with `parse/CLAUDE.md` and `parse/API.md`. The `Dockerfile` copies `parse/`, so `docker build` still works.
- [ ] Interface: `trait Parser { fn parse(&self, input: &str, ctx: &Context) -> Parsed }`.
  - `Context { today: NaiveDate, projects: Vec<String>, areas: Vec<String> }`
  - `Parsed { title, tokens: Vec<Token{start, end (byte offsets), kind, text}>, scheduled, deadline, estimate_min, priority, recurrence: Option<{rule, from}>, project, area, tags }`
  - `Kind` is one of `Date Time Duration Priority Project Area Tag Deadline Recurrence`.
  - Dates and times are produced in the 025 formats, and rules in the 027 RRULE subset.
- [ ] `RulesParser` implements the syntax below (default — owner may change). Each item has a unit test:
  - `#tag` (hierarchical `#a/b`)
  - `+project` and `@area`: matched against `ctx` names ignoring case, with `-`/`_` = space. An unknown name is still a token, with no resolved value.
  - `p1`–`p4`
  - durations `30m`, `45min`, `2h`, `1h30m`, `1.5h`
  - dates `today`/`tod`, `tomorrow`/`tmrw`/`tmr`, weekday names and 3-letter abbreviations (next such day, today included), `next week` (next Monday), `next month` (the 1st), `in N days|weeks|months`, `nov 12`/`12 nov`/`november 12` (next year if already past), `2026-11-12`
  - times `5pm`, `5:30pm`, `17:00`, `at 17`. A time alone means today.
  - deadlines `due <date>` / `by <date>`
  - recurrence `every day|weekday|week|month|year`, `daily`/`weekly`/`monthly`/`yearly`, `every <weekday>[, and <weekday>]`, `every N days|weeks|months|years`, `every 15th`. A `every!` prefix means from completion. A recurrence with no date also sets `scheduled` to its first occurrence on or after today.
  - `"quoted text"` is never parsed, and the quotes are dropped from the title.
- [ ] The title is the input minus tokens, with whitespace collapsed. The spec example `call mom tmrw 5pm 30m p2 #family every sunday` with `today` = 2026-10-30 (a Friday) gives title `call mom`, scheduled `2026-10-31T17:00` (an explicit date wins over the rule's first occurrence), estimate 30, priority 2, tags `["family"]`, recurrence `{rule: "FREQ=WEEKLY;BYDAY=SU", from: scheduled}`, and deadline/project/area all `None`. Its tokens come out in input order: `tmrw` Date, `5pm` Time, `30m` Duration, `p2` Priority, `#family` Tag, `every sunday` Recurrence.
- [ ] Plain titles stay plain: `read 1984`, `buy 2 apples`, `email may`, `watch sunday times` (only `sunday` → date is acceptable) each have a test documenting their result.
- [ ] `cargo test -p polykov-parse` passes. The crate has no I/O and no network.
- [ ] A new ADR, with the next free number, `docs/decisions/NNNN-parse-crate.md`, records the crate, the trait and the syntax. Date math uses `chrono`, or another crate named in the ADR.

## Out of scope
- The benchmark and the candidate comparison (030). HTTP endpoints (031).

## Notes for next
<!-- builder fills in -->
