---
id: 069
title: Embed live tasks and saved views in notes
milestone: M4
status: todo
deps: [059, 064, 065]
touches: [web/src/notes/extensions/index.ts+, web/src/notes/extensions/embed.ts, web/src/notes/Embed.svelte, web/src/notes/Embed.test.ts, web/src/lib/notes.ts, web/src/lib/notes.test.ts, web/e2e/notes-embeds.spec.ts, web/API.md, docs/decisions/0024-note-embeds.md]
spec: ["Notes", "Organization"]
---

## Goal
`![[Title]]` on its own line in a note shows that task or saved view live inside the note.

## Acceptance criteria
- [ ] The embed node serialises to exactly `![[Title]]` and resolves through `GET /api/resolve?title=` (task 064). `lib/notes.ts` gains `resolveTitle`, with a test.
- [ ] A task embed renders a `TaskRow`: complete and open work, and the row updates when completed. A view embed renders the view's layout through `viewSource` (task 059), limited to its first 20 items with `[ open view ]` to `#/v/<id>`. Any other type renders as a link (`hitHref`). An unresolved title shows `not found: Title` in dim text and keeps the markdown.
- [ ] The embed is one block: arrow keys move over it, and `Backspace` on an empty line after it removes it. Its contents don't become editor text.
- [ ] `Embed.test.ts` (stubbed `fetch`) covers task, view, other type and unresolved embeds. There's a round-trip fixture for `![[Title]]`.
- [ ] `web/e2e/notes-embeds.spec.ts`: seed a task and a list view, then a note embedding both. Complete the task from inside the note, and the server shows it done. On `phone`, `expectPhoneFit` passes.
- [ ] ADR `docs/decisions/0024-note-embeds.md` (≤ 15 lines). `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Embedding notes (transclusion) or images by wikilink. Inserting an embed via autocomplete (070 does `[[`; `![[` reuses it if 070 has landed, otherwise typing the markdown is enough).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
