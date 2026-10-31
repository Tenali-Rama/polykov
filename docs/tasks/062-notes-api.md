---
id: 062
title: Add notes and nested notebooks with markdown bodies, indexed for search
milestone: M4
status: todo
deps: []
touches: [server/migrations/0007_notes.sql, server/src/db/migrate.rs, server/src/notes/, server/src/search/store.rs, server/src/search/fuzzy.rs, server/src/lib.rs+, server/src/app.rs+, server/tests/notes.rs, server/tests/search.rs, server/API.md, docs/decisions/0018-notes.md, docs/decisions/0014-search-index.md]
spec: ["Notes", "3. Data Model"]
---

## Goal
Notes hold a markdown body and belong to a notebook. Notebooks nest. Note bodies are found by global search.

## Acceptance criteria
- [ ] Migration `0007_notes.sql` (`migrate.rs` is a normal edit, because 064 adds 0008 after this) creates two tables:
  - `notes (object_id PK → objects ON DELETE CASCADE, notebook_id → objects ON DELETE SET NULL, body TEXT NOT NULL DEFAULT '')`
  - `notebooks (object_id PK → objects ON DELETE CASCADE, parent_id → objects ON DELETE SET NULL)`

  It also rebuilds the FTS `search` table with a `body` column (column weight 1, as `props`) and adds triggers on `notes`, so editing a body re-indexes it. The front-matter properties are the object's `properties`; the body never holds them.
- [ ] Routes, all requiring auth. A note is the object JSON plus `"note":{"notebook_id":n|null,"body":"..."}`, and a notebook is the object JSON plus `"notebook":{"parent_id":n|null}`.
  - `POST /api/notes`, `GET /api/notes/{id}`, `PATCH /api/notes/{id}`: `title`, `tags`, `properties`, `pinned`, `archived`, `notebook_id`, `body`.
  - `GET /api/notes?notebook=<id>|none&limit&offset`: list items carry no `body`; they're ordered pinned first, then `updated` newest first.
  - `POST /api/notebooks`, `PATCH /api/notebooks/{id}` (`title`, `parent_id`).
  - `GET /api/notebooks`: a flat list, each with `notes` (live note count), ordered by title ignoring case.
  - An object made through `POST /api/objects` with type `note` or `notebook` reads with the defaults.
- [ ] Rules, each a `400`:
  - `notebook_id` must be a live notebook.
  - `parent_id` must be a live notebook that isn't the notebook itself or one of its descendants (`a notebook cannot be its own ancestor`).
  - `body` must be ≤ 1 MB of UTF-8 (default — owner may change).

  A notebook that's in the trash counts as none. Trashing a notebook leaves its notes in place.
- [ ] `server/tests/notes.rs` covers each route, each rule, the cycle check, list filtering by `notebook`, and `401`. `server/tests/search.rs` adds this case: a word only in a note body is found, and stops being found after the body changes.
- [ ] `cargo test -p polykov-server --test search_perf` stays green (bodies are empty there).
- [ ] ADR `docs/decisions/0018-notes.md` (≤ 15 lines): side tables, markdown as the stored body, front-matter = object properties. ADR 0014 gains the body column. `server/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Wikilink parsing (064), the property query terms (063), and any web UI.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
