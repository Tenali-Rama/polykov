---
id: 012
title: Add auth extractor, /api/me and logout
milestone: M0
status: done
deps: [004]
touches: [server/src/auth/, server/src/app.rs, server/tests/, server/API.md]
spec: ["9. Security & Self-Hosting"]
---

## Goal
Every non-public route requires a valid session, given either as `Authorization: Bearer <token>` or as the `polykov_session` cookie, through one reusable extractor that later routes add with one line.

## Acceptance criteria
- [x] An Axum extractor (or middleware) resolves the session from the bearer header or the cookie; missing, unknown or expired sessions get 401 with a JSON error body
- [x] `GET /api/me` returns 200 with `{"owner":true,"session_expires":"<RFC 3339>"}` when authenticated, 401 otherwise
- [x] `POST /api/logout` (auth required) deletes the session and clears the cookie; a following `/api/me` with that token returns 401
- [x] An expired session (expiry set in the past in the test) gets 401
- [x] `/health`, `GET/POST /api/setup` and `POST /api/login` stay public (tested)
- [x] Integration tests in `server/tests/` cover bearer, cookie, missing, wrong, expired, logout
- [x] `server/API.md` documents the extractor, how a new route requires auth, and both endpoints

## Out of scope
- API tokens (005), CSRF tokens beyond SameSite=Strict, rate limiting (M17).

## Notes for next
- Require auth on a route by adding a `session: auth::Session` (or `_: auth::Session`) handler argument; it works with any state where `Db: FromRef<S>`.
- Bearer header wins over the cookie; a present but bad bearer header is a 401 even with a valid cookie (no silent fallback).
- 401 body is `{"error":"authentication required"}`; logout returns 204 and clears the cookie with `Max-Age=0`.
- `session_expires` is formatted by SQLite (`strftime ... 'unixepoch'`), so no date crate was added.
- Shared integration-test helpers now live in `server/tests/common/mod.rs` (`DataDir`, `send`, `get`, `post`, `setup`, `login`).
