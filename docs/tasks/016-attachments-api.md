---
id: 016
title: Add attachment upload, download and delete stored in the data dir
milestone: M1
status: done
deps: [013]
touches: [server/src/attachments/, server/src/db/mod.rs, server/src/lib.rs+, server/src/app.rs+, server/tests/attachments.rs, server/API.md, docs/decisions/]
spec: ["3. Data Model", "2. Architecture & Stack", "Tasks (GTD)"]
---

## Goal
Any object can have file attachments. Files live in `<data dir>/attachments/`, with metadata in the `attachments` table from 013.

## Acceptance criteria
- [ ] `Db::data_dir(&self) -> &Path` exists. The module `server/src/attachments/` has its own `routes()` (one line in `app.rs`), and every route requires auth.
- [ ] `POST /api/objects/{id}/attachments?name=<file name>` with the raw file as the body (no multipart) returns `201 {"id","object_id","name","mime","size","created"}`:
  - `mime` is the request `Content-Type`, defaulting to `application/octet-stream`
  - the name must be 1–255 chars with no `/`, `\`, NUL or control chars, else `400`
  - an unknown or trashed object gets `404`
  - a body over 25 MiB gets `413` (default — owner may change)
- [ ] The file is written to `attachments/<id>` through a temp file and a rename. If the write fails, no row remains.
- [ ] `GET /api/objects/{id}/attachments` returns `200 {"attachments":[...]}`, oldest first.
- [ ] `GET /api/attachments/{aid}` returns the bytes with:
  - the stored `Content-Type`
  - `Content-Disposition: attachment; filename*=UTF-8''<percent-encoded>`
  - `X-Content-Type-Options: nosniff`
  - `?inline=1` switches to `inline` for `image/*` only, for `<img>` previews
- [ ] `DELETE /api/attachments/{aid}` removes the row and the file and returns `204`. An unknown id, or one on a trashed object, gets `404 {"error":"attachment not found"}`.
- [ ] `attachments::remove_files(&Db, object_id)` deletes the files of one object (for the trash purge in 017) and is documented in `server/API.md`.
- [ ] `server/tests/attachments.rs` covers upload/list/download round-trip, bytes on disk, bad names, 413, 404, 401, delete removing the file, and `remove_files`. It passes.
- [ ] ADR in `docs/decisions/` (≤ 15 lines): raw-body upload, files named by id, the size limit, and forced-download headers.

## Out of scope
- Image thumbnails, paste/drop in the note editor (M4), and voice uploads (M11).

## Notes for next
- 017 purge: call `attachments::remove_files(&db, id)` before `DELETE FROM objects` (rows cascade, files don't).
- `?inline=1` also refuses `image/svg+xml` (scripts on our origin); ADR 0008.
- A non-ASCII `Content-Type` on upload gets `400 {"error":"Content-Type must be ASCII"}`; a missing `name` is the same 400 as an empty one.
- `Db` now carries its data dir (`Db::data_dir()`); test `DataDir` does not pre-create `attachments/`, `store::create` makes it on demand.
- The file write runs while holding the DB mutex (inside the row's transaction); fine for one user, revisit if uploads block other requests.
