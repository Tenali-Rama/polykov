---
id: 044
title: Add the web command palette with Ctrl+K, global search and > from the capture bar
milestone: M3
status: todo
deps: [039]
touches: [web/src/palette/Palette.svelte, web/src/palette/Palette.test.ts, web/src/palette/commands.ts, web/src/palette/commands.test.ts, web/src/lib/search.ts, web/src/lib/search.test.ts, web/src/capture/CaptureBar.svelte, web/src/capture/CaptureBar.test.ts, web/src/Shell.svelte+, web/e2e/palette.spec.ts, web/API.md]
spec: ["Organization", "Capture", "7. Clients & UI"]
---

## Goal
`Ctrl+K` from anywhere opens a command palette. It searches every object, archived and completed ones included, and runs navigation commands. Typing `>` as the first character in the capture bar switches to the palette.

## Acceptance criteria
- [ ] `Shell.svelte` only gains lines: the import and `<Palette />`. The palette listens for `Ctrl+K` on `window` (capture phase, also while focus is in an input) and opens a top overlay with a prompt-style input. The browser default is prevented.
- [ ] `lib/search.ts` provides `search(q, {signal})` → `GET /api/search`. Each keystroke aborts the previous request, and only the latest response is applied.
- [ ] Commands live in `palette/commands.ts`:
  - go to each of Inbox, Today, Upcoming, Someday, Anytime, objects and trash
  - new task (opens the capture bar)
  - Commands are filtered by a case-insensitive subsequence match on the label, with a unit test.
  - An empty input shows all commands.
  - A non-empty input shows matching commands first, then search results: type, title, `[archived]`/`[done]` markers and the snippet, with the `[ ]` marks shown highlighted.
- [ ] Arrow keys move the selection, and `Enter` runs it. A search result navigates to the object (`#/v/<id>` for a `view`, the object detail otherwise). `Esc` closes and restores focus to the element that had it. `Tab` stays inside the overlay.
- [ ] In `CaptureBar.svelte`, typing `>` into an empty capture input closes the bar and opens the palette, carrying any text after `>`. This happens through a window event `polykov:palette-open` with `{detail:{text}}`. The palette dispatches `polykov:capture-open`, and the capture bar listens for it to open, which is how "new task" works.
- [ ] Vitest tests with stubbed `fetch` cover:
  - open on `Ctrl+K`, close on `Esc` with focus restored
  - command filtering
  - stale search responses ignored
  - `Enter` on a result navigates
  - `>` handoff from the capture bar
  - "new task" opens the capture bar
  - `npm --prefix web run test` passes
- [ ] `web/e2e/palette.spec.ts`, against the real server:
  - create an archived note and a done task through `api()`; `Ctrl+K`, type a misspelled word from each title, both show with markers, and `Enter` opens one
  - `Ctrl+Space`, `>`, `today`, `Enter` lands on `#/today`
  - on `phone` (360 px), `expectPhoneFit` passes with 10 results open, and every row is ≥ 44 px tall
- [ ] `web/API.md` updated.

## Out of scope
- Wiring the bottom `command` input to the palette (question for owner). Rebindable shortcuts (M16). Views in the command list (045 may add one line to `commands.ts`).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
