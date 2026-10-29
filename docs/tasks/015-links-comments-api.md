---
id: 015
title: Add links with backlinks and comments on any object
milestone: M1
status: todo
deps: [013]
touches: [server/src/links/, server/src/comments/, server/src/lib.rs+, server/src/app.rs+, server/tests/links.rs, server/tests/comments.rs, server/API.md]
spec: ["3. Data Model", "Tasks (GTD)"]
---

## Goal
Any object can link to any other object, backlinks are computed, and any object can carry comments. Each feature is its own module over the `links` and `comments` tables from 013.

## Acceptance criteria
- [ ] Modules `server/src/links/` and `server/src/comments/`, each with its own `routes()` merged in `app.rs` with one line each. All routes require auth. Targets are checked with `objects::store::summary`, so unknown or trashed objects get `404 {"error":"object not found"}`.
- [ ] `PUT /api/objects/{id}/links/{to}` returns `204` and is idempotent. Linking an object to itself gets `400 {"error":"cannot link an object to itself"}`.
- [ ] `DELETE /api/objects/{id}/links/{to}` returns `204`, or `404 {"error":"link not found"}`.
- [ ] `GET /api/objects/{id}/links` returns `200 {"links":[{"id","type","title"}],"backlinks":[...]}`. Both sides are ordered by title and skip trashed objects; a link reappears when its object is restored.
- [ ] `POST /api/objects/{id}/comments {"body"}` returns `201 {"id","object_id","body","created","updated"}`. The body is trimmed and must be 1–10000 chars (default — owner may change), else `400`.
- [ ] `GET /api/objects/{id}/comments` returns `200 {"comments":[...]}`, oldest first.
- [ ] `PATCH /api/comments/{cid} {"body"}` returns `200` with the comment and bumps its `updated`. `DELETE /api/comments/{cid}` returns `204`. An unknown comment, or one on a trashed object, gets `404 {"error":"comment not found"}`.
- [ ] `server/tests/links.rs` and `server/tests/comments.rs` cover every endpoint, the 400/404/401 paths, backlinks, and hiding trashed objects (soft-delete via `DELETE /api/objects/{id}`). Both pass.

## Out of scope
- Wikilinks parsed from note text (M4). Comment mentions or markdown rendering.

## Notes for next
<!-- builder fills in -->
