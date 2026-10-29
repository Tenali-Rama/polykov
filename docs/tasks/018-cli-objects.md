---
id: 018
title: Add CLI object commands new, show, ls, edit, rm, pin and archive
milestone: M1
status: done
deps: [014]
touches: [cli/src/cmd/object/, cli/src/cmd/mod.rs+, cli/src/main.rs+, cli/src/client.rs, cli/tests/object.rs, cli/tests/cli.rs+, cli/API.md, scripts/smoke.sh]
spec: ["7. Clients & UI", "3. Data Model"]
---

## Goal
The owner can do full object CRUD from the terminal, with `--json` on every command.

## Acceptance criteria
- [ ] `client::Client` gains `patch`, `put` and `delete` with the same `Reply` contract as `get`/`post`.
- [ ] Commands live in a new `cli/src/cmd/object/` module (one file per command, plus shared rendering), registered with one line in `cmd/mod.rs` and `main.rs` variants/arms:
  - `polykov new <type> <title>... [--tag T]... [--prop k=v]...` prints `created <id>`; `--json` gives the object
  - `polykov show <id>` prints title, type, id, tags, pinned/archived, properties and dates; `--json` gives the object
  - `polykov ls [--type T] [--tag T] [--pinned] [--archived|--all] [--limit N]` prints one line per object (`<id>  <type>  <title>  #tag ...`, with `*` marking pinned objects); `--json` gives `{"objects":[...],"total":n}`
  - `polykov edit <id> [--title T] [--add-tag T]... [--rm-tag T]... [--prop k=v]... [--unset K]...` does a GET, merges, then sends one PATCH
  - `polykov rm <id>` moves the object to the trash
  - `polykov pin|unpin|archive|unarchive <id>`
- [ ] `--prop k=v` parses `v` as an integer or float, then as `true`/`false`, else as a string (default — owner may change). A missing `=` gets exit 2.
- [ ] A server `400`/`404` prints `polykov: server returned 404: object not found` and exits 1.
- [ ] `cli/tests/object.rs` drives every command against `tests/common::Server`, in human and `--json` mode, including a 404. The `--help` list test in `cli/tests/cli.rs` includes the new commands. Both pass.
- [ ] `scripts/smoke.sh` additionally runs `polykov new note smoke --tag smoke` and checks that `polykov ls --tag smoke --json` has `total` 1.
- [ ] `cli/API.md` has a row per command.

## Out of scope
- `polykov add` (smart add, M2) and `polykov tui` (M15).
- Links, comments, trash (019) and attachments (020).

## Notes for next
- `cmd/object/` holds every object command; `flag.rs` covers pin/unpin/archive/unarchive. Commands with flags embed a `clap::Args` struct in `main`'s `Command` variant.
- `ls` human format: pinned ids carry a `*` prefix (`*3  note  Title  #tag`); a paged list ends with `(n of total shown)`, an empty one prints `no objects`.
- `show` lists properties as `.key` rows after the fixed fields; strings print unquoted, other values as JSON.
- `Client::put` has no caller yet (`#[allow(dead_code)]` until `polykov link`, task 019, uses it); remove the allow there.
- `--prop` floats that are not finite (`inf`, `nan`) stay strings, since JSON can't hold them.
