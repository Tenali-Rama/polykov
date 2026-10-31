---
id: 041
title: Compile view queries to SQL and serve GET /api/query
milestone: M3
status: done
deps: [039, 040]
touches: [server/src/query/compile.rs, server/src/query/routes.rs, server/src/query/mod.rs, server/src/app.rs+, server/tests/query.rs, server/API.md]
spec: ["Organization", "Tasks (GTD)", "3. Data Model"]
---

## Goal
Any query in the 040 language runs against the database. `GET /api/query?q=` returns the matching objects, with task fields included for tasks.

## Acceptance criteria
- [ ] `query::compile(&Query, today) -> (sql_where, params)`. It binds every value as a parameter and never formats user text into SQL. Text terms filter through the 039 search function.
- [ ] `query::run(&Db, &Query, today, limit, offset) -> Result<Page, _>` returns live objects, using the object list order (pinned first, then `updated` newest first, then `id`). Each task carries its `task` fields, as in the tasks API JSON.
- [ ] `GET /api/query?q=<query>&today=YYYY-MM-DD&limit=1..1000&offset=n` (auth required) returns `200 {"objects":[...],"total":<n>}`.
  - A parse error is `400 {"error":"...","at":<offset>}`.
  - A bad `today`, `limit` or `offset` gets the same `400` messages as the lists API.
  - An empty `q` matches every live, non-archived object.
- [ ] Integration tests in `server/tests/query.rs`:
  - the spec example `type:task priority:<=2 due:<7d tag:work/* -status:done` returns exactly the expected tasks from a seeded set
  - each field with at least one positive and one negative case
  - `due:none`; `archived:any` vs the default; `tag:work` vs `tag:work/*`
  - `project:"Title"` and `project:<id>`; a text term combined with fields
  - a `400` with `at`; `401` without auth
  - an SQL-injection-shaped value (`title'--`) is treated as text
- [ ] `server/API.md` updated with the endpoint and the `query::` functions. `scripts/verify.sh` passes.

## Out of scope
- Saved views, grouping and smart sort (042/043). CLI (M15).

## Notes for next
- `query::compile` returns `Compiled { sql_where, params }` over `query::FROM`; a text term is `Param::Search`, which `query::run` turns into a JSON id array via `search::matching_ids`. 042/043 can reuse `compile` and add their own ORDER BY.
- Defaults chosen: `status`/`priority`/`due`/`scheduled`/`completed`/`project` match tasks only (so `-status:done` keeps notes); `area:` also matches projects in the area; titles compare `COLLATE NOCASE`.
- `created`/`updated`/`completed` are Unix seconds and compare by their UTC date (`date(.., 'unixepoch')`); a local-day view would need the client's zone.
- A relative date beyond chrono's range is `400 date is out of range` (no `at`: the AST has no offsets). `q` is capped at 1000 chars.
- Results are fetched one object at a time (`tasks::store::get` / `objects::store::get`), fine for `limit` ≤ 1000.
