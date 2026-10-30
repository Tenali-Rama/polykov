---
id: 036
title: Add the web capture bar with Ctrl+Space and live token highlighting
milestone: M2
status: done
deps: [031, 033, 037]
touches: [web/src/capture/CaptureBar.svelte, web/src/capture/CaptureBar.test.ts, web/src/capture/highlight.ts, web/src/capture/highlight.test.ts, web/src/lib/smartadd.ts, web/src/lib/smartadd.test.ts, web/src/Shell.svelte+, web/e2e/capture.spec.ts, web/API.md]
spec: ["Capture", "7. Clients & UI"]
---

## Goal
From any screen, `Ctrl+Space` opens a one-line capture bar that highlights parsed tokens as you type. `Enter` saves through smart add and leaves you where you were.

## Acceptance criteria
- [ ] `Shell.svelte` only gains lines: the import and `<CaptureBar />`. The bar listens for `Ctrl+Space` on `window`, including while focus is in another input, and opens a top overlay with a prompt-style input (`> ▌`).
- [ ] `lib/smartadd.ts` provides `parse(text)` → `POST /api/parse` and `add(text)` → `POST /api/tasks/add`. Both send the local `today`.
- [ ] While typing, `parse` runs on each input, cancelling the previous request (`AbortController`). Only the latest response is applied.
  - A mirror layer behind the transparent input colours each token by kind, from the CSS variables in `app.css`. `highlight.ts` turns text plus UTF-16 spans into segments, with a unit test including an emoji.
  - Below the input, chips summarize the parse, e.g. `tomorrow 17:00 · 30m · p2 · #family · every sunday`. An unresolved `+project`/`@area` shows in `--err`.
  - Each chip has `[ x ]`, which wraps that token's text in quotes so it stays plain title text. Editing the text edits the tokens.
- [ ] `Enter` saves. The bar closes, focus returns to the element focused before it opened, and a status note `added to inbox: <title>` (or `added: <title> · <list>`) shows for 4 s. The window event `polykov:tasks-changed` is dispatched.
  - On a server `400` the bar stays open with the error inline and the text kept.
  - `Esc` closes without saving and restores focus.
  - An empty input does nothing.
- [ ] Vitest tests with stubbed `fetch` cover open/close, focus restore, highlight segments, chip removal quoting, stale responses ignored, save, 400 and empty. `npm --prefix web run test` passes.
- [ ] `web/e2e/capture.spec.ts`, against the real server:
  - From the object list and from a task detail page, `Ctrl+Space`, typing `call mom tmrw 5pm 30m p2 #family every sunday` and `Enter` creates a task that appears in Upcoming. The page URL is unchanged.
  - On `phone` (360 px), `expectPhoneFit` passes with the bar open and all chips shown. A 200-char input doesn't scroll the page horizontally.
  - The keyboard-only flow works: open, type, `Tab` to a chip's `[ x ]`, `Enter`, `Esc`.
- [ ] `web/API.md` updated.

## Out of scope
- `>` switching to the command palette (M3). The system-wide hotkey (M13). Voice input (M11).

## Notes for next
- `lib/api.ts` (outside `touches`) gained an optional `signal` on `send`/`post`, so `parse` can be aborted while still going through `api.ts`.
- The note's `<list>` is computed client-side by `smartadd.landing` from the server's default list rules (inbox/today/upcoming/anytime); keep it in sync if those rules become configurable.
- Chips are one per token (date and time are separate chips: `tomorrow · 17:00`); a repeated field shows its raw text, since the parser keeps the first value.
- Highlighting applies only when the latest parse is for the current text (no mis-coloured offsets mid-typing); `[ x ]` is ignored while the parse is stale.
- The bar renders the 4 s note itself, above the Shell body; `>` to switch to the palette (M3) should hook into `CaptureBar`'s `reparse`.
