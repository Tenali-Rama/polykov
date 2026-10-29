---
id: 004
title: Add owner setup, password login and session issuing
milestone: M0
status: todo
deps: [003]
touches: [server/src/auth/, server/migrations/, server/src/app.rs, server/Cargo.toml, Cargo.lock, server/tests/, server/API.md]
spec: ["9. Security & Self-Hosting", "1. Principles"]
---

## Goal
A single owner account can be created once on first run and log in with a password (argon2), receiving a session token (bearer, for the CLI) and a session cookie (for the web). Checking sessions on requests is task 012.

## Acceptance criteria
- [ ] One migration adds the owner and sessions tables
- [ ] `GET /api/setup` returns `{"needed":true}` until an owner exists, then `{"needed":false}`
- [ ] `POST /api/setup {"password":"..."}` creates the owner (argon2id hash) and returns 201; a second call returns 409; passwords shorter than 12 chars return 400 (default — owner may change)
- [ ] `POST /api/login` with the right password returns 200 with `{"token":"..."}` and sets cookie `polykov_session` (HttpOnly, SameSite=Strict, Path=/); wrong password, or no owner yet, returns 401 with no hint
- [ ] Session tokens are random (≥ 128 bits), stored only as a hash with an expiry 30 days out (default — owner may change); a test reads the sessions table and finds no plain token
- [ ] Integration tests in `server/tests/` cover every case above
- [ ] No `unwrap`/`expect` in non-test code; password and token never appear in logs (checked by reading the diff)
- [ ] `server/API.md` documents the endpoints and the session store functions 012 will use

## Out of scope
- Auth extractor, `/api/me`, logout (012); API tokens (005); TOTP 2FA and login rate limiting (M17); web screens (008); CLI login (009).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
