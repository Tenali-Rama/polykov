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
| builder | `.claude/agents/builder.md` | one task → code + tests + PR (isolated worktree) |
| reviewer | `.claude/agents/reviewer.md` | independent verification → squash-merge or changes |

Quality gates:
- `scripts/verify.sh` runs the same checks locally and in CI
- builders and the reviewer verify in a clean checkout (`clean-verify.sh`)
- the reviewer re-checks every acceptance criterion
- the review rubric is `docs/factory/QUALITY.md`
- branch protection: PR + green CI is the only way into `main`
- a learned-lessons loop

### Owner setup (once)
1. `scripts/factory/bootstrap.sh`
2. Create a GitHub fine-grained token for this repo only, with these permissions:
   - Contents RW
   - Pull requests RW
   - Actions R
   - Commit statuses R
   - Administration RW (for branch protection)

   Then run `gh auth login --with-token` and paste the token.
3. `scripts/factory/protect-main.sh`
4. `scripts/factory/preflight.sh` should show all OK.
5. Start the COO: `claude --dangerously-skip-permissions`, then type: `You are the COO. Start building.`
