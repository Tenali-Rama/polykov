---
id: 004
title: Add owner setup, password login, sessions and auth middleware
milestone: M0
status: todo
deps: [003]
touches: [server/src/auth/, server/migrations/, server/src/app.rs, server/Cargo.toml, Cargo.lock, server/tests/, server/API.md]
spec: ["9. Security & Self-Hosting", "1. Principles"]
---

## Goal
A single owner account can be created once on first run, log in with a password (argon2), and get a session that the web (cookie) and CLI (bearer token) both use. Non-public routes require it.

## Acceptance criteria
- [ ] `GET /api/setup` returns `{"needed":true}` until an owner exists, then `{"needed":false}`
- [ ] `POST /api/setup {"password":"..."}` creates the owner (argon2id hash) and returns 201; a second call returns 409; passwords shorter than 12 chars return 400 (default — owner may change)
- [ ] `POST /api/login` with the right password returns 200 with `{"token":"..."}` and sets cookie `polykov_session` (HttpOnly, SameSite=Strict, Path=/); wrong password returns 401 with no hint
- [ ] Session tokens are random (≥ 128 bits), stored only as a hash, and expire after 30 days (default — owner may change)
- [ ] `GET /api/me` returns 200 with either `Authorization: Bearer <token>` or the cookie, and 401 without
- [ ] `POST /api/logout` invalidates the session; a following `/api/me` with that token returns 401
- [ ] `/health` and `/api/setup`, `/api/login` stay public; integration tests in `server/tests/` cover every case above
- [ ] No `unwrap`/`expect` in non-test code; password and token never appear in logs (checked by reading the diff)
- [ ] `server/API.md` documents every endpoint and the auth extractor/middleware for later routes

## Out of scope
- API tokens (005), TOTP 2FA and login rate limiting (M17), web screens (008), CLI login (009).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
