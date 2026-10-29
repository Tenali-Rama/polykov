---
id: 026
title: Add the Inbox/Today/Upcoming/Someday/Anytime list API and sub-task listing
milestone: M2
status: todo
deps: [025]
touches: [server/src/lists/, server/src/lib.rs+, server/src/app.rs+, server/tests/lists.rs, server/API.md]
spec: ["Tasks (GTD)", "Capture"]
---

## Goal
Clients can fetch each GTD list and the sub-tasks of a task with one call.

## Acceptance criteria
- [ ] New module `server/src/lists/`, with its own `routes()`, registered with one line each in `lib.rs` and `app.rs`. It reads the `tasks`/`projects` tables with its own SQL and returns rows via `tasks::store::get_many`. It does not edit `server/src/tasks/`.
- [ ] `GET /api/lists/{name}?today=YYYY-MM-DD&project=<id>&area=<id>&limit=1..1000&offset=n` returns `200 {"tasks":[...],"total":n}`.
  - `today` defaults to the server's UTC date. Clients send their local date.
  - Each row is the task JSON plus `"subtasks":{"open":n,"total":n}`.
  - Only live, non-archived, `open` tasks are included.
  - A reference to a trashed project, area or parent counts as none.
- [ ] List rules (default — owner may change). Each one has a test with tasks on both sides of the boundary:
  - `inbox`: no project, no area, no parent, no scheduled date, no deadline, and not someday. Newest first.
  - `today`: scheduled date ≤ today, or deadline ≤ today, so overdue tasks are included. Any depth. Order: priority, then deadline (nulls last), then scheduled time, then `id`.
  - `upcoming`: scheduled date > today, or no scheduled date and deadline > today. Any depth. Ordered by that date, then priority.
  - `someday`: someday, no scheduled date, top-level (no live parent).
  - `anytime`: not someday, not in the inbox, scheduled null or ≤ today, top-level. Ordered by priority, then `id`.
  - `all`: every open top-level task. With `project=` it is the project view.
- [ ] `project=<id>` keeps only tasks in that project. `area=<id>` keeps tasks whose area is that area, or whose project's area is.
- [ ] An unknown list gets `404 {"error":"unknown list"}`. A bad `today`, `limit` or id gets `400 {"error":"..."}`.
- [ ] `GET /api/tasks/{id}/subtasks` returns `200 {"tasks":[...]}` with the direct children (open first, then done, each by `id`). It returns `404 {"error":"task not found"}` for a non-task.
- [ ] `cargo test -p polykov-server --test lists` passes, including 401 and a paging case.
- [ ] `server/API.md` documents both routes and the rules.

## Out of scope
- Saved views, query language and smart sort (M3). Completed history/logbook (M3).

## Notes for next
<!-- builder fills in -->
