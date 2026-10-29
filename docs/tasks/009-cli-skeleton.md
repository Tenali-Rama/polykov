---
id: 009
title: Build the CLI skeleton with login, logout and status
milestone: M0
status: todo
deps: [004]
touches: [cli/, Cargo.lock]
spec: ["7. Clients & UI", "2. Architecture & Stack", "9. Security & Self-Hosting"]
---

## Goal
`polykov` has a small HTTP client module and three subcommands that talk to the server, with a global `--json` flag, so later tasks only add subcommands.

## Acceptance criteria
- [ ] Server URL resolves from `--server`, then `POLYKOV_SERVER`, then the CLI config file, then `http://127.0.0.1:7700`
- [ ] `polykov login` prompts for the password without echo (or reads `POLYKOV_PASSWORD`), calls `POST /api/login`, and stores server URL + token in `$XDG_CONFIG_HOME/polykov/cli.toml` (fallback `~/.config/polykov/cli.toml`) with file mode `0600` (default — owner may change)
- [ ] `polykov status` prints server version, db state and whether the stored token is valid (`/health` + `/api/me`); `--json` prints one JSON object instead
- [ ] `polykov logout` calls `POST /api/logout` and removes the stored token
- [ ] Errors are one human line on stderr with exit code 1 (unreachable server, 401 "run polykov login"); `--json` errors print `{"error":"..."}`
- [ ] Integration tests in `cli/tests/` start the real server router (dev-dependency on `polykov-server`) on a random port with a temp data dir and cover login, status (valid and invalid token), logout, and unreachable server
- [ ] `cli/API.md` documents the client module and how to add a subcommand

## Out of scope
- Any object/task commands, the TUI (M15), API token commands.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
