---
id: 019
title: Add CLI link, comment and trash command groups
milestone: M1
status: todo
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
<!-- builder fills in -->
