---
id: 072
title: Paste and drop images into notes as attachments
milestone: M4
status: todo
deps: [065]
touches: [web/src/notes/extensions/index.ts+, web/src/notes/extensions/image.ts, web/src/notes/extensions/image.test.ts, web/e2e/notes-images.spec.ts, web/API.md]
spec: ["Notes"]
---

## Goal
Pasting or dropping an image into a note uploads it as an attachment of that note and shows it inline. The markdown keeps a plain image link.

## Acceptance criteria
- [ ] Pasting or dropping image files (`image/*`, several at once) uploads each through the existing attachments API (`lib/attachments.ts`) to the note. While it uploads, the editor shows a placeholder `uploading <name>…`. Then it inserts `![<name>](/api/attachments/<aid>?inline=1)`.
- [ ] A failed upload (e.g. too large) removes the placeholder and shows `error: <server message>` near the editor. Non-image files are ignored by this extension.
- [ ] Images render at most the column width, with `alt` from the name. The markdown round-trips unchanged, and external image URLs also render.
- [ ] Deleting the image from the text doesn't delete the attachment (default — owner may change). It stays in the note's attachments panel.
- [ ] `image.test.ts` (stubbed `fetch`) covers paste, drop of two files, a failure and the round-trip.
- [ ] `web/e2e/notes-images.spec.ts`: drop a PNG with `DataTransfer` in the page. The image shows, and after reload `GET /api/objects/{id}/attachments` lists it and the body has the link. On `phone`, `expectPhoneFit` passes, with the image no wider than the viewport.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Image resizing or captions. Non-image attachments in the body.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
