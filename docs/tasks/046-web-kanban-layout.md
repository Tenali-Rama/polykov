---
id: 046
title: Add the web Kanban layout with keyboard and drag moves
milestone: M3
status: done
deps: [045]
touches: [web/src/views/Kanban.svelte, web/src/views/Kanban.test.ts, web/src/views/move.ts, web/src/views/move.test.ts, web/src/views/ViewPage.svelte+, web/e2e/kanban.spec.ts, web/API.md]
spec: ["Tasks (GTD)", "Organization", "7. Clients & UI"]
---

## Goal
A view with `layout: kanban` shows one column per group. Moving a task card to another column changes the matching field.

## Acceptance criteria
- [ ] `ViewPage.svelte` only gains its `{:else if layout === 'kanban'}` branch and the import.
- [ ] Columns are box-bordered, in the server's group order, each headed by its label and count. Cards show the title, priority and dates.
- [ ] `views/move.ts` maps a move to an API call, with unit tests for each mapping:
  - `status` → complete or reopen
  - `priority` → set priority (`none` clears)
  - `project`/`section`/`area` → set that id (`none` clears)
  - `tag`/`type` → not movable; the UI says `read-only grouping`
- [ ] Keyboard: arrows move focus between cards and columns. `Shift+←/→` moves the focused card one column and keeps focus on it. `Enter` opens the task.
- [ ] Pointer: drag and drop a card onto a column. A failed move shows the server error and puts the card back.
- [ ] On `phone` the board scrolls horizontally inside its own box (columns ≥ 260 px, snap per column), and the page itself never scrolls horizontally.
- [ ] Vitest tests with stubbed `fetch` cover rendering, a keyboard move with the right `PATCH`/complete call, a failed move reverting, and a read-only grouping. `npm --prefix web run test` passes.
- [ ] `web/e2e/kanban.spec.ts`, against the real server:
  - keyboard only: a view grouped by priority; move a card from P3 to P2 with `Shift+←`; the server shows priority 2 after reload
  - a drag-and-drop move between status columns completes the task
  - on `phone`, `expectPhoneFit` passes, with cards ≥ 44 px tall
- [ ] `web/API.md` updated.

## Out of scope
- Manual card order within a column (spec has no manual order; sort comes from the view). Swipe gestures (M14).

## Notes for next
- `status` and `priority` boards always show every column (`open`/`done`, `P1`–`P4`), even empty ones, so a card can move there; the server lists only non-empty groups. Other groupings show only the server's groups.
- Moving to priority `none` sets 4 (the server's "no priority"; `priority` can't be `null`). A move to status `none` has no call.
- A section move only sets `section_id`; a section of another project fails with the server's `section is not in this project` and the card goes back.
- A move doesn't reload: the server's task replaces the card in its new column, so a card that no longer matches the query stays until the next reload.
- Mouse drag doesn't start from touch input, so the phone e2e drag is `test.fixme` until M14 swipe actions; phone users move cards by keyboard only for now.
