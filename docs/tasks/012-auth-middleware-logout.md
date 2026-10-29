---
id: 012
title: Add auth extractor, /api/me and logout
milestone: M0
status: todo
deps: [004]
touches: [server/src/auth/, server/src/app.rs, server/tests/, server/API.md]
spec: ["9. Security & Self-Hosting"]
---

## Goal
Every non-public route requires a valid session, given either as `Authorization: Bearer <token>` or as the `polykov_session` cookie, through one reusable extractor that later routes add with one line.

## Acceptance criteria
- [ ] An Axum extractor (or middleware) resolves the session from the bearer header or the cookie; missing, unknown or expired sessions get 401 with a JSON error body
- [ ] `GET /api/me` returns 200 with `{"owner":true,"session_expires":"<RFC 3339>"}` when authenticated, 401 otherwise
- [ ] `POST /api/logout` (auth required) deletes the session and clears the cookie; a following `/api/me` with that token returns 401
- [ ] An expired session (expiry set in the past in the test) gets 401
- [ ] `/health`, `GET/POST /api/setup` and `POST /api/login` stay public (tested)
- [ ] Integration tests in `server/tests/` cover bearer, cookie, missing, wrong, expired, logout
- [ ] `server/API.md` documents the extractor, how a new route requires auth, and both endpoints

## Out of scope
- API tokens (005), CSRF tokens beyond SameSite=Strict, rate limiting (M17).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
