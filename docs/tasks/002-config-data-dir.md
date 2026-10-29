---
id: 002
title: Load config.toml and env vars and create the data dir
milestone: M0
status: todo
deps: [001]
touches: [server/src/config/, server/src/main.rs, server/src/app.rs, server/Cargo.toml, Cargo.lock, server/API.md, server/CLAUDE.md]
spec: ["2. Architecture & Stack", "9. Security & Self-Hosting"]
---

## Goal
The server resolves a data dir, creates its layout on first start, and reads settings from `<data dir>/config.toml` overridden by `POLYKOV_*` env vars and CLI flags.

## Acceptance criteria
- [ ] Data dir resolves in order: `--data-dir` flag, `POLYKOV_DATA_DIR`, then `$XDG_DATA_HOME/polykov` (falling back to `~/.local/share/polykov`) (default — owner may change)
- [ ] On start, the data dir contains `attachments/`, `models/`, `backups/`; if `config.toml` is missing it is written with every option commented and set to its default
- [ ] Settings in M0: `bind` (default `127.0.0.1:7700`) and `web_dir` (used by 007). Precedence is flag > env (`POLYKOV_BIND`, `POLYKOV_WEB_DIR`) > `config.toml` > default
- [ ] Unit tests in `server/src/config/` cover: defaults, file value, env overriding file, flag overriding env, and an invalid TOML file failing with an error that names the file and line
- [ ] An unknown key in `config.toml` fails start-up with an error naming the key
- [ ] Running `polykov-server --data-dir /tmp/pk-test` twice works; the second run leaves an edited `config.toml` untouched
- [ ] `server/API.md` documents the config options and env vars

## Out of scope
- Creating `polykov.db` (003). Module toggles, ML, Telegram, TLS options (later milestones).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
