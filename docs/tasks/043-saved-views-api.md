---
id: 043
title: Add saved views with grouping, sort and layout-shaped results
milestone: M3
status: todo
deps: [041, 042]
touches: [server/src/views/, server/src/lib.rs+, server/src/app.rs+, server/tests/views.rs, server/API.md, docs/decisions/0016-saved-views.md]
spec: ["Organization", "Tasks (GTD)", "3. Data Model"]
---

## Goal
A view is a saved query with its own layout, grouping and sort. Its items endpoint returns results already grouped for list, Kanban or Eisenhower, so every client renders without re-implementing the rules.

## Acceptance criteria
- [ ] Views are objects of type `view` (no new table). Their `properties` hold:
  - `query`: string, must parse with 040
  - `layout`: `list|kanban|eisenhower`
  - `group`: `none|status|priority|project|section|area|tag|type`
  - `sort`: `smart|priority|deadline|scheduled|created|updated|completed|title`
  - Each sort has a fixed direction: dates newest-first for `created`/`updated`/`completed`, ascending otherwise.
  - Pinning a view uses the object `pinned` flag.
  - ADR `docs/decisions/0016-saved-views.md` records this, and that the web route for a view is `#/v/<id>`.
- [ ] `server/src/views/` owns its `routes()`. The only hub edits are one line each in `lib.rs` and `app.rs`.
- [ ] Endpoints (auth required):
  - `POST /api/views` with `{"title","query","layout","group","sort","pinned"}` returns `201` with the view. A bad query gets `400` with `at`, and a bad enum value gets `400` naming the allowed values.
  - `PATCH /api/views/{id}` validates the same way.
  - `GET /api/views` lists live views, pinned first, then by title.
  - Rename, delete and pin go through the existing object routes.
- [ ] `GET /api/views/{id}/items?today=YYYY-MM-DD` returns `{"view":{...},"groups":[{"key":...,"label":"...","items":[...]}],"total":<n>,"truncated":<bool>}`.
  - At most 500 items (default — owner may change), sorted within each group.
  - `group: none` gives one group.
  - `kanban` with `group: none` groups by `status`.
  - `eisenhower` ignores `group` and returns four fixed quadrants: urgent + important, important, urgent, neither. Important means priority ≤ 2. Urgent means overdue, or a deadline or scheduled date within 2 days of today (default — owner may change).
  - Unknown view id or a non-view object returns `404 {"error":"view not found"}`.
  - A stored view whose properties no longer validate gets `422` naming the field.
- [ ] Group keys: status, priority, project, section and area groups list the empty key (`null`, label `none`) last. `tag` groups by each object's first tag (default — owner may change).
- [ ] `server/tests/views.rs` covers:
  - create, patch and list validation
  - each group kind
  - each sort including `smart`
  - the four Eisenhower quadrants with boundary dates
  - truncation at 500
  - `404`, `422` and `401`
- [ ] `server/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Calendar and timeline layouts (M8). Front-matter property filters (M4). Web UI (045–047). CLI views (M15).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
