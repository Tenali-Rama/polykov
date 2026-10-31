---
id: 064
title: Turn wikilinks in note bodies into links and serve their resolution
milestone: M4
status: todo
deps: [062]
touches: [server/migrations/0008_link_origin.sql, server/src/db/migrate.rs, server/src/notes/wikilinks.rs, server/src/notes/mod.rs, server/src/notes/store.rs, server/src/links/mod.rs, server/src/links/store.rs, server/tests/wikilinks.rs, server/tests/links.rs, server/API.md, docs/decisions/0020-wikilinks.md]
spec: ["Notes", "3. Data Model"]
---

## Goal
Writing `[[Title]]` in a note links the note to that object, so the existing backlinks show it, and clients can tell which wikilinks resolve.

## Acceptance criteria
- [ ] Syntax (default — owner may change): `[[Title]]`, `[[Title|label]]` and the embed form `![[Title]]`, but not inside code spans or fenced code. A title resolves to the live object with exactly that title, ignoring case. If several objects match, the most recently `updated` wins.
- [ ] Migration `0008_link_origin.sql` (the next free number; `migrate.rs` is a normal edit, after 062's 0007) adds `links.origin TEXT NOT NULL DEFAULT 'manual'`, with `'wikilink'` for links made from a body. Saving a note's body (create or `PATCH`) replaces that note's `wikilink` links with the resolved set. A `manual` link to the same target stays `manual` and is never removed by a save. The link JSON from `GET /api/objects/{id}/links` gains `origin`.
- [ ] `GET /api/notes/{id}/wikilinks` returns `[{"text":"Title","label":null,"embed":false,"target":{id,type,title}|null}]` in body order. `GET /api/resolve?title=<t>` returns `{id,type,title}` or 404 (for the editor's autocomplete and embeds).
- [ ] `server/tests/wikilinks.rs` covers:
  - a link appears in the target's backlinks
  - editing it out removes it
  - a manual link survives
  - code is ignored
  - an unresolved title gives `target: null`, and a later object with that title resolves on the next save
  - ties, embeds and `401`

  `server/tests/links.rs` still passes, with `origin` asserted once.
- [ ] ADR `docs/decisions/0020-wikilinks.md` (≤ 15 lines): title-based syntax, resolve at save time, the `origin` column, and the fact that renaming a target doesn't rewrite bodies. `server/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Rewriting bodies on rename. Web UI (070).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
