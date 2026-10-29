---
id: 017
title: Add the trash with restore, permanent delete and 30-day auto-purge
milestone: M1
status: todo
deps: [013, 016]
touches: [server/src/trash/, server/src/main.rs+, server/src/lib.rs+, server/src/app.rs+, server/tests/trash.rs, server/API.md]
spec: ["3. Data Model"]
---

## Goal
Soft-deleted objects can be listed, restored or deleted for good, and they are purged on their own 30 days after deletion, attachment files included.

## Acceptance criteria
- [ ] Module `server/src/trash/` with its own `routes()` (one line in `app.rs`). All routes require auth.
- [ ] `GET /api/trash` returns `200 {"objects":[...]}`, newest deleted first. Each object has the usual JSON plus `"deleted":"<RFC 3339>"` and `"purge_at":"<deleted + 30 days>"`.
- [ ] `POST /api/trash/{id}/restore` clears `deleted_at` and returns `200` with the object (tags, links and comments intact). An id that is not in the trash gets `404 {"error":"not in trash"}`.
- [ ] `DELETE /api/trash/{id}` deletes the row (cascading tags, links, comments, attachment rows) and calls `attachments::remove_files`, then returns `204`. An id not in the trash gets `404`.
- [ ] `DELETE /api/trash` empties the trash and returns `200 {"purged":<n>}`.
- [ ] `trash::purge_expired(&Db, now_unix) -> Result<usize, _>` removes objects deleted more than 30 days before `now`. `trash::spawn_purger(Db)` runs it at start-up and then every hour (default — owner may change), and is called from `main.rs` with one line.
- [ ] `server/tests/trash.rs` covers list, restore, permanent delete (attachment file gone from disk), empty, and `purge_expired` with a `deleted_at` set 31 days back (purged) and 29 days back (kept). Live objects are never touched. It passes.

## Out of scope
- The general background job runner (M9). Backups (M17).

## Notes for next
<!-- builder fills in -->
