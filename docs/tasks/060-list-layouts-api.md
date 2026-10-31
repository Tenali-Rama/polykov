---
id: 060
title: Serve GTD lists and project views grouped for the Kanban and Eisenhower layouts
milestone: M4
status: todo
deps: []
touches: [server/src/lists/mod.rs, server/src/lists/layout.rs, server/src/views/items.rs, server/src/views/keys.rs, server/src/views/mod.rs, server/tests/list_layouts.rs, server/API.md, docs/decisions/0022-list-layouts.md]
spec: ["Tasks (GTD)"]
---

## Goal
The spec says layouts are "available for any task list or view". M3 gave them only to saved views. After this task the GTD lists and the project view can be fetched in the same grouped shape as `GET /api/views/{id}/items`.

## Acceptance criteria
- [ ] `GET /api/lists/{name}/items?layout=list|kanban|eisenhower&group=<group>&sort=default|smart&project=<id>&area=<id>&today=YYYY-MM-DD` returns `200 {"groups":[...],"total":n,"truncated":bool}`, with the same group and item shapes as view items. The tasks are the ones `GET /api/lists/{name}` matches (same `project`/`area`/`today` rules), in the list's own or smart order, the first 500.
- [ ] Grouping reuses `views::items`. There's no second copy of the grouping or quadrant code (expose them `pub(crate)`, plus a way to build keys from task ids in `views::keys`). `layout` defaults to `list` and `group` to `none`. `kanban` with `group=none` groups by `status`. `eisenhower` ignores `group`, as in ADR 0016.
- [ ] Errors: `layout must be one of: list, kanban, eisenhower`, `group must be one of: ...`, and the existing list errors (`unknown list` 404, bad `today`/`sort`/id 400).
- [ ] `server/tests/list_layouts.rs` covers: `today` as kanban by priority; a project's `all` as kanban by section (key the section id, `none` last); `inbox` as eisenhower with four groups; each error above; `401` without auth.
- [ ] ADR `docs/decisions/0022-list-layouts.md` (≤ 15 lines) records why lists reuse the view grouping and that list layout choice is a client setting. `server/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Storing a layout per list on the server (the web keeps it locally, task 061). Calendar and timeline layouts (M8).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
