---
id: 039
title: Add the FTS5 search index and the fuzzy global search API
milestone: M3
status: done
deps: []
touches: [server/migrations/0006_search.sql, server/src/db/migrate.rs+, server/src/search/, server/src/lib.rs+, server/src/app.rs+, server/tests/search.rs, server/tests/search_perf.rs, server/API.md, docs/decisions/0014-search-index.md]
spec: ["Organization", "3. Data Model", "10. Non-Functional"]
---

## Goal
`GET /api/search?q=` finds any live object (archived and completed included) by full text with fuzzy matching, in under 100 ms on 100k objects. Later tasks (041 query language, 044 palette) reuse its match function.

## Acceptance criteria
- [ ] Migration `0006_search.sql` creates the FTS5 index and the triggers that keep it in sync on insert/update/delete of `objects`, `object_tags` and `comments`. It backfills existing rows. Indexed text: title, tags, string values of `properties`, and comment bodies (default — owner may change). Soft-deleted objects never match; restoring one makes it match again.
- [ ] `server/src/search/` owns its `routes()`. The only hub edits are one line each in `lib.rs`, `app.rs` and `migrate.rs`.
- [ ] `GET /api/search?q=<text>&type=<type>&limit=1..100&offset=n` (auth required) returns `200 {"results":[{"object":{...},"snippet":"...","score":<f64>}],"total":<n>}`, best match first.
  - `snippet` marks the matched text with `[` `]`.
  - `q` is trimmed and must be 1 to 200 characters, else `400`. FTS syntax characters in `q` never cause a `500`.
- [ ] Fuzzy behaviour, each with an integration test in `server/tests/search.rs`:
  - prefix: `meet` finds `Meeting notes`
  - substring: `ocer` finds `Groceries`
  - one typo: `groceris` finds `Groceries`
  - tags and comments: a tag `work/clienta` and a comment body are findable
  - archived and `done` objects are returned, each with its flags in the object JSON
  - trashed objects are not returned; restored ones are
  - `q` of `"`, `*`, `NEAR(` and `AND` returns `200`
  - unauthenticated requests get `401`
- [ ] `search::` exposes a function that later tasks call to filter by text (e.g. `search::matching_ids` or a SQL fragment plus params), documented in `server/API.md`.
- [ ] `server/tests/search_perf.rs` is an `#[ignore]` test. It seeds 100k objects with realistic titles, tags and properties in one transaction, then runs 20 varied queries (prefix, substring, typo). It asserts p95 < 100 ms. `cargo test --release -p polykov-server --test search_perf -- --ignored` passes, and the PR states the measured p50/p95.
- [ ] `docs/decisions/0014-search-index.md` records the tokenizer(s), the fuzzy strategy and what is indexed.
- [ ] `server/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- The view query language (040/041). Web UI (044). CLI search (M15). Note bodies (M4 indexes them through the same triggers or extends them).

## Notes for next
- Filter by text with `search::matching_ids(&conn, text)` (ranked ids, archived/done included, trashed never); bind them into your SQL as a JSON array via `json_each`.
- Matching rules live in `search::fuzzy` (words = letter/digit runs, substring, 1 typo for 6+ char words with no exact match anywhere); the FTS expression in `store::match_expr` must stay a superset of what `fuzzy::find` accepts (ADR 0014).
- New searchable text (note bodies, M4): add a column to `search` / extend the `search_source` view and its triggers in a new migration; never write `search` from Rust.
- A query made only of 1–2 char words can't use the trigram index and scans every row.
- Defaults chosen: `limit` default 20; task/project results carry their task/project JSON so `status` is visible.
