---
id: 054
title: Fix Eisenhower urgency for done tasks, share query binding and tighten query dates and tags
milestone: M4
status: done
deps: []
touches: [server/src/views/items.rs, server/src/views/keys.rs, server/src/query/mod.rs, server/src/query/parse.rs, server/src/query/tests.rs, server/tests/views/items.rs, server/API.md, docs/decisions/0015-query-language.md, docs/decisions/0016-saved-views.md]
spec: ["Organization", "Tasks (GTD)"]
---

## Goal
Three M3 review findings on the server: done tasks no longer count as urgent, `views::keys` stops duplicating `query::run`'s parameter binding, and two query parser edge cases are rejected.

## Acceptance criteria
- [ ] Eisenhower (`server/src/views/items.rs`): only an `open` task can be urgent. A done task with a past deadline or scheduled date goes to `important` (P1/P2) or `neither`. A test in `server/tests/views/items.rs` seeds a done P1 task with yesterday's deadline and asserts it is in `important`, and an open one in `urgent & important`.
- [ ] `views/keys.rs` has no copy of the binding code: `query` exposes one `pub(crate)` function (e.g. `query::bind(conn, &[Param])`) that both `query::run` and `views::keys::select` call. `cargo test -p polykov-server query views` passes unchanged.
- [ ] Query parser (`query/parse.rs`), each with a unit test in `query/tests.rs`:
  - `due:-12345678d` (a 10-char signed offset) parses as a relative offset (and then gets `400 date is out of range` from compile), not as a date. `due:2026-10-31` still parses as a date.
  - `tag:work/*/*` and any `*` other than one trailing `/*` is a parse error with the offset of the `*`, e.g. `tag wildcard must be a single trailing /*`.
- [ ] ADR 0016 and `server/API.md` say "urgent: an open task whose ..."; ADR 0015 notes the tag wildcard rule.
- [ ] `scripts/verify.sh` passes.

## Out of scope
- Any new query field or layout. Web changes.

## Notes for next
- Eisenhower urgency now checks `Key.status == "open"`; done tasks fall to `important`/`neither` by priority.
- `query::bind` (crate-internal, defined in `query/compile.rs`) is the single binder for `Param`s; `views::keys` calls it. Making it `pub(crate)` was a one-word edit to `compile.rs`, outside `touches`.
- `DateValue::resolve` now returns `None` outside the years 0000–9999 (not just outside chrono's range), so `-12345678d` gets `400 date is out of range` as the criterion asks, and `YYYY-MM-DD` text comparison stays sound (chrono prints year 10000+ as `+10000-...`).
- A query date is absolute only in the exact `DDDD-DD-DD` shape; `tag:` rejects any `*` except one trailing `/*`, at the offset of the first bad `*`.
