---
id: 047
title: Add the web Eisenhower matrix layout
milestone: M3
status: done
deps: [045]
touches: [web/src/views/Eisenhower.svelte, web/src/views/Eisenhower.test.ts, web/src/views/ViewPage.svelte+, web/e2e/eisenhower.spec.ts, web/API.md]
spec: ["Tasks (GTD)", "7. Clients & UI"]
---

## Goal
A view with `layout: eisenhower` shows its tasks in the four quadrants the server computes: do, schedule, delegate and drop.

## Acceptance criteria
- [x] `ViewPage.svelte` only gains its `{:else if layout === 'eisenhower'}` branch and the import.
- [x] The matrix is 2×2 on wide screens and stacks to one column below 600 px. Each quadrant is box-bordered, labelled with the server's label and count, and lists `TaskRow`s.
- [x] An empty quadrant shows `empty` and keeps its size.
- [x] Quadrants are read-only: moving a task between quadrants is out of scope (default — owner may change). Opening and completing a task work as in the list layout.
- [x] Keyboard: `Tab` order goes quadrant by quadrant (urgent + important first), and each quadrant is a labelled region.
- [x] Vitest tests with stubbed `fetch` cover the four quadrants rendered in order, an empty quadrant and completing a task. `npm --prefix web run test` passes.
- [x] `web/e2e/eisenhower.spec.ts`, against the real server: seed one task for each quadrant with `api()` (priority and deadline relative to today); each appears in its quadrant. On `phone`, `expectPhoneFit` passes with the stacked layout.
- [x] `web/API.md` updated.

## Out of scope
- Drag between quadrants. Calendar and timeline layouts (M8).

## Notes for next
- `Eisenhower.svelte` repeats `ListLayout`'s load/keys logic (~60 lines): `ListLayout` was outside `touches`. Kanban (046) or a later task could extract a shared `layout items` helper.
- Quadrants are `<section aria-label=<server label>>` regions; e2e must use `exact: true`, since `important` and `urgent` are substrings of `urgent & important`.
- Stacks below 600 px (`max-width: 599.98px`, as `Nav`); each quadrant is at least 8rem tall, and quadrants in a row share its height.
- Non-task items (notes etc.) land in `neither` on the server; they render as `<type>` + link, as in the list layout.
- The e2e seeds through a local `POST /api/tasks` helper (as `views.spec.ts`): the shared `api()` helper has no task call.
