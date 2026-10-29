---
id: 001
title: Create Cargo workspace with server /health and CLI stub
milestone: M0
status: done
deps: []
touches: [Cargo.toml, Cargo.lock, rust-toolchain.toml, server/, cli/, docs/decisions/]
spec: ["2. Architecture & Stack", "10. Non-Functional"]
---

## Goal
A Cargo workspace exists with two crates: `server/` (package and binary `polykov-server`, Axum + Tokio) and `cli/` (package and binary `polykov`, clap). The server answers `GET /health`.

## Acceptance criteria
- [x] Root `Cargo.toml` is a workspace with members `server` and `cli`. `cargo build --workspace` produces `target/debug/polykov-server` and `target/debug/polykov`
- [x] `server` has both a lib target (`server/src/lib.rs`, crate `polykov_server`) and the binary; `server/tests/` and the CLI's dev-dependency (task 009) use the lib
- [x] The router is built by `polykov_server::app::router()` in `server/src/app.rs`, separate from `main`, so integration tests can call it without binding a port
- [x] The `/health` handler lives in `server/src/app.rs` (task 003 changes it there)
- [x] `polykov-server` listens on `127.0.0.1:7700` by default (default — owner may change); `--bind <addr>` overrides it
- [x] `curl -s localhost:7700/health` returns 200 with `{"status":"ok","version":"<crate version>"}`
- [x] Integration test `server/tests/health.rs` checks the 200 and the JSON body, and checks that an unknown path returns 404 (task 007 changes this to `/api/<unknown>` once the SPA fallback exists)
- [x] `polykov --version` prints the version; `polykov --help` lists no unimplemented subcommands
- [x] `server/CLAUDE.md`, `server/API.md`, `cli/CLAUDE.md`, `cli/API.md` exist (each ≤ 30 lines); `server/API.md` documents `/health`
- [x] ADR `docs/decisions/0001-workspace-layout.md` records the crate layout and the default port
- [x] `scripts/verify.sh` prints `VERIFY: PASS` (fmt, clippy, test steps run)

## Out of scope
- Config files, env vars, data dir (task 002), database (003), auth (004), serving the web UI (007).
- Any CLI subcommand that talks to the server (009).

## Notes for next
- Version lives once in root `[workspace.package]` (placeholder `0.0.0`; the owner picks the first real number). Crates use `version.workspace = true`.
- Tests drive `polykov_server::app::router()` via `tower::ServiceExt::oneshot` (dev-deps `tower` + `http-body-util`); see `server/tests/health.rs`.
- `router()` takes no args yet; task 002/003 will need to add state (config, db pool) to it, so update callers in `main.rs` and tests.
- `server/src/main.rs` parses args with clap (`--bind`); task 002 adds config there.
- CLI has an empty `Cli {}` struct; task 009 adds a `Subcommand` enum. `cli/tests/cli.rs` asserts `--help` has no `Commands:` section, so update that test then.
