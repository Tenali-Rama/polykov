---
id: 011
title: Wire CI fully with an end-to-end smoke test and Docker build
milestone: M0
status: todo
deps: [005, 008, 009, 010]
touches: [scripts/verify.sh, scripts/smoke.sh, .github/workflows/]
spec: ["10. Non-Functional", "2. Architecture & Stack"]
---

## Goal
`scripts/verify.sh` (and so CI) runs a smoke test of the real binaries together, and CI also proves the Docker image builds and is healthy. Release builds include the web UI.

## Acceptance criteria
- [ ] `scripts/smoke.sh` starts `target/debug/polykov-server` with a temp data dir on a free port and `web_dir=web/dist`, then checks: `/health` ok, `/` returns `index.html`, setup via `POST /api/setup`, `polykov login` (via `POLYKOV_PASSWORD`), `polykov status --json` reports a valid token, an API token can be created and used on `/api/me`; it always stops the server and removes the temp dir, and exits non-zero on the first failed check with a one-line reason
- [ ] `scripts/verify.sh` runs `smoke` as a step after the cargo and web steps, only when both `Cargo.toml` and `web/package.json` exist; `scripts/verify.sh` prints `VERIFY: PASS`
- [ ] `.github/workflows/ci.yml` gains a `docker` job that builds the image, runs it, and waits for `/health` to return ok (fails after 30 s)
- [ ] `.github/workflows/release.yml` builds `web/dist` before packaging and ships it in a `polykov-web-<tag>.tar.gz` asset
- [ ] Breaking the smoke test on purpose (e.g. wrong expected status) makes `verify.sh` print `FAIL smoke` with the reason (checked locally, not committed)
- [ ] `scripts/factory/pr-ci.sh <pr>` reports `success` for this task's PR

## Out of scope
- UI end-to-end browser tests (added with the first core UI flow in M2), coverage reporting, Android/desktop release jobs.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
