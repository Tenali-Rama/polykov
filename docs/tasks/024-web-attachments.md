---
id: 024
title: Add the web attachments panel with upload, image preview, download and delete
milestone: M1
status: done
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
- `humanSize` lives in `lib/attachments.ts` and matches the CLI's `human_size` format (`512 B`, `5.0 KiB`).
- Uploads run one at a time in pick/drop order; the first failure stops the rest, and a drop during an upload is ignored.
- `.name` has `flex: 1 1 14ch; min-width: 14ch`, so the row metadata wraps under it at phone width (checked in headless Chromium at 360/412 px).
- Delete confirm cancels on Esc via `preventDefault` (Detail stays) and refocuses the row's `[ delete ]`.
- Thumbnails cover every `image/*`; the server only serves `inline` for non-SVG images, so an SVG thumbnail may not render. 023 adds its panel line beside ours (keep both on rebase).
