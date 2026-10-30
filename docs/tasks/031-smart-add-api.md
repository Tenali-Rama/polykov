---
id: 031
title: Add the smart add API with live parse and add-to-inbox
milestone: M2
status: done
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
  - the spec example `call mom tmrw 5pm 30m p2 #family every sunday` with `today=2026-10-30` → title `call mom`, `scheduled` `2026-10-31T17:00`, `estimate` 30, `priority` 2, `tags` `["family"]`, `recurrence` `{"rule":"FREQ=WEEKLY;BYDAY=SU","from":"scheduled"}`, and `deadline`/`project_id`/`area_id` `null`. It appears in `GET /api/lists/upcoming?today=2026-10-30`
  - project and area resolution
  - both 400s
  - UTF-16 offsets
  - 401
- [ ] The < 20 ms budget is checked by 030's release bench. `server/tests/smartadd.rs` adds a loose guard: `POST /api/parse` of a 200-char line answers in < 100 ms in the debug build.
- [ ] `server/API.md` updated.

## Out of scope
- Voice input (M11), Telegram (M10), the auto-tagger (M11).

## Notes for next
- `smartadd::add(&Db, text, today)` is blocking and returns `ApiError`; Telegram (M10) and voice (M11) should call it inside `http::blocking`.
- `resolved` comes from running `RulesParser` on the token text alone, so it uses the parser's own name folding and isn't a second copy of that rule.
- The parser is a `thread_local!` (`Box<dyn Parser>` is not `Sync`), so each blocking-pool thread reads the model once.
- Defaults: `priority` in the parse preview is `null` when not given; `text` over 1000 *characters* is a 400; the unknown-name check runs before the empty-title check.
- The spec-example and Inbox tests read back through `GET /api/lists/upcoming|inbox` (026), so a list-rule change shows up here too.
