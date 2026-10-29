---
id: 034
title: Add web GTD lists, project view with sections, and task rows
milestone: M2
status: todo
deps: [026, 027, 028, 033, 037]
touches: [web/src/tasks/Nav.svelte, web/src/tasks/TaskList.svelte, web/src/tasks/TaskRow.svelte, web/src/tasks/ProjectView.svelte, web/src/tasks/route.ts, web/src/tasks/TaskList.test.ts, web/src/tasks/ProjectView.test.ts, web/src/tasks/route.test.ts, web/src/lib/tasks.ts, web/src/lib/tasks.test.ts, web/src/lib/projects.ts, web/src/lib/projects.test.ts, web/src/Shell.svelte+, web/e2e/lists.spec.ts, web/API.md]
spec: ["Tasks (GTD)", "7. Clients & UI"]
---

## Goal
In the web UI, the owner can open Inbox, Today, Upcoming, Someday, Anytime, any project and any area, and tick tasks off there. Projects show their tasks grouped by section, and sections can be managed.

## Acceptance criteria
- [ ] Task routes live in `web/src/tasks/route.ts`: `#/inbox`, `#/today`, `#/upcoming`, `#/someday`, `#/anytime`, `#/p/<id>` (project) and `#/a/<id>` (area). `Shell.svelte` only gains lines: imports, `<Nav />`, and a route branch checked before the existing ones. `#/` stays the object list (default — owner may change).
- [ ] `lib/tasks.ts` wraps `/api/lists/*`, subtasks, complete, reopen, and `GET`/`PATCH`/`POST /api/tasks`. It always sends the browser's local `today`. `lib/projects.ts` wraps 028's routes.
- [ ] `Nav.svelte` shows the five lists, then `objects` and `trash`, then active projects and areas. Each is a link, and the current one is marked with `aria-current`.
  - Below 600 px wide the nav collapses into one `[ lists ]` button that toggles a vertical menu. `Esc` closes the menu and refocuses the button.
- [ ] `TaskRow.svelte` shows:
  - `[ ]`/`[x]` (complete/reopen; a recurring task shows the note `done · next <date>`)
  - `p1`–`p3` (P4 hidden)
  - the title, linking to `#/o/<id>`
  - the scheduled date, and `due <date>` in `--err` when overdue
  - `[open/total]` sub-tasks
  - `#tags`
  - At 360 px the meta wraps under the title.
- [ ] `TaskList.svelte` (props `name`, `filter`) renders a list. `upcoming` is grouped under date headers. Empty lists show `nothing in <name>`. It reloads on the window event `polykov:tasks-changed`, which 036 dispatches.
  - `j`/`k` move the selection, `Enter` opens, `x` toggles done. All three are ignored while a control owns keys.
- [ ] `ProjectView.svelte` shows:
  - the project title, status select, area select and dates
  - an unsectioned group, then each section with its open tasks
  - `[ add section ]` (prompt, Enter), and per section `[ rename ]`, `[ up ]`/`[ down ]` and `[ delete ]` → `delete <title>? [ yes ] [ no ]` (`[ no ]` focused)
  - The area view is `TaskList name=all filter={area}` plus that area's projects.
- [ ] Vitest tests with stubbed `fetch` cover the routes, each list's rendering, complete/reopen including the recurring note, keys, the reload event, and section add/rename/move/delete. `npm --prefix web run test` passes.
- [ ] `web/e2e/lists.spec.ts`, against the real server:
  - On `phone` (360 px), `expectPhoneFit` passes on Today with 3 tasks (one overdue, one with sub-tasks, one with long tags), on Upcoming, on a project with 2 sections, and with the nav menu open.
  - Keyboard flow: `tabTo('lists')`, `Enter`, arrow/Tab to `today`, `Enter`; then `j`, `x` marks the task done.
  - On `desktop`: the nav is visible without the toggle.
- [ ] `web/API.md` updated.

## Out of scope
- Task field editing, checklists and sub-task creation (035). Capture bar (036). Kanban/Eisenhower layouts and smart sort (M3). Drag to reorder (M8 / later).

## Notes for next
<!-- builder fills in -->
