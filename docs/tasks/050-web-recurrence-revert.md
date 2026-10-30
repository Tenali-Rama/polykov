---
id: 050
title: Revert the recurrence editor to the saved rule after a rejected save
milestone: M3
status: done
deps: []
touches: [web/src/tasks/Recurrence.svelte, web/src/tasks/Recurrence.test.ts, web/src/tasks/TaskFields.svelte, web/src/tasks/TaskFields.test.ts, web/e2e/task-detail.spec.ts]
spec: ["Tasks (GTD)"]
---

## Goal
M2 review backlog. When the server rejects a recurrence change (e.g. interval `0`), `Recurrence.svelte` currently keeps showing the rejected value. After this task it shows the server error and returns every control to the saved rule.

## Acceptance criteria
- [ ] After a failed save, the `repeat`, `interval`, weekday, month day and `repeat from` controls all show the last saved rule, and the error is shown inline. A successful save still updates them.
- [ ] The fix keys the controls off the saved value, e.g. by re-rendering on failure. `onsave` reports failure to the editor; if that needs a change in how `TaskFields.svelte` calls it, keep the change minimal.
- [ ] New `web/src/tasks/Recurrence.test.ts` (stubbed `fetch`):
  - interval `0` → `400` → input shows the saved interval and the error
  - a valid change sticks
  - a `400` on a weekday toggle reverts the weekday
  - `npm --prefix web run test` passes
- [ ] `web/e2e/task-detail.spec.ts` gains one test: type interval `0`, `Tab`, and the input shows the saved interval again.

## Out of scope
- Client-side validation that duplicates the server's rules. Other field editors.

## Notes for next
- `Recurrence`'s `onsave` now returns `Promise<boolean>`; `TaskFields`' `run`/`patch` resolve `false` on failure (the error still shows inline).
- The revert resets only the changed control's DOM value (`orRevert`) instead of a `{#key}` re-render, so keyboard focus survives a Tab-triggered save.
- Weekday toggles need no revert: `aria-pressed` is derived from the saved `value`.
- `web/API.md` updated for the new `onsave` contract (outside `touches`).
