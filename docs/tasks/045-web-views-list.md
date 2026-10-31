---
id: 045
title: Add web saved views with the editor and the grouped list layout
milestone: M3
status: done
deps: [043]
touches: [web/src/views/ViewPage.svelte, web/src/views/ViewPage.test.ts, web/src/views/ViewEditor.svelte, web/src/views/ViewEditor.test.ts, web/src/views/ViewsIndex.svelte, web/src/views/ListLayout.svelte, web/src/views/route.ts, web/src/views/route.test.ts, web/src/lib/views.ts, web/src/lib/views.test.ts, web/src/Shell.svelte+, web/src/tasks/Nav.svelte+, web/src/palette/commands.ts+, web/e2e/views.spec.ts, web/API.md]
spec: ["Organization", "Tasks (GTD)", "7. Clients & UI"]
---

## Goal
The owner can create, edit, pin and open saved views in the web UI. `#/views` lists them, and `#/v/<id>` shows the results in a grouped list.

## Acceptance criteria
- [ ] `lib/views.ts` wraps `GET/POST /api/views`, `PATCH /api/views/{id}` and `GET /api/views/{id}/items` (sending the local `today`).
- [ ] `views/route.ts` parses `#/views` and `#/v/<id>`, with a unit test.
- [ ] Hub edits only add lines:
  - `Shell.svelte`: the route branch
  - `Nav.svelte`: one `views` link
  - `palette/commands.ts`: one "go to views" command, if 044 has landed (else skip it and say so in the PR)
- [ ] `ViewsIndex` lists views (pinned marked) and has a `new view` button.
- [ ] `ViewEditor` has a prompt-style `query` input and `layout`/`group`/`sort` selects. Save shows the server's `400` inline, marking the error offset `at` under the query, and keeps the input.
- [ ] `ViewPage` shows the title, a pin toggle, an `edit` toggle that reveals the editor, and the results. The layout is chosen in one `{#if}` chain. This task renders `ListLayout` for every layout, so 046/047 each add one `{:else if}` line.
- [ ] `ListLayout`:
  - one section per group, with a header showing the label and count
  - task rows reuse `tasks/TaskRow.svelte`; other objects show a type plus title link
  - shows `showing 500 of N` when truncated
  - reloads on `polykov:tasks-changed`
- [ ] Vitest tests with stubbed `fetch` cover the route parsing, editor validation errors, grouped rendering, the empty state and pinning. `npm --prefix web run test` passes.
- [ ] `web/e2e/views.spec.ts`, against the real server:
  - keyboard only: create a view `type:task priority:<=2 -status:done` grouped by priority; tasks seeded with `api()` appear in the right groups; completing one with the keyboard removes it
  - on `phone`, `expectPhoneFit` passes on `#/views`, on the editor, and on a view with 3 groups
- [ ] `web/API.md` updated.

## Out of scope
- Kanban (046) and Eisenhower (047) rendering. The favourites sidebar (048). Drag reordering of views.

## Notes for next
- 046/047: add one `{:else if view.properties.layout === '<layout>'}` branch above `{:else}` in `ViewPage.svelte`; each layout takes `{id}`, loads `viewItems(id)` itself and listens for `TASKS_CHANGED` (see `ListLayout`). ViewPage remounts the layout after an edit via `{#key revision}`.
- `ApiError` now carries `at` (the server's error offset); `ViewEditor` uses it for the `^` marker.
- View items carry no sub-task counts, so `ListLayout` passes `subtasks: {0,0}` to `TaskRow` (no `[open/total]` shown).
- `x` in a view completes the task and dispatches `TASKS_CHANGED`, so it disappears when the query excludes done tasks; clicking `[ ]` only strikes it through (as in `TaskList`).
- ViewsIndex tests live in `views/route.test.ts` (no own test file in `touches`).
