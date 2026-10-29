---
id: 023
title: Add web links/backlinks and comments panels to the object detail
milestone: M1
status: todo
deps: [015, 021]
touches: [web/src/objects/Links.svelte, web/src/objects/Links.test.ts, web/src/objects/Comments.svelte, web/src/objects/Comments.test.ts, web/src/lib/links.ts, web/src/lib/comments.ts, web/src/lib/links.test.ts, web/src/lib/comments.test.ts, web/src/objects/Detail.svelte+, web/API.md]
spec: ["7. Clients & UI", "3. Data Model", "Tasks (GTD)"]
---

## Goal
The object detail shows its links and backlinks (clickable) and a comment thread that can be added to, edited and deleted.

## Acceptance criteria
- [ ] `Links.svelte` (props: `id`) shows `links` and `backlinks` sections. Each entry opens `#/o/<id>`.
  - `[ link ]` opens a prompt that takes an object id, or a title searched through `listObjects` (first 10 matches by title, filtered on the client) (default — owner may change), and sends `PUT`
  - each outgoing link has `[ unlink ]`
- [ ] `Comments.svelte` (props: `id`) lists comments oldest first with their created time, and has a textarea prompt where Ctrl+Enter submits. Each comment has `[ edit ]` (inline, Ctrl+Enter saves, Esc cancels) and `[ delete ]`. A server `400` text is shown inline.
- [ ] Each panel is added to `Detail.svelte` with one line in the slot area.
- [ ] Vitest tests with mocked `fetch` cover rendering, link add by id and by title, unlink, backlink navigation, and comment add/edit/delete/400. `npm --prefix web run test` passes.
- [ ] `web/API.md` updated.

## Out of scope
- Wikilinks in note text (M4). Markdown rendering of comments.

## Notes for next
<!-- builder fills in -->
