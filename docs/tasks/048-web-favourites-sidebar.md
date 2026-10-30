---
id: 048
title: Add the favourites section to the web sidebar
milestone: M3
status: todo
deps: []
touches: [web/src/favourites/Favourites.svelte, web/src/favourites/Favourites.test.ts, web/src/tasks/Nav.svelte+, web/src/objects/Detail.svelte+, web/e2e/favourites.spec.ts, web/API.md]
spec: ["Organization", "7. Clients & UI"]
---

## Goal
Every pinned object, saved views included, appears under `favourites` in the nav, so the owner can reach it in one click or a few keys.

## Acceptance criteria
- [ ] `Nav.svelte` only gains lines: the import and `<Favourites />` placed before the list links. Inside the phone `[ lists ]` menu, favourites appear first.
- [ ] `Favourites` loads `GET /api/objects?pinned=true&limit=100` through `lib/objects.ts`. Each entry is a link with the object's type and title:
  - `view` objects link to `#/v/<id>` (route from ADR 0016)
  - projects link to `#/p/<id>`
  - areas link to `#/a/<id>`
  - everything else links to the object detail
  - The current route's entry is marked `aria-current="page"`.
- [ ] The list reloads on `hashchange` and on the window events `polykov:tasks-changed` and `polykov:objects-changed`.
  - This task adds a dispatch of `polykov:objects-changed` after pin/unpin in the object detail, as one added line in `objects/Detail.svelte`.
  - With no pinned objects, the section is hidden.
- [ ] Each link is ≥ 44 px tall and is keyboard-reachable in order, inside the existing Nav arrow-key handling.
- [ ] Vitest tests with stubbed `fetch` cover the href per type, the hidden empty state, reload on event and `aria-current`. `npm --prefix web run test` passes.
- [ ] `web/e2e/favourites.spec.ts`, against the real server:
  - pin a note in its detail with the keyboard; it appears under favourites without a reload
  - unpin it; it disappears
  - a pinned `view` object created via `api()` links to `#/v/<id>`
  - on `phone`, `expectPhoneFit` passes with the menu open and 5 favourites
- [ ] `web/API.md` updated.

## Out of scope
- Manual reordering of favourites (sidebar ordering is M16). Collapsing projects/areas in the nav.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
