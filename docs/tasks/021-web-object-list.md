---
id: 021
title: Add the web object list with tag folders, filters, create and a read-only detail
milestone: M1
status: todo
deps: [014]
touches: [web/src/lib/api.ts, web/src/lib/api.test.ts, web/src/lib/objects.ts, web/src/lib/objects.test.ts, web/src/objects/, web/src/Shell.svelte, web/src/Shell.test.ts, web/API.md]
spec: ["7. Clients & UI", "Organization", "3. Data Model"]
---

## Goal
After logging in, the shell body shows the owner's objects with tag-folder and filter navigation, a way to create an object, and a read-only detail page. Later tasks plug panels into that page.

## Acceptance criteria
- [ ] `api.ts` gains a generic `send(method, path, body?)` for GET/POST/PUT/PATCH/DELETE. A `Blob`/`File` body is sent raw with its type; anything else is sent as JSON. Errors use the existing `ApiError` rules. Covered in `api.test.ts`.
- [ ] `web/src/lib/objects.ts` has `listObjects(filter)`, `getObject(id)`, `createObject(input)`, `updateObject(id, patch)`, `deleteObject(id)`, `listTags()` and the `Obj`/`Filter` types, all built on `send`, with tests in `objects.test.ts`.
- [ ] `Shell` routes on the hash: `#/` shows the list, `#/o/<id>` shows the detail, and an unknown hash shows the list. Browser back/forward works.
- [ ] List view (`web/src/objects/List.svelte`):
  - a tag-folder sidebar from `/api/tags`, indented by depth, with counts
  - filters for type, pinned only, and archived (`false`/`true`/`any`)
  - rows show `*` for pinned, then title, type and `#tags`
  - `j`/`k` move between rows and `Enter` opens one
  - paging is a `[ more ]` button
  - empty state `no objects`
- [ ] Create form above the list (type select defaulting to `note`, title prompt input, Enter submits). The new object appears at the top, and a server `400` text is shown inline.
- [ ] Detail view (`web/src/objects/Detail.svelte`, read-only here) shows title, type, id, tags, pinned/archived, properties as `key: value` lines, and created/updated. It ends with a clearly marked slot area where 023/024 add one line each. `Esc` returns to the list. A `404` shows `object not found`.
- [ ] Vitest tests with mocked `fetch` cover list rendering, each filter changing the request query, tag-folder click, create (success and 400), `j`/`k`/`Enter`, and detail rendering and 404. `npm --prefix web run test` passes.
- [ ] `web/API.md` documents the new modules and the hash routes.

## Out of scope
- Editing, delete and the trash view (022), links/comments (023), attachments (024).
- The command palette and capture bar (M2/M3).

## Notes for next
<!-- builder fills in -->
