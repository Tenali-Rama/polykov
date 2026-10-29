---
id: 013
title: Add the M1 object schema and object create/get/update/delete API
milestone: M1
status: todo
deps: []
touches: [server/migrations/0004_objects.sql, server/src/db/migrate.rs, server/src/objects/, server/src/http.rs, server/src/lib.rs+, server/src/app.rs+, server/tests/objects.rs, server/tests/common/mod.rs, server/API.md, docs/decisions/]
spec: ["3. Data Model"]
---

## Goal
The database holds every M1 table in one migration, and the owner can create, read, update and soft-delete generic objects over REST. Later M1 tasks only add modules, with no further migrations.

## Acceptance criteria
- [ ] `server/migrations/0004_objects.sql`, appended to `MIGRATIONS`, creates all M1 tables, with times in Unix seconds and FKs `ON DELETE CASCADE` to `objects`:
  - `objects (id INTEGER PRIMARY KEY AUTOINCREMENT, type, title, properties TEXT JSON default '{}', pinned, archived, created, updated, deleted_at NULL)`
  - `object_tags (object_id, tag, PK(object_id, tag))`
  - `links (src_id, dst_id, created, PK(src_id, dst_id), CHECK(src_id <> dst_id))`
  - `comments (id, object_id, body, created, updated)`
  - `attachments (id, object_id, name, mime, size, created)`
  - indexes on `object_tags(tag)`, `links(dst_id)`, `comments(object_id)`, `attachments(object_id)`
- [ ] `server/src/http.rs` has a shared `ApiError` (`IntoResponse`: `4xx {"error":"..."}`, or a bare `500` with a one-line log for store failures) and a `blocking(f)` helper, so feature modules don't copy auth's private ones.
- [ ] New module `server/src/objects/` with its own `routes()`, merged in `app.rs` with one line. Every route requires `auth::Session`.
- [ ] Object JSON: `{"id":1,"type":"note","title":"...","created":"<RFC 3339 UTC>","updated":"...","archived":false,"pinned":false,"tags":[],"properties":{}}`. `tags` stays `[]` until task 014.
- [ ] `POST /api/objects` with body `{type, title, properties?, pinned?, archived?}` returns `201` and the object. `400 {"error":...}` for:
  - a type outside the §3 list `task project area goal milestone habit note notebook journal_entry daily_note review event time_entry view forecast_record` (default — owner may change)
  - a title that is empty after trimming, or longer than 1000 chars (default — owner may change)
  - `properties` that is not an object, has more than 100 keys or a key not 1–64 chars, or has a value other than string/number/bool/null/array of those (no nested objects)
- [ ] `GET /api/objects/{id}` returns `200` with the object, or `404 {"error":"object not found"}` for unknown and soft-deleted ids.
- [ ] `PATCH /api/objects/{id}` takes any of `title, properties, pinned, archived`, validated as above. `properties` replaces the whole map. Any given field bumps `updated`, and `{}` returns the object unchanged. Unknown/trashed id gets `404`.
- [ ] `DELETE /api/objects/{id}` sets `deleted_at` (moves to trash) and returns `204`. Repeating it gets `404`.
- [ ] `objects::store::summary(&Db, id) -> Result<Option<Summary{id, type, title}>, _>` returns live (non-deleted) objects only, for links/comments/attachments to check targets. Documented in `server/API.md`.
- [ ] `server/tests/common/mod.rs` gains `token(&Db) -> String` (setup + login), `call(&Db, Method, path, token, Option<Value>) -> Reply`, and `create_object(&Db, token, type, title) -> i64`.
- [ ] `server/tests/objects.rs` covers create, each validation failure, get, patch, delete, 404s and 401, and `cargo test -p polykov-server --test objects` passes.
- [ ] ADR `docs/decisions/0007-object-model.md`: one `objects` table plus side tables, integer ids that are never reused (AUTOINCREMENT), Unix-second times, and relations served as sub-resources, not embedded in the object JSON.

## Out of scope
- Listing/filtering and tags (014), links and comments (015), attachments (016), the trash endpoints and purge (017).
- Type-specific fields (task status, dates, ...) come in later milestones.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
