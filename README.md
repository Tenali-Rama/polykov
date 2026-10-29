# Polykov

A self-hosted, single-user productivity tool: tasks, notes, calendar, habits, goals, journaling, and local-AI analytics, all in one place. It has a CLI look and runs on web, CLI, Linux, and Android. Your data stays on your server.

- What it does: [`spec.md`](spec.md)
- License: [AGPL-3.0](LICENSE)

> Early development. Not usable yet.

## How this repo is built

An AI "software factory" running on Claude Code writes this repo. See [`docs/factory/`](docs/factory/).

| Role | Defined in | Job |
|---|---|---|
| COO | `docs/factory/COO.md` | the owner's single contact; runs the loop |
| planner | `.claude/agents/planner.md` | spec → milestone → small task files |
| builder | `.claude/agents/builder.md` | one task → code + tests on a `task/*` branch (isolated worktree) |
| reviewer | `.claude/agents/reviewer.md` | independent verification → `merge.sh` to main, or changes |

The factory needs only an SSH deploy key: no PRs and no GitHub token. CI runs on every pushed branch and publishes its result as a git ref (`refs/ci/pass|fail/<sha>`), which the factory reads with `scripts/factory/ci-status.sh`.

Quality gates:
- `scripts/verify.sh` runs the same checks locally and in CI
- the reviewer verifies in a clean checkout and re-checks every acceptance criterion
- the review rubric is `docs/factory/QUALITY.md`, and every review is recorded in `docs/reviews/`
- `merge.sh` is the only path to `main`: it requires an approved review and green CI, and re-verifies after the squash
- a learned-lessons loop

### Owner setup (once)
1. `scripts/factory/bootstrap.sh`
2. `scripts/factory/preflight.sh` should show all OK.
3. Start the COO: `claude --dangerously-skip-permissions`, then type: `You are the COO. Start building.`
