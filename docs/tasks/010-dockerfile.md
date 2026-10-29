---
id: 010
title: Add Dockerfile and docker-compose for one-command start
milestone: M0
status: todo
deps: [003, 006, 007]
touches: [Dockerfile, .dockerignore, docker-compose.yml, docs/install.md]
spec: ["9. Security & Self-Hosting", "2. Architecture & Stack", "10. Non-Functional"]
---

## Goal
`docker compose up -d` builds and runs Polykov with the web UI and a persistent data volume, reachable at `http://localhost:7700`.

## Acceptance criteria
- [ ] Multi-stage `Dockerfile`: a Node stage builds `web/dist`, a Rust stage builds `polykov-server` in release mode, and a slim runtime stage runs it as a non-root user
- [ ] Runtime image sets `POLYKOV_DATA_DIR=/data`, `POLYKOV_BIND=0.0.0.0:7700`, `POLYKOV_WEB_DIR=/app/web`, declares `VOLUME /data`, `EXPOSE 7700`, and a `HEALTHCHECK` on `/health`
- [ ] `docker build -t polykov . && docker run -d -p 7700:7700 polykov` then `curl -s localhost:7700/health` returns `"status":"ok"` and `curl -s localhost:7700/` returns the web `index.html`
- [ ] `docker-compose.yml` runs the image with a named volume for `/data` and `restart: unless-stopped`; restarting the container keeps `polykov.db`
- [ ] Final image is under 150 MB (`docker image ls polykov`)
- [ ] `.dockerignore` excludes `target/`, `node_modules/`, `.factory/`, `.git/`
- [ ] `docs/install.md` (≤ 60 lines) covers Docker Compose and running the binary, and notes HTTPS via a reverse proxy and disk encryption are the owner's responsibility (spec §9)

## Out of scope
- CI image build (011), systemd unit and one-command install script (M17), built-in TLS.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
