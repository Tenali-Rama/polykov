# 0004 — Verify once, looser overlap

- **Status**: accepted
- **Context**: The factory machine has 2 cores and 3 GB RAM. Each PR ran the full verify four times, and every worktree compiled Rust from scratch (580–900 MB each). Shared hubs made server tasks run one at a time.
- **Decision**:
  - Per round, the builder runs `scripts/verify.sh` once and PR CI is the gate. The reviewer doesn't re-run it, and merges a rebase-only round when `git range-diff` shows no changes and CI is green. `clean-verify.sh` is kept for a red main.
  - `board.sh --overlap` defines overlap. `Cargo.lock`, `API.md` files and `docs/decisions/` never count, and neither do two append-only (`+`) entries on the same file. Trailing `/` doesn't matter, and `--check` requires it on existing dirs. Their conflicts are resolved by the "Rebase" recipe in LESSONS.md.
- **Rejected: one cargo `build-dir` shared by all worktrees.** Cargo names workspace crates by their workspace-relative path and checks freshness by mtime. So every worktree writes the same artifact files, and one worktree can silently reuse another branch's build. That happened in review: a test binary had a deleted worktree's `CARGO_MANIFEST_DIR` baked in. A possible follow-up is to share only dependency builds (e.g. sccache) and keep `target/` per worktree.
- **Consequences**: Each worktree still has its own `target/`. Don't add a file to the shared list unless its conflicts are mechanical.
