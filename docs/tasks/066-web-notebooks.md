---
id: 066
title: Add the web notebooks browser and route notes from nav, palette, search and favourites
milestone: M4
status: todo
deps: [065]
touches: [web/src/notes/NotesIndex.svelte, web/src/notes/NotesIndex.test.ts, web/src/notes/NotebookTree.svelte, web/src/notes/NotebookTree.test.ts, web/src/Shell.svelte+, web/src/Shell.test.ts, web/src/tasks/Nav.svelte, web/src/tasks/route.test.ts, web/e2e/lists.spec.ts, web/src/palette/commands.ts, web/src/palette/commands.test.ts, web/src/lib/search.ts, web/src/lib/search.test.ts, web/src/favourites/Favourites.svelte, web/src/favourites/Favourites.test.ts, web/e2e/favourites.spec.ts, web/e2e/notebooks.spec.ts, web/API.md]
spec: ["Notes", "3. Data Model", "7. Clients & UI"]
---

## Goal
The owner browses nested notebooks, creates notes and notebooks, moves a note between notebooks, and reaches notes from every place that already links to objects.

## Acceptance criteria
- [ ] `#/notes` and `#/nb/<id>`: a `NotebookTree` on the left (or above it on phones) with expandable nested notebooks, their note counts and `unfiled`, next to the notes of the selected notebook (`listNotes`, pages of 50).
  - `[ new note ]` creates a note in the current notebook and opens `#/n/<id>`.
  - `[ new notebook ]` creates one under the current notebook.
  - A notebook can be renamed, and moved by choosing a parent; a cycle shows the server error.
  - A note's notebook can be changed from its row.
  - The tree works with arrow keys (`↑/↓`, `→` expand, `←` collapse, `Enter` open), following the WAI tree pattern.
- [ ] These shared lists change, and their existing tests are updated in this PR:
  - Nav gains `notes` after `views`: `tasks/route.test.ts` link list and count, and `e2e/lists.spec.ts` names.
  - The palette gains `go to notes` after `go to views`, and `new note`: `commands.test.ts`.
  - `hitHref` sends `note` to `#/n/<id>` and `notebook` to `#/nb/<id>`: `search.test.ts`.
  - Favourites does the same for notes and notebooks: `Favourites.test.ts` and `e2e/favourites.spec.ts` (`note …` → `#/n/<id>`).
  - Shell gains the `#/notes`/`#/nb/<id>` branch: `Shell.test.ts`.
- [ ] Vitest tests cover the tree (nesting, keys, counts), create note and notebook, and moving a note.
- [ ] `web/e2e/notebooks.spec.ts`, against the real server: keyboard only, make nested notebooks, create a note inside the inner one, open it from the palette search, and move it to `unfiled`. On `phone`, `expectPhoneFit` passes for `#/notes` and `#/nb/<id>`.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Drag and drop in the tree. Editor features.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
