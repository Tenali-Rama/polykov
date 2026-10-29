---
id: 005
title: Add long-lived API tokens for scripts and clients
milestone: M0
status: done
deps: [012]
touches: [server/src/auth/, server/migrations/, server/src/app.rs, server/tests/, server/API.md]
spec: ["9. Security & Self-Hosting"]
---

## Goal
The owner can create, list and revoke named API tokens that authenticate exactly like a session bearer token, through the extractor from 012.

## Acceptance criteria
- [ ] `POST /api/tokens {"name":"..."}` (auth required) returns 201 with `{"id":...,"name":"...","token":"..."}`; the plain token is shown only in this response and stored only as a hash
- [ ] `GET /api/tokens` lists `id`, `name`, `created`, `last_used` and never the token or its hash
- [ ] `DELETE /api/tokens/{id}` returns 204; the revoked token then gets 401 on `/api/me`; unknown id returns 404
- [ ] API tokens do not expire (default — owner may change); `last_used` updates on use
- [ ] Empty or > 64-char names return 400
- [ ] Integration tests in `server/tests/` cover create, use, list, revoke, unauthenticated access (401) to all three endpoints, and bad names
- [ ] `server/API.md` documents the endpoints

## Out of scope
- Token scopes/permissions, a web UI for tokens (later milestone), CLI token commands (009 uses session login).

## Notes for next
- `auth::Session` now also accepts API tokens; `Session.expires_at` is `Option<String>` (`None` for an API token), and `/api/me` returns `"session_expires":null` for one.
- Tokens use the session token format (64 hex) and `session::hash_token`; lookup tries `sessions` then `api_tokens` (ADR 0005).
- Names are trimmed, must be 1-64 chars, and need not be unique; the id identifies a token.
- `POST /api/logout` with an API token is a no-op; revoke only via `DELETE /api/tokens/{id}`.
- Migration 0003 adds `api_tokens`; this needed an append-only line in `src/db/migrate.rs` (not in `touches`).
