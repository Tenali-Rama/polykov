---
id: 061
title: Add the layout switcher to GTD lists and the project view
milestone: M4
status: todo
deps: [059, 060]
touches: [web/src/tasks/LayoutBar.svelte, web/src/tasks/LayoutBar.test.ts, web/src/tasks/TaskList.svelte, web/src/tasks/TaskList.test.ts, web/src/tasks/ProjectView.svelte, web/src/tasks/ProjectView.test.ts, web/src/lib/tasks.ts, web/src/lib/tasks.test.ts, web/e2e/list-layouts.spec.ts, web/API.md]
spec: ["Tasks (GTD)", "7. Clients & UI"]
---

## Goal
Every GTD list (Inbox, Today, Upcoming, Someday, Anytime) and the project view can be shown as list, Kanban or Eisenhower, like a saved view.

## Acceptance criteria
- [ ] `lib/tasks.ts` gains `listItems(name, {layout, group, project?, area?, sort?})` for `GET /api/lists/{name}/items` (sends the local `today`), with a unit test, and a `listSource(...)` that returns a `Source` (task 059).
- [ ] `LayoutBar.svelte`: `layout` select (`list|kanban|eisenhower`), plus a `group` select while `kanban` is chosen (`status|priority|project|section|area|tag`). Each list and each project stores its choice in `localStorage` under `polykov:layout:<list>` or `polykov:layout:p/<id>` (default `list` — owner may change). Both selects are labelled and ≥ 44 px tall.
- [ ] `TaskList` and `ProjectView` render the bar. `list` keeps today's rendering: existing tests pass unchanged, and `ProjectView.svelte` stays ≤ 400 lines. `kanban`/`eisenhower` render the 059 components with `listSource`. Kanban moves work as in views, and a moved task leaves or stays in the list per the server.
- [ ] Vitest tests with stubbed `fetch` cover switching layout, the persisted choice after re-mount, and the `GET` sent.
- [ ] `web/e2e/list-layouts.spec.ts`, against the real server: keyboard only, switch Today to Kanban by priority, move a card with `Shift+→`, reload, and the board and move persist. A project as Eisenhower shows its tasks in quadrants. On `phone`, `expectPhoneFit` passes for both.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Server-side storage of the choice. Sub-task counts on cards.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
