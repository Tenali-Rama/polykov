---
id: 008
title: Add web setup wizard, login and logout screens
milestone: M0
status: todo
deps: [004, 006]
touches: [web/src/]
spec: ["9. Security & Self-Hosting", "7. Clients & UI"]
---

## Goal
Opening the web UI on a fresh install shows a first-run setup screen to set the owner password; after that it shows a login screen, and once logged in, the shell with a logout action.

## Acceptance criteria
- [ ] On load the app calls `GET /api/setup` then `GET /api/me` and routes to setup, login, or the shell accordingly
- [ ] Setup screen: password + confirm fields; mismatch and too-short (< 12 chars) are shown inline before submitting; server 400/409 errors are shown as text
- [ ] Login screen: password field; 401 shows `wrong password`; success shows the shell without a page reload
- [ ] Auth uses the `polykov_session` cookie only; the token is never written to `localStorage`
- [ ] Logout calls `POST /api/logout` and returns to the login screen; it is reachable by keyboard
- [ ] Prompt-style inputs, terminal look matching 006; Enter submits; first field is focused on each screen; works at 360 px width
- [ ] Vitest tests with mocked `fetch` cover: routing for each of the three states, setup mismatch, login 401, logout
- [ ] `web/API.md` updated

## Out of scope
- TOTP, password change, API token management UI, any app views beyond the shell.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
