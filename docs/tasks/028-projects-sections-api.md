---
id: 028
title: Add project fields, the project list and project sections API
milestone: M2
status: todo
deps: [025]
touches: [server/src/projects/, server/src/lib.rs+, server/src/app.rs+, server/tests/projects.rs, server/API.md]
spec: ["Tasks (GTD)", "3. Data Model"]
---

## Goal
Projects have a status, an area and dates, can be listed, and hold ordered sections that tasks are filed under.

## Acceptance criteria
- [ ] New module `server/src/projects/`, with its own `routes()`, registered with one line each in `lib.rs` and `app.rs`. It uses the `projects`/`sections` tables from 025 and adds no migration. Every route takes `auth::Session`.
- [ ] Project JSON is the object JSON plus `"project":{"status","area_id","start","deadline","open_tasks":n,"done_tasks":n}`. A `project` object without a `projects` row is served with defaults (`active`, nulls).
- [ ] `GET /api/projects?status=active|someday|done|any&area=<id>` (status defaults to `active`) returns `200 {"projects":[...]}` with live, non-archived projects ordered by title. `GET /api/projects/{id}` returns `200`. `PATCH /api/projects/{id}` takes `status`, `area_id` (a live `area`, or `null`), `start` and `deadline` (same date rules as tasks, 025), upserts the row and bumps `updated`.
- [ ] A non-project or trashed id gets `404 {"error":"project not found"}`. A bad field gets `400 {"error":"..."}`.
- [ ] Sections:
  - `GET /api/projects/{id}/sections` returns `200 {"sections":[{"id","project_id","title","position"}]}`, by position.
  - `POST /api/projects/{id}/sections {title, position?}` returns `201`. The section is appended last by default, and an explicit position shifts later ones.
  - `PATCH /api/sections/{sid} {title?, position?}` renames and/or moves the section. Positions stay a dense `0..n`.
  - `DELETE /api/sections/{sid}` returns `204`. Its tasks keep their project and lose the section.
  - The title is trimmed and 1–200 chars (default — owner may change). An unknown section gets `404 {"error":"section not found"}`.
- [ ] `cargo test -p polykov-server --test projects` passes. It covers list filters, counts, patch rules, section add/move/rename/delete with dense positions, the deleted section's tasks, 404s and 401.
- [ ] `server/API.md` documents the routes.

## Out of scope
- Areas need no new routes. They are `type=area` objects, listed through `GET /api/objects?type=area`.
- Goal links (M9). Project timeline layouts (M8).

## Notes for next
<!-- builder fills in -->
