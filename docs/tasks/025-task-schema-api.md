---
id: 025
title: Add the M2 task schema and task create/get/update/complete API
milestone: M2
status: todo
deps: []
touches: [server/migrations/0005_tasks.sql, server/src/db/migrate.rs+, server/src/tasks/, server/src/lib.rs+, server/src/app.rs+, server/Cargo.toml, Cargo.lock, server/tests/tasks.rs, server/tests/common/mod.rs, server/API.md, docs/decisions/]
spec: ["3. Data Model", "Tasks (GTD)"]
---

## Goal
The database holds every M2 table in one migration. The owner can create, read and update tasks with all their GTD fields, including a checklist, and complete or reopen them over REST. Later M2 tasks add modules only, with no further migrations.

## Acceptance criteria
- [ ] `server/migrations/0005_tasks.sql`, appended to `MIGRATIONS` (one line), creates the following tables, with times in Unix seconds and dates as TEXT:
  - `tasks (object_id PK → objects ON DELETE CASCADE, status TEXT 'open'|'done' default 'open', priority INTEGER 1..4 default 4, scheduled TEXT NULL, deadline TEXT NULL, estimate INTEGER NULL, someday 0/1, parent_id → objects ON DELETE SET NULL, project_id → objects SET NULL, section_id → sections SET NULL, area_id → objects SET NULL, checklist TEXT JSON default '[]', recurrence TEXT JSON NULL, completed INTEGER NULL) STRICT`. Indexes on `parent_id`, `project_id`, `area_id`, `scheduled` and `deadline`.
  - `projects (object_id PK → objects CASCADE, status TEXT 'active'|'someday'|'done' default 'active', area_id → objects SET NULL, start TEXT NULL, deadline TEXT NULL) STRICT`.
  - `sections (id INTEGER PRIMARY KEY AUTOINCREMENT, project_id → objects CASCADE, title, position INTEGER, created) STRICT`, indexed on `project_id`.
  - `recurrence` stays `NULL` until task 027.
- [ ] New module `server/src/tasks/` (`mod.rs` with its own `routes()`, plus `store.rs` and `validate.rs`), registered with one line each in `lib.rs` and `app.rs`. Every route takes `auth::Session`.
- [ ] Task JSON is the object JSON plus `"task":{"status","priority","scheduled","deadline","estimate","someday","parent_id","project_id","section_id","area_id","checklist":[{"text","done"}],"recurrence":null,"completed":<RFC 3339>|null}`. An object of type `task` created through `POST /api/objects` has no `tasks` row and is served with the defaults (LEFT JOIN, no write on read).
- [ ] Field rules. Each failure is `400 {"error":"..."}`, and each has a test:
  - `priority` is 1–4. P4 means "no priority" (default — owner may change).
  - `scheduled` and `deadline` are a real calendar date `YYYY-MM-DD` or a floating local time `YYYY-MM-DDTHH:MM`, with no time zone (default — owner may change).
  - `estimate` is whole minutes, 1–10080.
  - `parent_id` is a live task, not the task itself, and not one of its descendants (`a task cannot be its own ancestor`).
  - `project_id` is a live `project`, and `area_id` is a live `area`.
  - `section_id` must belong to the task's project (`section is not in this project`). Changing `project_id` without giving `section_id` clears the section.
  - `checklist` has at most 100 items. Each `text` is trimmed and 1–500 chars.
- [ ] `POST /api/tasks` takes `{title, tags?, properties?, pinned?, task?:{...fields}}` and returns `201` with the task JSON. Tags and title use the objects rules. The object and the task row are written in one transaction.
- [ ] `GET /api/tasks/{id}` returns `200`. `PATCH /api/tasks/{id}` takes any task fields except `status`/`completed` (a `null` clears a nullable field), bumps the object's `updated`, and returns `200`. An id that is unknown, trashed, or not a task gets `404 {"error":"task not found"}`.
- [ ] `POST /api/tasks/{id}/complete` sets `status: done` and `completed`, and returns `200 {"task":{...},"next":null}`. Completing an already done task is a no-op `200`. `POST /api/tasks/{id}/reopen` sets `open`, clears `completed`, and returns `200` with the task.
- [ ] 025 adds `chrono` (default features off; `clock` and `std` on) to `server/Cargo.toml` and `Cargo.lock`. It uses `chrono::NaiveDate`/`NaiveDateTime` for the date rules above and `chrono::Utc::now().date_naive()` for the server's "today". Later M2 server tasks (026, 027, 031) rely on this dependency and don't edit `server/Cargo.toml` for dates. `server/API.md` names it.
- [ ] `tasks::store::{create, get, get_many(&Db, &[i64]) -> Vec<Task> (in the given order), update, complete, reopen}` are documented in `server/API.md`, so that lists (026) and smart add (031) reuse them.
- [ ] `server/tests/common/mod.rs` gains `create_task(&Db, token, title, task: Value) -> i64`. Later M2 server tests use it and don't edit `common`.
- [ ] `cargo test -p polykov-server --test tasks` passes. It covers create, get, patch, every rule above, complete/reopen, 404s and 401.
- [ ] A new ADR, with the next free number, `docs/decisions/NNNN-task-fields.md`, covers task and project fields in side tables (amending ADR 0007), floating local dates, and status values.

## Out of scope
- GTD lists (026), recurrence (027), project and section routes (028), smart add (031).
- Goal/milestone, actual time and buffer fields: these come in M6, M8 and M9.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
