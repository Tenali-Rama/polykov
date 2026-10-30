---
id: 053
title: Reinstall stale web deps in verify and fix M2 docs nits
milestone: M3
status: todo
deps: []
touches: [scripts/verify.sh, web/CLAUDE.md, docs/factory/QUALITY.md, server/CLAUDE.md, docs/decisions/0001-workspace-layout.md]
spec: ["2. Architecture & Stack"]
---

## Goal
M2 review backlog. `scripts/verify.sh` reinstalls web deps when `web/node_modules` is stale against `web/package-lock.json`; this was hit twice. Four docs nits are also fixed.

## Acceptance criteria
- [ ] `verify.sh` runs the `web-install` step (`npm ci`) when `web/node_modules` is missing, or when `web/package-lock.json` differs from what was last installed. Detect this by comparing it with `web/node_modules/.package-lock.json`, or with a stored hash (builder's choice; say which in the PR). Otherwise it skips the step as today.
- [ ] Checked by hand, with the transcript in the PR:
  - with a fresh install, verify prints no `web-install` line
  - after adding a dependency to the lockfile (e.g. `git stash` of a lockfile change, or a checkout of an older lockfile), verify prints `PASS web-install`
- [ ] Docs:
  - `web/CLAUDE.md` says how to run only the phone e2e project (`scripts/e2e.sh --project=phone`)
  - `docs/factory/QUALITY.md` item 10 says touch targets are ≥ 44 px on every client at phone width (web included), not only Android
  - `server/CLAUDE.md` Layout lists `src/db/` (connection, `migrate.rs`, how to add a migration) and the per-feature module pattern (`src/<feature>/` with `routes()`)
  - ADR 0001 describes the router accurately as `polykov_server::app::router(db, web_dir)`, and lists the `parse/` workspace member. Keep it ≤ 15 lines.
- [ ] `scripts/verify.sh` passes.

## Out of scope
- Other verify steps. Rewording docs beyond these nits.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
