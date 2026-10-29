---
id: 022
title: Add web object editing, pin/archive/delete and the trash view
milestone: M1
status: done
deps: [017, 021]
touches: [web/src/objects/Detail.svelte, web/src/objects/Detail.test.ts, web/src/objects/TagsEditor.svelte, web/src/objects/PropsEditor.svelte, web/src/objects/Trash.svelte, web/src/objects/Trash.test.ts, web/src/objects/List.svelte, web/src/lib/trash.ts, web/src/lib/trash.test.ts, web/src/Shell.svelte, web/API.md]
spec: ["7. Clients & UI", "3. Data Model", "Organization"]
---

## Goal
From the detail page the owner can edit an object, pin, archive or delete it, and can restore or purge deleted objects from a trash view.

## Acceptance criteria
- [ ] Detail editing:
  - the title is edited in place (Enter saves, Esc cancels)
  - tags are added and removed through `TagsEditor`, with the server's normalized tags shown after save
  - properties are edited as `key: value` lines in `PropsEditor`; values parse as number/bool/string as in the CLI (018), and a parse error is shown inline without sending
  - each save is one `PATCH`, and a server `400` text is shown inline
- [ ] `[ pin ]`/`[ unpin ]`, `[ archive ]`/`[ unarchive ]` and `[ delete ]` buttons on the detail page. Delete returns to the list with the note `moved to trash`.
- [ ] The list has a `trash` link to `#/trash`. `Trash.svelte` lists deleted objects with their deleted and purge dates, `[ restore ]` and `[ delete forever ]` (with a confirm step), and `[ empty trash ]` (with a confirm step). It is built on `web/src/lib/trash.ts`.
- [ ] Vitest tests with mocked `fetch` cover title/tags/properties saves (requests checked), the property parse error, pin/archive toggles, delete, and trash restore/purge/empty including the confirm steps. `npm --prefix web run test` passes.
- [ ] `web/API.md` updated.

## Out of scope
- Links, comments (023) and attachments (024).

## Notes for next
- `route.ts` (trash route) and `Shell.test.ts` were edited outside `touches`: the `#/trash` route lives in `parseHash`, and the Shell owns the `moved to trash` note.
- `PropsEditor` keeps a line's old value when its text is unchanged, so arrays, `null` and numeric-looking strings survive an edit; new or changed values parse like CLI `--prop`.
- Detail's window `Esc` skips events with `defaultPrevented`; editors (title, tags, properties) `preventDefault` the Esc they use to cancel. 023/024 panels should follow this.
- The `moved to trash` note sits in Shell above the list and clears once the route leaves the list.
- Trash confirm steps focus `[ no ]`; Esc cancels a pending confirm before it leaves the view.
