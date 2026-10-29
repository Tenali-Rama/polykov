---
id: 002
title: Load config.toml and env vars and create the data dir
milestone: M0
status: done
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
- `config::load(flags, env)` returns `Config { data_dir, bind, web_dir }`; `main.rs` calls it before binding. 003 should open `cfg.data_dir.join("polykov.db")`.
- `web_dir` defaults to `web/dist` (from 007's criteria) and is passed through unchecked; 007 decides what a missing dir means.
- Adding a setting: field in `config/file.rs` `FileConfig` + line in its `DEFAULT` template, merge in `config/mod.rs`, flag in `main.rs`, row in `server/API.md`.
- Env lookup is injected (`Fn(&str) -> Option<String>`), so tests never touch the process env; empty env vars count as unset.
- A relative `XDG_DATA_HOME` is ignored (per the XDG spec) and falls through to `$HOME/.local/share/polykov`.
