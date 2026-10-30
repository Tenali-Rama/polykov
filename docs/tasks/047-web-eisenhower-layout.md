---
id: 047
title: Add the web Eisenhower matrix layout
milestone: M3
status: todo
deps: [045]
touches: [web/src/views/Eisenhower.svelte, web/src/views/Eisenhower.test.ts, web/src/views/ViewPage.svelte+, web/e2e/eisenhower.spec.ts, web/API.md]
spec: ["Tasks (GTD)", "7. Clients & UI"]
---

## Goal
A view with `layout: eisenhower` shows its tasks in the four quadrants the server computes: do, schedule, delegate and drop.

## Acceptance criteria
- [ ] `ViewPage.svelte` only gains its `{:else if layout === 'eisenhower'}` branch and the import.
- [ ] The matrix is 2×2 on wide screens and stacks to one column below 600 px. Each quadrant is box-bordered, labelled with the server's label and count, and lists `TaskRow`s.
- [ ] An empty quadrant shows `empty` and keeps its size.
- [ ] Quadrants are read-only: moving a task between quadrants is out of scope (default — owner may change). Opening and completing a task work as in the list layout.
- [ ] Keyboard: `Tab` order goes quadrant by quadrant (urgent + important first), and each quadrant is a labelled region.
- [ ] Vitest tests with stubbed `fetch` cover the four quadrants rendered in order, an empty quadrant and completing a task. `npm --prefix web run test` passes.
- [ ] `web/e2e/eisenhower.spec.ts`, against the real server: seed one task for each quadrant with `api()` (priority and deadline relative to today); each appears in its quadrant. On `phone`, `expectPhoneFit` passes with the stacked layout.
- [ ] `web/API.md` updated.

## Out of scope
- Drag between quadrants. Calendar and timeline layouts (M8).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
