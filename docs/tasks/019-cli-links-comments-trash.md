---
id: 019
title: Add CLI link, comment and trash command groups
milestone: M1
status: done
deps: [015, 017, 018]
touches: [cli/src/cmd/link.rs, cli/src/cmd/comment.rs, cli/src/cmd/trash.rs, cli/src/cmd/mod.rs+, cli/src/main.rs+, cli/tests/link.rs, cli/tests/comment.rs, cli/tests/trash.rs, cli/tests/cli.rs+, cli/API.md]
spec: ["7. Clients & UI", "3. Data Model"]
---

## Goal
Links, backlinks, comments and the trash can be managed from the CLI, with `--json`.

## Acceptance criteria
- [ ] `polykov link add <id> <to>`, `polykov link rm <id> <to>`, and `polykov link ls <id>`. `ls` prints `links:` and `backlinks:` sections of `<id>  <type>  <title>`; `--json` gives the server body.
- [ ] `polykov comment add <id> <text>...`, `polykov comment ls <id>` (prints `#<cid> <created>` then the body), `polykov comment edit <cid> <text>...`, and `polykov comment rm <cid>`.
- [ ] Trash commands:
  - `polykov trash ls` shows `<id>  <type>  <title>  deleted <date>  purge <date>`
  - `polykov trash restore <id>` and `polykov trash purge <id>`
  - `polykov trash empty --yes` prints `purged <n>`; without `--yes` it exits 2 with a message and nothing is deleted
- [ ] Each group is one file in `cli/src/cmd/` and one `Command` variant with clap subcommands. It reuses the object rendering from 018 rather than copying it.
- [ ] `cli/tests/link.rs`, `comment.rs` and `trash.rs` cover every subcommand in human and `--json` mode, plus one 404 each. The `--help` list test includes `link`, `comment` and `trash`. All pass.
- [ ] `cli/API.md` has rows for all three groups.

## Out of scope
- Attachments (020).

## Notes for next
- Each group is a clap `Cmd` enum in `cli/src/cmd/<group>.rs` with one `#[command(subcommand)]` variant in `main.rs`.
- Reuse: `object::summary` (`<id>  <type>  <title>`, now also used by `line`) and `object::text` were made `pub` in `cli/src/cmd/object/mod.rs` (outside touches, visibility plus a 9-line extraction).
- `trash ls` prints dates as `YYYY-MM-DD` (date part of `deleted`/`purge_at`); `comment ls` prints the full RFC 3339 `created`.
- `trash empty` makes `--yes` a required clap flag, so without it clap exits 2 naming `--yes` before any request.
- Test helpers (`setup`/`ok`) are per test file, as in `tests/object.rs`; moving them to `tests/common` would be a separate chore.
