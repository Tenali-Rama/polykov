---
id: 070
title: Add wikilink autocomplete and rendering and the backlinks panel to notes
milestone: M4
status: todo
deps: [064, 065]
touches: [web/package.json, web/package-lock.json, web/src/notes/extensions/index.ts+, web/src/notes/extensions/wikilink.ts, web/src/notes/extensions/wikilink.test.ts, web/src/notes/Backlinks.svelte, web/src/notes/Backlinks.test.ts, web/src/notes/NotePage.svelte+, web/src/lib/notes.ts, web/src/lib/notes.test.ts, web/e2e/wikilinks.spec.ts, web/API.md]
spec: ["Notes", "3. Data Model"]
---

## Goal
Typing `[[` in a note offers matching objects and inserts a link. Links render as clickable, and each note shows what links to it.

## Acceptance criteria
- [ ] Typing `[[` opens a listbox of up to 8 `/api/search` hits (`<type> <title>`) that updates as you type. `↑/↓` and `Enter` insert `[[Title]]`, and `Esc` closes it without leaving the editor (handled: `preventDefault` + `stopPropagation`). The list is ARIA-wired (`combobox`/`listbox`, `aria-activedescendant`).
- [ ] A wikilink node serialises to `[[Title]]` or `[[Title|label]]` and shows the label or title. After load and after each save it's marked resolved or unresolved from `GET /api/notes/{id}/wikilinks`: resolved links go to `hitHref(target)` on click, or `Ctrl+Enter` with the cursor in them; unresolved ones are dim with the title `no object titled "…"`.
- [ ] `Backlinks.svelte`, added to `NotePage`'s panels with one line, lists `GET /api/objects/{id}/links` backlinks (`<type> <title>`, linked), marked `[[ ]]` when the origin is `wikilink`, or shows `no backlinks`. It refreshes after a save.
- [ ] Unit tests: round-trip for both wikilink forms and for text containing `[[` inside code; autocomplete keys with stubbed `fetch`; and `Backlinks` rendering.
- [ ] `web/e2e/wikilinks.spec.ts`: keyboard only, in note A type `[[`, pick note B, and wait for `saved`. Open B, and A is in its backlinks. Follow the link back with `Ctrl+Enter`. On `phone`, `expectPhoneFit` passes, with the listbox inside the viewport.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Hover previews (073). Rename propagation.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
