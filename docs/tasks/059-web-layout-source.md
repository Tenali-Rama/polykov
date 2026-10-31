---
id: 059
title: Extract a shared layout loader so list, Kanban and Eisenhower layouts take a source
milestone: M4
status: done
deps: []
touches: [web/src/views/source.ts, web/src/views/source.test.ts, web/src/views/ListLayout.svelte, web/src/views/Eisenhower.svelte, web/src/views/Eisenhower.test.ts, web/src/views/Kanban.svelte, web/src/views/Kanban.test.ts, web/src/views/ViewPage.svelte, web/src/views/ViewPage.test.ts, web/API.md]
spec: ["Tasks (GTD)", "Organization", "7. Clients & UI"]
---

## Goal
The three layout components stop loading a view themselves. Each takes a `source` that loads grouped items, so task 061 can feed them a GTD list, and `Eisenhower` loses its ~60 copied lines of `ListLayout` load/key logic.

## Acceptance criteria
- [x] `web/src/views/source.ts` exports `type Source = { group: GroupBy; load(): Promise<{ groups: Group[]; total: number; truncated: boolean }> }`, `viewSource(view)` (wraps `viewItems`), and the shared load/error/reload and row-key handling that `ListLayout` and `Eisenhower` both used. A unit test in `source.test.ts` covers it.
- [x] `ListLayout`, `Eisenhower` and `Kanban` take `source: Source` instead of `id`, and use the shared helper. `Eisenhower.svelte` has no copy of the load/key code: the PR states its line count before and after.
- [x] `ViewPage` passes `viewSource(view)`. Rendering, keys and moves don't change: every existing `views/*.test.ts` and `web/e2e/{views,kanban,eisenhower}.spec.ts` passes, with only the mount props changed in the unit tests.
- [x] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Layouts on GTD lists (061). The Kanban bug fixes (057). If 057 is merged first, keep its behaviour.

## Notes for next
- `Source = {group, load()}` lives in `web/src/views/source.ts`; a GTD list source (061) only needs `group` plus a `load()` returning `{groups, total, truncated}`.
- `Source.group` is the grouping the groups come in: `viewSource` maps a kanban view without one to `status` (`boardGroup`), so `Kanban` reads `source.group` directly.
- Layouts read `source` once (`untrack`); ViewPage still remounts them via `{#key revision}` after an edit.
- `source.ts` is plain TS (no runes): `loader` takes callbacks, `rowKey` returns the new selection; layouts keep their own `$state`.
