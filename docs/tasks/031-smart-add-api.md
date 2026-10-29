---
id: 031
title: Add the smart add API with live parse and add-to-inbox
milestone: M2
status: todo
deps: [025, 027, 028, 030]
touches: [server/src/smartadd/, server/src/lib.rs+, server/src/app.rs+, server/Cargo.toml, Cargo.lock, server/tests/smartadd.rs, server/API.md]
spec: ["Capture", "5. Intelligence", "Tasks (GTD)"]
---

## Goal
One line of text becomes a fully filled-in task in a single call. A separate parse call returns highlighted tokens for live preview in the clients.

## Acceptance criteria
- [ ] New module `server/src/smartadd/`, with its own `routes()`, registered with one line each in `lib.rs` and `app.rs`. It depends on `polykov-parse` and uses `default_parser()`. `Context` is filled with the titles of live, non-archived projects and areas. Every route takes `auth::Session`.
- [ ] `POST /api/parse {text, today?}` returns `200 {"title","tokens":[{"start","end","kind","text","resolved"}],"tags":[...],"task":{scheduled, deadline, estimate, priority, recurrence, project_id, area_id}}`.
  - `start`/`end` are UTF-16 code-unit offsets, so the web can slice JS strings. A test uses an emoji before a token.
  - `resolved` is false for an unknown `+project`/`@area`.
  - Text over 1000 chars gets `400`.
- [ ] `POST /api/tasks/add {text, today?}` parses the text and creates the task through `tasks::store::create`. It returns `201` with the task JSON.
  - With no project, area or date it lands in the Inbox (026 rules).
  - An unknown project/area gets `400 {"error":"unknown project: <name>"}` / `unknown area: <name>` and creates nothing.
  - An empty title after parsing gets `400 {"error":"title must be 1 to 1000 characters"}`.
- [ ] `smartadd::add(&Db, text, today) -> Result<Task, ApiError>` is public and documented, so Telegram (M10) reuses it.
- [ ] `cargo test -p polykov-server --test smartadd` covers:
  - the spec example `call mom tmrw 5pm 30m p2 #family every sunday` → all fields, appearing in `GET /api/lists/upcoming`
  - project and area resolution
  - both 400s
  - UTF-16 offsets
  - 401
- [ ] The < 20 ms budget is checked by 030's release bench. `server/tests/smartadd.rs` adds a loose guard: `POST /api/parse` of a 200-char line answers in < 100 ms in the debug build.
- [ ] `server/API.md` updated.

## Out of scope
- Voice input (M11), Telegram (M10), the auto-tagger (M11).

## Notes for next
<!-- builder fills in -->
