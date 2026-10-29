# 0004 — Verify once, shared build dir, looser overlap

- **Status**: accepted
- **Context**: The factory machine has 2 cores and 3 GB RAM. Each PR ran the full verify four times, and every worktree compiled Rust from scratch (580–900 MB each). Shared hubs made server tasks run one at a time.
- **Decision**:
  - Per round, the builder runs `scripts/verify.sh` once and PR CI is the gate. The reviewer doesn't re-run it, and merges a rebase-only round when `git range-diff` shows no changes and CI is green. `clean-verify.sh` is kept for a red main.
  - `.cargo/config.toml` sets `build.build-dir` to `{cargo-cache-home}/polykov-build`. Every local worktree shares compiled deps, and final binaries stay in each checkout's `target/`. CI workflows set `CARGO_BUILD_BUILD_DIR` back to `target/`, so CI and rust-cache behave as before.
  - `board.sh --overlap` defines overlap. `Cargo.lock`, `API.md` files and `docs/decisions/` never count, and neither do two append-only (`+`) entries on the same file. Their conflicts are resolved by the "Rebase" recipe in LESSONS.md.
- **Consequences**: We use `build-dir` rather than `CARGO_TARGET_DIR`, so `target/debug/<bin>` paths stay valid, and parallel worktrees can't overwrite each other's binaries. Any new workflow that builds Rust must set the same env var, or it loses rust-cache. Concurrent local builds wait on cargo's lock. Don't add a file to the shared list unless its conflicts are mechanical.
