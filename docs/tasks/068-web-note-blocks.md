---
id: 068
title: Add toggles, callouts and block reordering by drag and keyboard to the note editor
milestone: M4
status: todo
deps: [065]
touches: [web/src/notes/extensions/index.ts+, web/src/notes/extensions/toggle.ts, web/src/notes/extensions/callout.ts, web/src/notes/extensions/reorder.ts, web/src/notes/extensions/blocks.test.ts, web/e2e/notes-blocks.spec.ts, web/API.md, docs/decisions/0025-note-blocks.md]
spec: ["Notes", "7. Clients & UI"]
---

## Goal
Notion-style blocks: collapsible toggles, callouts, and moving any top-level block up or down by drag or keyboard.

## Acceptance criteria
- [ ] Toggle: stored as `<details><summary>Title</summary>\n\nbody\n\n</details>`. It opens and closes with a click or with `Enter` on its summary. The open/closed state isn't saved (default — owner may change).
- [ ] Callout: GitHub alert syntax `> [!NOTE]` / `[!TIP]` / `[!WARNING]`, rendered box-bordered with its kind as a text label (no icons).
- [ ] Reorder: a drag handle appears on hover or focus at the left of each top-level block, and dropping it moves the block. `Alt+↑`/`Alt+↓` moves the block at the cursor. The handle is ≥ 44 px on touch, and the move is announced to screen readers (`aria-live`).
- [ ] Each is a file in `extensions/`, registered with one appended line in `index.ts`. Typing `/toggle` or `/callout` isn't required; a toolbar-less insertion via markdown input rules is (`>[!NOTE] `, `+++ ` for a toggle; default — owner may change).
- [ ] `blocks.test.ts`: round-trip fixtures for a toggle (nested content included) and each callout kind, plus a unit test that `Alt+↓` swaps two paragraphs in the serialised markdown.
- [ ] `web/e2e/notes-blocks.spec.ts`: keyboard only, make a callout and a toggle, then reorder with `Alt+↑`, reload, and the order is kept. A pointer drag moves a block. On `phone`, `expectPhoneFit` passes.
- [ ] ADR `docs/decisions/0025-note-blocks.md` (≤ 15 lines): the markdown each block becomes. `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Embeds (069). A slash-command menu.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
