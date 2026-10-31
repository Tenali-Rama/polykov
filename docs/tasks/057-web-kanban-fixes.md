---
id: 057
title: Fix the Kanban priority-none display, recurring done moves and tag-board arrow keys
milestone: M4
status: todo
deps: []
touches: [web/src/views/Kanban.svelte, web/src/views/Kanban.test.ts, web/src/views/move.ts, web/src/views/move.test.ts, web/e2e/kanban.spec.ts, web/API.md]
spec: ["Tasks (GTD)", "7. Clients & UI"]
---

## Goal
Three Kanban bugs from the M3 review are fixed: priority boards agree with the server about "no priority", completing a recurring card shows its next copy, and arrow keys stay in the column the focus is in.

## Acceptance criteria
- [ ] Priority 4 is the server's "no priority". A priority board shows exactly four columns, `P1`, `P2`, `P3` and `P4 · none` (default — owner may change), and never a separate `none` column. A card moved to `P4 · none` is still in that column after a reload. Unit tests in `move.test.ts` cover `columns('priority', …)` and `planMove('priority', 4)`.
- [ ] Moving a recurring task to `done` reloads the board after the move succeeds, so the next copy the server created shows in `open`. There's a Vitest test with stubbed `fetch` where `complete` returns `next`, and the board then shows it.
- [ ] Arrow keys go by the (column, row) of the focused element, not by the task id. On a `tag` board with a task in several columns, `↓` from that task's card in column 2 stays in column 2. There's a Vitest test for this.
- [ ] `web/e2e/kanban.spec.ts` adds a keyboard-only test: move a card to `P4 · none`, reload, and it's still there. The existing tests still pass.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Changing the server's groups. The shared layout loader (task 059).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
