---
id: 020
title: Add CLI attach command group for upload, list, download and delete
milestone: M1
status: done
deps: [016, 018]
touches: [cli/src/cmd/attach.rs, cli/src/client.rs, cli/src/cmd/mod.rs+, cli/src/main.rs+, cli/tests/attach.rs, cli/tests/cli.rs+, cli/API.md]
spec: ["7. Clients & UI", "3. Data Model"]
---

## Goal
Files can be attached to objects and fetched back from the CLI.

## Acceptance criteria
- [ ] `client::Client` gains `post_bytes(path, content_type, bytes)` and `get_bytes(path) -> (status, headers, bytes)`, with the same error mapping as the JSON calls.
- [ ] `polykov attach add <id> <file> [--name N]` uploads `?name=` (the file's basename by default) with a `Content-Type` guessed from a small extension table (default `application/octet-stream`). It prints `attached <aid>`, and `--json` gives the attachment.
- [ ] `polykov attach ls <id>` prints `<aid>  <name>  <size human>  <mime>`.
- [ ] `polykov attach get <aid> [-o PATH]` writes the bytes to `PATH`, or to the server-given name in the current dir. It refuses to overwrite an existing file unless `--force` (exit 1).
- [ ] `polykov attach rm <aid>`.
- [ ] An unreadable local file gets `polykov: cannot read <file>: <cause>` and exit 1, without a request.
- [ ] `cli/tests/attach.rs` covers an add/ls/get round-trip (bytes identical), refusing to overwrite, `rm`, a missing local file, and `--json`. The `--help` list test includes `attach`. Both pass.
- [ ] `cli/API.md` has a row for `attach`.

## Out of scope
- Voice upload and transcription (M11).

## Notes for next
- `get_bytes` returns `BytesReply { status, headers, bytes }`; `BytesReply::ok()` reuses `Reply::ok` for the error mapping. Bodies are read up to 26 MiB (ureq's default cap is 10 MiB).
- `attach get` without `-o` decodes `filename*=UTF-8''...` and refuses a name with `/`, `\`, `.` or `..`. It downloads first, then opens with `create_new`, so an existing file is never truncated.
- Output choices (spec silent): `wrote <path> (<size>)`, `removed <aid>`, `no attachments`; JSON for `get` is `{"id","path","size"}`, for `rm` `{"id","deleted":true}`.
- `object::ls::encode` became `pub` so `attach add` can percent-encode `?name=` (one-word change outside `touches`).
- The MIME table lives in `cmd/attach.rs::mime`; extend it there.
