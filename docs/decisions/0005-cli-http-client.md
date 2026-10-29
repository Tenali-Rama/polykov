# 0005 — CLI HTTP client and token storage

- **Status**: accepted
- **Context**: Task 009 gives the CLI its server client and a place to keep the session token (spec §2, §9).
- **Decision**: The CLI is synchronous and uses `ureq` 3 (blocking, rustls) with a 10 s timeout. Every HTTP status comes back as a `Reply`; `Reply::ok()` maps `401` to the shared "run polykov login" error. The token and its server live in `cli.toml` (new files `0600`), and the token is sent only to the server that issued it.
- **Consequences**: No Tokio runtime in the CLI binary; the TUI (M15) can stay sync too or add a runtime of its own. Never send the stored token to a server URL that differs from the stored one.
