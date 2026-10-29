# Polykov

Self-hosted, single-user productivity app. `spec.md` defines *what* to build. This file and `docs/factory/` define *how*.

## Roles
- Told you are the **COO** → read `docs/factory/COO.md` and follow it. Do that before anything else.
- Spawned as **planner / builder / reviewer** → your agent definition is your instructions.
- Any other direct work → follow the builder rules in `.claude/agents/builder.md`.

## Stack
- Rust workspace:
  - server: Axum + SQLite
  - CLI: clap + ratatui
- Web UI: Svelte + TS in `web/`.
- Linux and Android apps: Tauri 2.
- Every top-level code dir gets its own short `CLAUDE.md` once it exists.

## Commands
- `scripts/verify.sh`: the one check that counts; CI runs the same script. Prints one line per step, with full logs in `.factory/logs/`.
- `scripts/factory/board.sh [--ready|--check]`: task status table.
- `scripts/factory/spec.sh "<heading>"`: print a single spec section.

## Context rules (everyone)
- Load only what the task needs, in this order:
  1. the task file
  2. its spec sections (via `spec.sh`)
  3. `API.md` of the modules it touches
  4. the specific code
- Never read all of `spec.md`, whole directories, or full logs. Search first, then read line ranges.
- Logs: read the tail of the log file. Never paste a full log.
- Broad exploration: use an Explore subagent and keep only its conclusion.
- At about half of your context: stop, write handoff notes into the task file, and report.

## Code rules
- Keep modules small and files ≤ 400 lines. Each module's public interface is summarized in its `API.md`.
- Match the surrounding style. No dead code, no unrelated changes.
- Every behaviour change has a test.
- Lasting decisions go in `docs/decisions/NNNN-title.md`, ≤ 15 lines.
- Read `docs/factory/LESSONS.md` before building.

## Git
- Never commit or push to main; `main` is protected. Work on `task/NNN-slug` (or `plan/*`, `chore/*`, `fix/*`), open a PR, and the reviewer merges it once CI is green.
- Git goes over SSH with the deploy key (remote `origin`). `gh` targets the repo through `GH_REPO`, set in `.claude/settings.json`.
- `scripts/factory/clean-verify.sh <ref>` runs verify in a fresh checkout: the local stand-in for CI.
- Author is always `Tenali-Rama <tenalirama.krishna125@gmail.com>`, set in repo config.
- Never put AI attribution or co-author lines in commits or PRs. A git hook, a Claude hook, and CI all enforce this.
