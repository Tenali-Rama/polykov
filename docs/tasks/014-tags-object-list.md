---
id: 014
title: Add hierarchical tags and the filtered object list
milestone: M1
status: done
deps: [013]
touches: [server/src/tags/, server/src/objects/, server/src/lib.rs+, server/src/app.rs+, server/tests/tags.rs, server/tests/object_list.rs, server/API.md]
spec: ["3. Data Model", "Organization"]
---

## Goal
Objects carry hierarchical tags that act as folders, and the owner can list objects filtered by type, tag folder, pinned and archived. This covers the archive/favourites browsing that spec Organization asks for.

## Acceptance criteria
- [ ] New module `server/src/tags/` with `normalize(&str) -> Result<String, _>`, its own `routes()` (one line in `app.rs`) and the `object_tags` store.
- [ ] Tag rules:
  - normalizing strips a leading `#`, trims, and lowercases (default — owner may change)
  - segments are split by `/`, each non-empty, with no whitespace, `#` or `,`
  - a tag is at most 100 chars, and an object has at most 50 tags
  - violations get `400 {"error":"invalid tag: <input>"}`
  - duplicates after normalizing collapse into one
- [ ] `POST /api/objects` and `PATCH /api/objects/{id}` accept `tags: [..]`. On PATCH it replaces the set and bumps `updated`. Object JSON returns `tags` sorted.
- [ ] `GET /api/objects` returns `200 {"objects":[...],"total":<n>}`:
  - live objects only
  - optional `type=`, `tag=` (matches the tag and everything under it: `tag=work` matches `work` and `work/clienta/design`, but not `workshop`), `pinned=true|false` and `archived=false|true|any` (default `false`)
  - `limit` (default 100, max 1000) and `offset`
  - ordered pinned first, then `updated` desc, then `id` desc
  - a bad parameter value gets `400`
- [ ] `GET /api/tags` returns `200 {"tags":[{"tag":"work","count":2},{"tag":"work/clienta","count":1}]}`, sorted by tag:
  - it includes implicit parent folders
  - `count` is the number of live, non-archived objects carrying that tag or a descendant
- [ ] `server/tests/tags.rs` covers normalization, invalid tags, replace on patch and `/api/tags` counts with parents. `server/tests/object_list.rs` covers each filter, prefix vs `workshop`, archived default, ordering, paging and that trashed objects are excluded. Both pass.
- [ ] A test seeds 10k objects and asserts that `GET /api/objects?tag=x` finishes in < 100 ms in release builds (skip the timing assert in debug).

## Out of scope
- Full-text search, the view query language and saved views (M3). Renaming tags.

## Notes for next
- Defaults chosen: normalize = trim, strip one leading `#`, lowercase (so `" #Work"` works); more than 50 distinct tags gets `400 an object can have at most 50 tags`; an unknown list query parameter gets axum's standard `400`; `limit=0` is rejected.
- `GET /api/tags` lists only folders used by live, non-archived objects, so no entry ever has `count: 0`.
- Tag prefix filter uses the range `tag >= t||'/' AND tag < t||'0'` on the existing `object_tags_tag` index (no migration); M3 view queries (`tag:work/*`) can reuse `tags::store::under`.
- Object JSON reads tags via `json_group_array(tag ORDER BY tag)` (`tags::store::SORTED_JSON`), needs SQLite ≥ 3.44 (bundled is newer).
- The 10k timing assert only runs under `cargo test --release`; verify runs debug, so it only checks correctness there.
