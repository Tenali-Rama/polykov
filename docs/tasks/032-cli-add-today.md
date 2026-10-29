---
id: 032
title: Add CLI smart add, GTD list commands, done and reopen
milestone: M2
status: todo
deps: [026, 031]
touches: [cli/src/cmd/task/, cli/src/cmd/mod.rs+, cli/src/main.rs+, cli/Cargo.toml, Cargo.lock, cli/tests/task.rs, cli/tests/cli.rs+, cli/API.md, scripts/smoke.sh]
spec: ["7. Clients & UI", "Capture", "Tasks (GTD)"]
---

## Goal
The owner can capture with `polykov add "call mom tmrw 5pm p2"`, see `polykov today`, and tick tasks off from the terminal.

## Acceptance criteria
- [ ] New dir `cli/src/cmd/task/`, registered with lines added to `cmd/mod.rs` and `main.rs` only. It does not edit `client.rs`. The local date comes from `chrono::Local` and is sent as `today` on every call.
- [ ] `polykov add <text>...` sends `POST /api/tasks/add`, joining the words with spaces.
  - It prints `added <id>  <title>`, then a summary line of the parsed fields, e.g. `tomorrow 17:00 · 30m · p2 · #family · every sunday` (`inbox` when there are none).
  - A `400` prints the server text and exits 1.
- [ ] `polykov today|inbox|upcoming|someday|anytime [--project ID] [--area ID] [--limit N]` sends `GET /api/lists/<name>`.
  - One line per task: `<id>  p<n>  <title>  <date>  #tags`. Overdue dates get a leading `!`. A task with sub-tasks shows `[open/total]`.
  - `upcoming` groups rows under date headers.
  - An empty list prints `nothing in <name>`.
- [ ] `polykov done <id>...` completes each task and prints `done <id>`, plus `next <new id>  <date>` for a recurring task. `polykov reopen <id>` prints `reopened <id>`.
- [ ] `--json` prints the server JSON (the list response, the task, or `{"done":[{task,next}]}`).
- [ ] `cli/tests/task.rs` passes against the real router. It covers add with the spec example, the 400, each list, upcoming grouping, done with a recurring task, reopen and `--json`. The help listing test in `cli/tests/cli.rs` gains the new commands.
- [ ] `scripts/smoke.sh` adds `polykov add "smoke task tomorrow p1"` and checks `polykov upcoming --json` contains it.
- [ ] `cli/API.md` updated.

## Out of scope
- CLI editing of task fields, checklists and sections. Use `polykov edit` for the generic fields. Full CLI parity is M15.
- `polykov j` (M7), the TUI (M15).

## Notes for next
<!-- builder fills in -->
