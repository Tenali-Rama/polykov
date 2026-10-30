# Lessons

These rules come from repeated review findings, and the COO maintains them through retro PRs. Every builder reads this file before starting.

- Keep this file to at most 30 bullets. Merge or drop stale ones.

<!-- rules go below -->
- `touches` must name every file the task edits, hubs included (`lib.rs`, `db/mod.rs`, `db/migrate.rs`, `app.rs`, ADRs). Several M0 tasks had to edit hubs that weren't listed, which made parallel dispatch unsafe.
- Put a new feature in its own module dir and its own test file. Shared hubs get only append-only registration lines (`touches` entry with a trailing `+`).
- Verify runs once per round locally (`scripts/verify.sh`), then PR CI is the gate. Don't run `clean-verify.sh` for a PR. It's for a red main.
- Planners pre-assign ADR numbers: a task that needs an ADR names its number in the task file, and parallel tasks never share one. In M2, four PRs picked the same next free number and needed rebase rounds.
- Planners and reviewers leave the main checkout on a clean `main` when they finish. Do review and branch work in a worktree. In M2 the shared checkout was left on other branches, which blocked the next dispatch.
- When several tasks fill a placeholder slot in a shared component, give each filler its own line, so parallel PRs only add lines. In M1, four PRs needed rebase rounds because they all edited the same hub lines.

## UI tasks
- Before you open the PR, run the built app in headless Chromium at 360 px. Check that there's no horizontal scroll, that touch targets are ≥ 44 px, and that the keyboard flow works, and put the measured numbers in the PR. jsdom tests can't catch layout: in M1 a column collapsed to one character per line at 360 px, and targets under 44 px got through.
- An editor that handles Esc must mark the event handled (`preventDefault` + `stopPropagation`), so the page doesn't navigate away while input is unsaved.

## Rebase
Shared files (`Cargo.lock`, `API.md`, `docs/decisions/`) and append-only (`+`) files may conflict when parallel PRs land. Resolve them mechanically, then let CI check the result:
- `Cargo.lock`: `git checkout --ours Cargo.lock` (during a rebase, "ours" is main), then `cargo metadata --format-version 1 >/dev/null` to re-add your deps, then `git add Cargo.lock`.
- `API.md` and append-only files: keep both sides' lines.
- ADR number taken on main: rename yours to the next free number, and fix references to it.
- Any other conflict is a real one. Resolve it carefully, and say so in the PR comment so the reviewer re-reviews it.
