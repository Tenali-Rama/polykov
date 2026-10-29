---
id: 024
title: Add the web attachments panel with upload, image preview, download and delete
milestone: M1
status: todo
deps: [016, 021]
touches: [web/src/objects/Attachments.svelte, web/src/objects/Attachments.test.ts, web/src/lib/attachments.ts, web/src/lib/attachments.test.ts, web/src/objects/Detail.svelte+, web/API.md]
spec: ["7. Clients & UI", "3. Data Model", "Tasks (GTD)"]
---

## Goal
The object detail lists its attachments, and the owner can upload, preview images, download and delete them.

## Acceptance criteria
- [ ] `web/src/lib/attachments.ts` has `listAttachments(id)`, `uploadAttachment(id, file)` (raw body via `send`, `?name=` encoded), `deleteAttachment(aid)` and `downloadUrl(aid, inline?)`, with tests.
- [ ] `Attachments.svelte` (props: `id`):
  - rows show `<name>  <size human>  <mime>`, with `[ download ]` (link to `/api/attachments/<aid>`) and `[ delete ]` (with a confirm step)
  - `image/*` rows show a thumbnail from `?inline=1`
  - `[ attach ]` opens a keyboard-reachable file picker; dropping files on the panel also uploads, one request per file, with a progress line `uploading <name>…`
  - a `413` shows `file too large (max 25 MiB)`, and other errors show the server text
- [ ] The panel is added to `Detail.svelte` with one line in the slot area.
- [ ] Vitest tests with mocked `fetch` cover listing, upload (request body is the file, name encoded), drop, 413 text, delete with confirm, and the image thumbnail URL. `npm --prefix web run test` passes.
- [ ] `web/API.md` updated.

## Out of scope
- Paste/drop images into the note editor (M4). Voice input (M11).

## Notes for next
<!-- builder fills in -->
