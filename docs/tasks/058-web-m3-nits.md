---
id: 058
title: Fix TaskRow priority wrapping, palette/capture overlay clash and the event-name copy
milestone: M4
status: todo
deps: []
touches: [web/src/tasks/TaskRow.svelte, web/src/palette/Palette.svelte, web/src/palette/Palette.test.ts, web/src/palette/commands.ts, web/src/palette/commands.test.ts, web/src/capture/CaptureBar.svelte, web/src/capture/CaptureBar.test.ts, web/src/objects/Detail.svelte, web/e2e/palette.spec.ts, web/API.md]
spec: ["7. Clients & UI", "Capture", "Organization"]
---

## Goal
Small M3 web review findings: the priority badge never breaks across lines, only one of palette or capture bar is open at a time, and `Detail` uses the shared event constant.

## Acceptance criteria
- [ ] `TaskRow`: `p1` to `p3` never wraps (`white-space: nowrap; flex-shrink: 0`). Checked in headless Chromium at 360 px with a 200-character title in a Kanban column and an Eisenhower quadrant: the badge's bounding box is a single line tall. Put the measurement in the PR.
- [ ] Overlays: opening the palette (`Ctrl+K`) while the capture bar is open closes the capture bar first, and the other way round (`Ctrl+Space`). A `>` typed in the capture bar still hands off to the palette with its text. Vitest tests cover both directions, and `web/e2e/palette.spec.ts` gains one keyboard-only check that at most one `dialog` is visible.
- [ ] `Detail.svelte` imports `OBJECTS_CHANGED` instead of repeating `'polykov:objects-changed'`. `grep -rn "polykov:objects-changed" web/src --include=*.svelte` matches only the definition.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- New palette commands. The layout loader refactor (task 059).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
