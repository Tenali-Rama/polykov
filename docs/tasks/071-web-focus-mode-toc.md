---
id: 071
title: Add note focus mode with typewriter scrolling and a floating table of contents
milestone: M4
status: todo
deps: [065]
touches: [web/src/notes/focus.ts, web/src/notes/focus.test.ts, web/src/notes/Toc.svelte, web/src/notes/Toc.test.ts, web/src/notes/NotePage.svelte+, web/src/Shell.svelte, web/src/Shell.test.ts, web/src/app.css, web/src/palette/commands.ts, web/src/palette/commands.test.ts, web/e2e/notes-focus.spec.ts, web/API.md]
spec: ["Notes", "7. Clients & UI"]
---

## Goal
A shortcut hides everything except the note text, centred, with optional typewriter scrolling and dimming of inactive paragraphs. A floating table of contents lets the owner jump between headings.

## Acceptance criteria
- [ ] Focus mode: `Ctrl+Shift+F` (default — owner may change) on a note page toggles it, and so does the palette command `toggle focus mode` (added to `commands.ts`; update the order in `commands.test.ts`).
  - When on, the Shell header, nav, prompt, the note's title bar, properties, panels and TOC are hidden. Shell marks its chrome `data-chrome` and `app.css` hides `[data-chrome]` under `body[data-focus]`.
  - The text column is centred (max 70ch).
  - `Esc` leaves focus mode only when no editor menu is open.
  - Leaving the note page turns it off.
- [ ] Options in focus mode, each toggled by a key and stored in `localStorage`, both off by default:
  - typewriter scrolling: the line with the cursor stays at the vertical centre
  - dimming: blocks other than the current one at reduced opacity, using a CSS variable
- [ ] TOC (`Toc.svelte`, one line in `NotePage` panels): the note's headings, indented by level, that update as you type. `Enter` or a click scrolls to the heading and puts the cursor there, and the current heading is marked `aria-current`. It floats at the right from 900 px up. Below that it sits behind a `[ toc ]` button (`aria-expanded`). With no headings it isn't shown.
- [ ] Unit tests: `focus.ts` (toggle, persisted options, cleanup on leave) and `Toc` (heading list, update, jump). `Shell.test.ts` asserts `data-chrome` on its header and nav.
- [ ] `web/e2e/notes-focus.spec.ts`: keyboard only, enter focus mode, check that nav and header aren't visible and the text is centred (left and right margins within 2 px of each other at 1280 px), leave with `Esc`, and jump to the third heading through the TOC. On `phone`, `expectPhoneFit` passes with focus mode on.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Focus mode outside notes. Rebindable shortcuts (M16).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
