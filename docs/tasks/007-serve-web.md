---
id: 007
title: Serve the built web UI from the server with SPA fallback
milestone: M0
status: done
deps: [002]
touches: [server/src/web/, server/src/app.rs, server/Cargo.toml, Cargo.lock, server/tests/, server/API.md]
spec: ["2. Architecture & Stack"]
---

## Goal
`polykov-server` serves static files from the configured `web_dir` at `/`, so the web UI and API share one origin. Files are read from disk at run time, not embedded (default — owner may change), so `cargo build` does not depend on a web build.

## Acceptance criteria
- [ ] `web_dir` comes from config (task 002); default is `web/dist` relative to the working directory
- [ ] `GET /` returns `index.html`; `GET /assets/<file>` returns the file with the right `Content-Type`
- [ ] Any other `GET` path not under `/api/` or `/health` that matches no file returns `index.html` (SPA fallback); unknown `/api/...` paths still return a JSON 404
- [ ] Path traversal (`/../config.toml`, encoded variants) cannot read outside `web_dir` (tested)
- [ ] If `web_dir` is missing, the server still starts, logs one warning, and `/` and every other non-API `GET` path return 503 with a short plain-text message; API routes and `/health` are unaffected
- [ ] The 001 test in `server/tests/health.rs` that expects 404 for an unknown path is changed to request an unknown `/api/...` path (JSON 404), since other paths now get the SPA fallback
- [ ] Integration tests in `server/tests/` use a temp dir with a fake `index.html` and asset, covering every case above
- [ ] `server/API.md` documents the behaviour

## Out of scope
- Building the web app (006), caching headers tuning, compression, TLS.

## Notes for next
- `app::router` now takes `(db, web_dir: &Path)`; tests that don't care pass a dir with no `index.html` (one warning on stderr, harmless).
- Static files use `tower-http` `ServeDir` + `ServeFile` fallback (its path validation rejects `..`, encoded separators and prefixes; rejected paths get `index.html`).
- Whether the UI exists is decided once, when the router is built: if `index.html` was missing at start, restart after building the web app. With it present, rebuilt files are served live.
- `/api`, `/api/*` and `/health/*` are reserved: unknown ones return `404 {"error":"not found"}` for any method. New API routes added via `.route` take precedence over the wildcard.
- Non-GET/HEAD requests to web paths return `405`. No cache headers yet (out of scope).
