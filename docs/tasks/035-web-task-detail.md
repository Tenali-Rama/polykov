---
id: 035
title: Add the web task panel with fields, recurrence, checklist and sub-tasks
milestone: M2
status: todo
deps: [034, 037]
touches: [web/src/tasks/TaskFields.svelte, web/src/tasks/TaskFields.test.ts, web/src/tasks/Recurrence.svelte, web/src/tasks/Checklist.svelte, web/src/tasks/Checklist.test.ts, web/src/tasks/Subtasks.svelte, web/src/tasks/Subtasks.test.ts, web/src/lib/tasks.ts, web/src/lib/tasks.test.ts, web/src/objects/Detail.svelte+, web/e2e/task-detail.spec.ts, web/API.md]
spec: ["Tasks (GTD)", "7. Clients & UI", "3. Data Model"]
---

## Goal
Opening a task shows every task field editable in place, with its checklist and sub-tasks. It keeps M1's links, comments and image attachments panels.

## Acceptance criteria
- [ ] `Detail.svelte` gains one line in its `panels` section: `{#if obj.type === 'task'}<TaskFields .../><Checklist .../><Subtasks .../>{/if}`, plus the imports. Nothing else in `Detail` changes.
- [ ] `TaskFields.svelte` (props `id`) shows:
  - `[ complete ]`/`[ reopen ]`
  - priority select P1–P4
  - scheduled and deadline as `<input type=date>`, plus an optional time and `[ clear ]`
  - estimate, which accepts `90`, `90m`, `1h30m` (inline error otherwise)
  - a someday toggle
  - project select → section select of that project
  - area select
  - parent: id-or-title prompt, like `Links` (via `findByTitle`), with `[ clear ]`
  - Every change is one `PATCH /api/tasks/{id}`. The server's `400` text shows inline.
- [ ] `Recurrence.svelte` shows a frequency select (none/day/week/month/year), an interval number, weekday toggles for week, a month day for month, and `from` (scheduled/completion). It builds the 027 rule. The existing rule shows as text (e.g. `every 2 weeks on mon, thu · from completion`).
- [ ] `Checklist.svelte` shows items `[ ]`/`[x] text`.
  - Click or Space toggles an item. `[ edit ]` edits inline (Enter saves, Esc cancels, with `preventDefault`). `[ delete ]` removes it. Alt+↑/↓ or `[ up ]`/`[ down ]` reorders it. The `add item` prompt adds with Enter.
  - Each change sends the whole checklist in one `PATCH`, and focus stays on the item acted on.
- [ ] `Subtasks.svelte` lists children with `TaskRow` (034) and has an `add sub-task` prompt (Enter → `POST /api/tasks` with `parent_id`).
- [ ] Vitest tests with stubbed `fetch` cover each field's PATCH body, estimate parsing, the recurrence rule builder, checklist toggle/edit/reorder/delete, sub-task add, and non-task objects showing no task panels. `npm --prefix web run test` passes.
- [ ] `web/e2e/task-detail.spec.ts`, against the real server:
  - On `phone` (360 px), `expectPhoneFit` passes on a task with every field set, 5 checklist items (one 300-char), 2 sub-tasks, a comment and an uploaded PNG attachment with its preview.
  - Keyboard flow: Tab through every field in visual order; add a checklist item with Enter, toggle it with Space, and move it with Alt+↓, without using the mouse.
  - Completing a recurring task from the panel shows the next occurrence's date.
- [ ] `web/API.md` updated.

## Out of scope
- Time tracking and actual time (M6). Reminders (M9). Editing task fields via smart-add text.

## Notes for next
<!-- builder fills in -->
