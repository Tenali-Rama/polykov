---
id: 006
title: Scaffold the Svelte + TS web app with the terminal-look shell
milestone: M0
status: done
deps: []
touches: [web/]
spec: ["2. Architecture & Stack", "7. Clients & UI"]
---

## Goal
`web/` holds a Svelte + TypeScript single-page app (Vite, no SvelteKit server) (default — owner may change) with a minimal terminal-look shell that shows the server's `/health` status, plus the lint/check/test/build scripts `verify.sh` runs.

## Acceptance criteria
- [x] `web/package.json` defines `lint`, `check`, `test`, `build` scripts; `web/package-lock.json` is committed; `npm --prefix web ci && scripts/verify.sh` runs `web-lint`, `web-check`, `web-test`, `web-build` and prints `VERIFY: PASS`
- [x] `npm --prefix web run build` writes static files to `web/dist/` (git-ignored) with `index.html` at its root, using relative-safe asset paths served from `/`
- [x] The shell uses a monospace font stack, a restrained palette, box-drawing style borders, and a block cursor on the prompt-style input placeholder; no icon library
- [x] On load the shell calls `GET /health` and shows `server ok <version>` or `server unreachable`; a Vitest test covers both outcomes with a mocked `fetch`
- [x] `npm --prefix web run dev` proxies `/health` and `/api` to `http://127.0.0.1:7700`
- [x] Usable at 360 px width (no horizontal scroll) and every interactive element is reachable by Tab with a visible focus style
- [x] `web/CLAUDE.md` and `web/API.md` exist (≤ 30 lines each); `API.md` lists the modules and the API client helper

## Out of scope
- Login and setup screens (008), serving from the Rust server (007), routing beyond a single shell view, themes (M16).

## Notes for next
- Stack: Svelte 5 (runes) + Vite 7 + TS 5.9, Vitest 3 + jsdom + Testing Library; `svelteTesting()` plugin in `vite.config.ts` handles browser resolve + auto-cleanup.
- All server calls go through `web/src/lib/api.ts` (`getJson`, `ApiError`); 008 adds POST helpers there.
- Build output `web/dist/` uses absolute `/assets/...` URLs, so 007 must serve it at `/` (SPA fallback is safe for deep paths).
- `/health` returning 503 (task 003 db error) currently shows `server unreachable`; split the message if a distinct "db error" state is wanted.
- On this FS `npm install` can fail renaming `.bin` links (EOVERFLOW); a fresh `npm ci` in a new checkout works.
