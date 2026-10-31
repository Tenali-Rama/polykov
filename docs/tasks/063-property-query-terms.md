---
id: 063
title: Add property terms to the view query language
milestone: M4
status: todo
deps: []
touches: [server/src/query/parse.rs, server/src/query/compile.rs, server/src/query/props.rs, server/src/query/mod.rs, server/src/query/tests.rs, server/tests/query.rs, server/API.md, docs/decisions/0019-property-queries.md]
spec: ["Notes", "Organization"]
---

## Goal
Front-matter properties (an object's `properties`) can be filtered in `GET /api/query` and in saved views, e.g. `type:note prop.status:draft prop.rating:>=4`.

## Acceptance criteria
- [ ] The grammar adds `prop.<key>:<value>[,<value>..]`, where `<key>` follows the property name rules (1–64 chars). Quoting works as for other values: `prop."due date":x`. It's negatable, and commas mean OR.
  - A plain value matches a string ignoring case, a number equal to it, `true`/`false`, or an array that contains it.
  - `<`, `<=`, `>`, `>=` compare numbers numerically when the value is a number. Otherwise they compare strings, so `YYYY-MM-DD` dates order correctly.
  - `prop.<key>:none` matches a missing key or `null`, and `-prop.<key>:none` means "has it".
- [ ] The SQL uses `json_extract`/`json_each` over `objects.properties`, with every key and value as a parameter. There's no string-built JSON path from user text that isn't escaped. A test with a key containing `"` and `.` covers this.
- [ ] Existing queries parse and compile the same: every test in `query/tests.rs` and `server/tests/query.rs` passes unchanged. New unit tests cover each form above and the error cases (`prop.:x`, a 65-char key). `server/tests/query.rs` adds integration cases over notes and tasks with properties, including a saved view whose query uses `prop.`.
- [ ] `compile.rs` stays ≤ 400 lines (put the property SQL in `query/props.rs`).
- [ ] ADR `docs/decisions/0019-property-queries.md` (≤ 15 lines). `server/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Grouping or sorting views by a property. Web editor hints.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
