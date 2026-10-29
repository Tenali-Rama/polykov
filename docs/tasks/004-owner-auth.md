---
id: 004
title: Add owner setup, password login and session issuing
milestone: M0
status: done
deps: [003]
touches: [server/src/auth/, server/migrations/, server/src/app.rs, server/Cargo.toml, Cargo.lock, server/tests/, server/API.md]
spec: ["9. Security & Self-Hosting", "1. Principles"]
---

## Goal
A single owner account can be created once on first run and log in with a password (argon2), receiving a session token (bearer, for the CLI) and a session cookie (for the web). Checking sessions on requests is task 012.

## Acceptance criteria
- [x] One migration adds the owner and sessions tables
- [x] `GET /api/setup` returns `{"needed":true}` until an owner exists, then `{"needed":false}`
- [x] `POST /api/setup {"password":"..."}` creates the owner (argon2id hash) and returns 201; a second call returns 409; passwords shorter than 12 chars return 400 (default — owner may change)
- [x] `POST /api/login` with the right password returns 200 with `{"token":"..."}` and sets cookie `polykov_session` (HttpOnly, SameSite=Strict, Path=/); wrong password, or no owner yet, returns 401 with no hint
- [x] Session tokens are random (≥ 128 bits), stored only as a hash with an expiry 30 days out (default — owner may change); a test reads the sessions table and finds no plain token
- [x] Integration tests in `server/tests/` cover every case above
- [x] No `unwrap`/`expect` in non-test code; password and token never appear in logs (checked by reading the diff)
- [x] `server/API.md` documents the endpoints and the session store functions 012 will use

## Out of scope
- Auth extractor, `/api/me`, logout (012); API tokens (005); TOTP 2FA and login rate limiting (M17); web screens (008); CLI login (009).

## Notes for next
- 012: check requests with `auth::session::is_valid(&db, token)` (bearer header or `auth::COOKIE_NAME` cookie); run it in `spawn_blocking` like the handlers in `auth/mod.rs`. Logout can delete by `session::hash_token(token)`.
- Tokens are 64 hex chars (256 bits from `getrandom`); `sessions` holds only their SHA-256 hex, with `created_at`/`expires_at` in Unix seconds (see ADR 0003).
- The cookie has no `Secure` flag so plain-HTTP LAN installs still work; add it when built-in TLS lands, or decide otherwise in an ADR.
- Login with no owner returns 401 without hashing, so it is faster than a wrong-password 401; acceptable since `GET /api/setup` is public anyway. Rate limiting is M17.
- argon2 is slow in debug builds (~1 s per hash); auth tests take ~9 s. `db::Db::conn()` (`pub(crate)`) is how crate modules get the connection.
