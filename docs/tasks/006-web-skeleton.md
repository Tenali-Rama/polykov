---
id: 006
title: Scaffold the Svelte + TS web app with the terminal-look shell
milestone: M0
status: todo
deps: []
touches: [web/]
spec: ["2. Architecture & Stack", "7. Clients & UI"]
---

## Goal
`web/` holds a Svelte + TypeScript single-page app (Vite, no SvelteKit server) (default — owner may change) with a minimal terminal-look shell that shows the server's `/health` status, plus the lint/check/test/build scripts `verify.sh` runs.

## Acceptance criteria
- [ ] `web/package.json` defines `lint`, `check`, `test`, `build` scripts; `web/package-lock.json` is committed; `npm --prefix web ci && scripts/verify.sh` runs `web-lint`, `web-check`, `web-test`, `web-build` and prints `VERIFY: PASS`
- [ ] `npm --prefix web run build` writes static files to `web/dist/` (git-ignored) with `index.html` at its root, using relative-safe asset paths served from `/`
- [ ] The shell uses a monospace font stack, a restrained palette, box-drawing style borders, and a block cursor on the prompt-style input placeholder; no icon library
- [ ] On load the shell calls `GET /health` and shows `server ok <version>` or `server unreachable`; a Vitest test covers both outcomes with a mocked `fetch`
- [ ] `npm --prefix web run dev` proxies `/health` and `/api` to `http://127.0.0.1:7700`
- [ ] Usable at 360 px width (no horizontal scroll) and every interactive element is reachable by Tab with a visible focus style
- [ ] `web/CLAUDE.md` and `web/API.md` exist (≤ 30 lines each); `API.md` lists the modules and the API client helper

## Out of scope
- Login and setup screens (008), serving from the Rust server (007), routing beyond a single shell view, themes (M16).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
