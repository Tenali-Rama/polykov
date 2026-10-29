# Lessons

These rules come from repeated review findings, and the COO maintains them through retro PRs. Every builder reads this file before starting.

- Keep this file to at most 30 bullets. Merge or drop stale ones.

<!-- rules go below -->
- `touches` must name every file the task edits, hubs included (`lib.rs`, `db/mod.rs`, `db/migrate.rs`, `app.rs`, ADRs). Several M0 tasks had to edit hubs that weren't listed, which made parallel dispatch unsafe.
- Put a new feature in its own module dir and its own test file. Shared hubs get only append-only registration lines (`touches` entry with a trailing `+`).
- Verify runs once per round locally (`scripts/verify.sh`), then PR CI is the gate. Don't run `clean-verify.sh` for a PR. It's for a red main.
- Local cargo builds share one build dir, `~/.cargo/polykov-build` (`.cargo/config.toml`, ADR 0004). Binaries are still in `./target/debug/`. Concurrent builds wait on cargo's lock, which is expected. Delete that dir to reclaim disk, but only while no build is running.

## Rebase
Shared files (`Cargo.lock`, `API.md`, `docs/decisions/`) and append-only (`+`) files may conflict when parallel PRs land. Resolve them mechanically, then let CI check the result:
- `Cargo.lock`: `git checkout --ours Cargo.lock` (during a rebase, "ours" is main), then `cargo metadata --format-version 1 >/dev/null` to re-add your deps, then `git add Cargo.lock`.
- `API.md` and append-only files: keep both sides' lines.
- ADR number taken on main: rename yours to the next free number, and fix references to it.
- Any other conflict is a real one. Resolve it carefully, and say so in the PR comment so the reviewer re-reviews it.
